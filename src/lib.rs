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

pub mod example_algorithms;
#[allow(deprecated)]
pub mod algorithms;
mod implementation;

extern crate crossbeam;
extern crate failure;
#[macro_use]
extern crate failure_derive;

use std::time::Instant;

pub use self::algorithms::*;
use implementation::*;

/// A prerequisite for implementing any Decider trait. It provides the
/// associated type for [`Decision`](enum.Decision.html)'s additional
/// information for negative decisions.
pub trait TypedDecider {
    /// The type for additional information on negative decisions.
    type T;
}

#[derive(Fail, Debug, PartialEq)]
/// Returned in the non-conforming case. The rate-limiting algorithm
/// may return additional information (e.g. time to wait until the
/// next conforming cell can be allowed through).
pub enum NonConforming<T> {
    /// A single cell or a batch of cells is non-conforming and can
    /// not be let through at this time. A `Decider` implementation
    /// can provide additional information about when the next cell
    /// might be let through again.
    #[fail(display = "Rate-limited.")]
    No(T),

    /// The bucket's capacity is less than the number of cells that
    /// try to fit into it. This error indicates that the request can
    /// never be accomodated.
    #[fail(display = "bucket does not have enough capacity to accomodate {} cells", _0)]
    InsufficientCapacity(u32),
}

/// The main decision trait. It allows checking a single cell against
/// the rate-limiter, either at the current time instant, or at a
/// given instant in time, both destructively.
pub trait Decider: DeciderImpl {
    /// Tests if a single cell can be accommodated at
    /// `Instant::now()`. See [`check_at`](#method.check_at).
    fn check(&mut self) -> Result<(), NonConforming<Self::T>> {
        self.test_and_update(Instant::now())
    }

    /// Tests is a single cell can be accommodated at the given time
    /// stamp.
    fn check_at(&mut self, at: Instant) -> Result<(), NonConforming<Self::T>> {
        self.test_and_update(at)
    }
}

pub trait MultiDecider: MultiDeciderImpl {
    /// Tests if `n` cells can be accommodated at the given time
    /// stamp. An error [`NonConforming::InsufficientCapacity`](enum.NonConforming.html#variant.InsufficientCapacity) is
    /// returned if `n` exceeds the bucket capacity.
    fn check_n_at(&mut self, n: u32, at: Instant) -> Result<(), NonConforming<Self::T>> {
        self.test_n_and_update(n, at)
    }

    /// Tests if `n` cells can be accommodated at the current time
    /// (`Instant::now()`). An error
    /// [`NonConforming::InsufficientCapacity`](enum.NonConforming.html#variant.InsufficientCapacity)
    /// is returned if `n` exceeds the bucket capacity.
    fn check_n(&mut self, n: u32) -> Result<(), NonConforming<Self::T>> {
        self.test_n_and_update(n, Instant::now())
    }
}
