//! # Leaky Bucket Rate-Limiting (as a meter) in Rust
//! This crate implements
//! the
//! [generic cell rate algorithm](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm) (GCRA)
//! for rate-limiting and scheduling in Rust.
//!
//! ## Interface
//!
//! This crate implements two "serious" rate-limiting/traffic-shaping
//! algorithms:
//! [GCRA](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm)
//! and a [Leaky
//! Bucket](https://en.wikipedia.org/wiki/Leaky_bucket#As_a_meter). An
//! "unserious" implementation is provided also, the
//! [`Allower`](example_algorithms/struct.Allower.html), which returns
//! "Yes" to all rate-limiting queries.
//!
//! The Generic Cell Rate Algorithm can be used by creating a builder
//! from the [`GCRA`](algorithms/gcra/struct.GCRA.html) struct:
//!
//! ``` rust
//! use std::time::Duration;
//! use ratelimit_meter::{Decider, GCRA};
//!
//! let mut lim = GCRA::for_capacity(50).unwrap() // Allow 50 units of work
//!     .per(Duration::from_secs(1)) // We calculate per-second (this is the default).
//!     .cell_weight(1).unwrap() // Each cell is one unit of work "heavy".
//!     .build(); // Construct a GCRA decider.
//! assert_eq!(Ok(()), lim.check());
//! ```
//!
//! The rate-limiter interface is intentionally geared towards only
//! providing callers with the information they need to make decisions
//! about what to do with each cell. Whenever possible, additional
//! information about why a cell should be denied - the `GCRA`
//! implementation will return a `time::Instant` alongside the decision to
//! allow callers to e.g. provide better error messages to users.
//!
//! Due to this, the `ratelimit_meter` crate does not provide any facility
//! to wait until a cell would be allowed - if you require this, you
//! should use the `Instant` returned with negative decisions and wait
//! in your own, e.g. event loop.
//!
//! ## Rate-limiting Algorithms
//!
//! ### Design and implementation of GCRA
//!
//! The GCRA limits the rate of cells by determining when the "next"
//! cell is expected to arrive; any cells that arrive before that time
//! are classified as non-conforming; the methods for checking cells
//! also return an expected arrival time for these cells, so that
//! callers can choose to wait (adding jitter), or reject the cell.
//!
//! Since using the GCRA results in a much smoother usage pattern, it
//! appears to be very useful for "outgoing" traffic behaviors,
//! e.g. throttling API call rates, or emails sent to a person in a
//! period of time.
//!
//! Unlike token or leaky bucket algorithms, the GCRA assumes that all
//! units of work are of the same "weight", and so allows some
//! optimizations which result in much more concise and fast code (it
//! does not even use multiplication or division in the "hot" path).
//!
//! See [the documentation of the GCRA type](algorithms/gcra/struct.GCRA.html) for
//! more details on its implementation and on trade-offs that apply to
//! it.
//!
//! ### Design and implementation of the leaky bucket
//!
//! In contrast to the GCRA, the leaky bucket algorithm does not place
//! any constraints on the next cell's arrival time: Whenever there is
//! capacity left in the bucket, it can be used. This means that the
//! distribution of "yes" decisions from heavy usage on the leaky
//! bucket rate-limiter will be clustered together. On average, the
//! cell rates of both the GCRA and the leaky bucket will be the same,
//! but in terms of observable behavior, the leaky bucket will appear
//! to allow requests at a more predictable rate.
//!
//! This kind of behavior is usually what people of online APIs expect
//! these days, which makes the leaky bucket a very popular technique
//! for rate-limiting on these kinds of services.
//!
//! The leaky bucket algorithm implemented in this crate is fairly
//! standard: It only updates the bucket fill gauge when a cell is
//! checked, and supports checking "batches" of cells in a single call
//! with no problems.
//!
//! ## Thread-safe operation
//!
//! The implementations in this crate use compare-and-set to keep
//! state, and are safe to share across threads..
//!
//! Example:
//!
//! ```
//! use std::thread;
//! use std::time::Duration;
//! use ratelimit_meter::{Decider, GCRA};
//!
//! let mut lim = GCRA::for_capacity(50).unwrap() // Allow 50 units of work
//!     .per(Duration::from_secs(1)) // We calculate per-second (this is the default).
//!     .cell_weight(1).unwrap() // Each cell is one unit of work "heavy".
//!     .build(); // Construct a GCRA decider.
//! let mut thread_lim = lim.clone();
//! thread::spawn(move || { assert_eq!(Ok(()), thread_lim.check());});
//! assert_eq!(Ok(()), lim.check());
//! ```

pub mod algorithms;
pub mod example_algorithms;
mod implementation;
mod thread_safety;

extern crate failure;
#[macro_use]
extern crate failure_derive;

use std::time::{Duration, Instant};

use implementation::*;

pub use self::algorithms::LeakyBucket;
pub use self::algorithms::GCRA;

/// Provides additional information about non-conforming cells, most
/// importantly the earliest time until the next cell could be
/// considered conforming.
///
/// Since this does not account for effects like thundering herds,
/// users should always add random jitter to the times given.
#[derive(Fail, Debug, PartialEq)]
#[fail(display = "rate-limited, wait at least {:?}", min_time)]
pub struct NonConformance {
    t0: Instant,
    min_time: Duration,
}

impl NonConformance {
    pub(crate) fn new(t0: Instant, min_time: Duration) -> NonConformance {
        NonConformance { t0, min_time }
    }
}

impl NonConformance {
    /// Returns the earliest time at which a decision could be
    /// conforming (excluding conforming decisions made by the Decider
    /// that are made in the meantime).
    pub fn earliest_possible(&self) -> Instant {
        self.t0 + self.min_time
    }

    /// Returns the minimum amount of time from the time that the
    /// decision was made (relative to the `at` argument in a
    /// `Decider`'s `check_at` method) that must pass before a
    /// decision can be conforming. Since Durations can not be
    /// negative, a zero duration is returned if `from` is already
    /// after that duration.
    pub fn wait_time_from(&self, from: Instant) -> Duration {
        if from == self.t0 {
            self.min_time
        } else if from < self.t0 + self.min_time {
            (self.t0 + self.min_time).duration_since(from)
        } else {
            Duration::new(0, 0)
        }
    }

    /// Returns the minimum amount of time (down to 0) that needs to
    /// pass from the current instant for the Decider to consider a
    /// cell conforming again.
    pub fn wait_time(&self) -> Duration {
        self.wait_time_from(Instant::now())
    }
}

/// Gives additional information about the negative outcome of a batch
/// cell decision.
///
/// Since batch queries can be made for batch sizes bigger than a
/// Decider could accomodate, there are now two possible negative
/// outcomes:
///
///   * `BatchNonConforming` - the query is valid but the Decider can
///     not accomodate them.
///
///   * `InsufficientCapacity` - the Decider can never accomodate the
///     cells queried for.
#[derive(Fail, Debug, PartialEq)]
pub enum NegativeMultiDecision {
    /// A batch of cells (the first argument) is non-conforming and
    /// can not be let through at this time. The second argument gives
    /// information about when that batch of cells might be let
    /// through again (not accounting for thundering herds and other,
    /// simultaneous decisions).
    #[fail(display = "{} cells: {}", _0, _1)]
    BatchNonConforming(u32, NonConformance),

    /// The number of cells tested (the first argument) is larger than
    /// the bucket's capacity, which means the decision can never have
    /// a conforming result.
    #[fail(display = "bucket does not have enough capacity to accomodate {} cells", _0)]
    InsufficientCapacity(u32),
}

/// The main decision trait. It allows checking a single cell against
/// the rate-limiter, either at the current time instant, or at a
/// given instant in time, updating the `Decider`'s internal state if
/// the cell is conforming.
pub trait Decider: DeciderImpl {
    /// Tests is a single cell can be accommodated at the given time
    /// stamp. If it can be, `check` updates the `Decider` to account
    /// for the conforming cell and returns `Ok(())`.
    ///
    /// If the cell is non-conforming (i.e., it can't be accomodated
    /// at this time stamp), `check_at` returns `Err` with information
    /// about the earliest time at which a cell could be considered
    /// conforming (see [`NonConformance`](struct.NonConformance.html)).
    fn check_at(&mut self, at: Instant) -> Result<(), NonConformance> {
        self.test_and_update(at)
    }

    /// Tests if a single cell can be accommodated at
    /// `Instant::now()`. See [`check_at`](#method.check_at).
    fn check(&mut self) -> Result<(), NonConformance> {
        self.test_and_update(Instant::now())
    }
}

/// The "batch" decision trait, allowing a Decider to make a decision
/// about multiple cells at once.
pub trait MultiDecider: MultiDeciderImpl {
    /// Tests if `n` cells can be accommodated at the given time
    /// stamp. If (and only if) all cells in the batch can be
    /// accomodated, the `MultiDecider` updates the internal state to
    /// account for all cells and returns `Ok(())`.
    ///
    /// If the entire batch of cells would not be conforming but the
    /// `MultiDecider` has the capacity to accomodate the cells at any
    /// point in time, `check_n_at` returns error
    /// [`NegativeMultiDecision::BatchNonConforming`](enum.NegativeMultiDecision.html#variant.BatchNonConforming),
    /// holding the number of cells and
    /// [`NonConformance`](struct.NonConformance.html) information.
    ///
    /// If `n` exceeds the bucket capacity, `check_n_at` returns
    /// [`NegativeMultiDecision::InsufficientCapacity`](enum.NegativeMultiDecision.html#variant.InsufficientCapacity),
    /// indicating that a batch of this many cells can never succeed.
    fn check_n_at(&mut self, n: u32, at: Instant) -> Result<(), NegativeMultiDecision> {
        self.test_n_and_update(n, at)
    }

    /// Tests if `n` cells can be accommodated at the current time
    /// (`Instant::now()`), using [`check_n_at`](#method.check_n_at)
    fn check_n(&mut self, n: u32) -> Result<(), NegativeMultiDecision> {
        self.test_n_and_update(n, Instant::now())
    }
}

#[test]
fn test_wait_time_from() {
    let now = Instant::now();
    let nc = NonConformance::new(now, Duration::from_secs(20));
    assert_eq!(nc.wait_time_from(now), Duration::from_secs(20));
    assert_eq!(
        nc.wait_time_from(now + Duration::from_secs(5)),
        Duration::from_secs(15)
    );
}
