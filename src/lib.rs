//! # Leaky Bucket Rate-Limiting (as a meter) in Rust
//! This crate implements
//! the
//! [generic cell rate algorithm](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm) (GCRA)
//! for rate-limiting and scheduling in Rust.
//!
//! ## Interface
//!
//! You construct a rate limiter using the `Limiter` builder:
//!
//! ``` rust
//! use std::time::Duration;
//! use ratelimit_meter::{Limiter, Decider, GCRA, Decision};
//!
//! let mut lim = Limiter::new()
//!     .time_unit(Duration::from_secs(1)) // We calculate per-second (this is the default).
//!     .capacity(50) // Allow 50 units of work per second
//!     .weight(1) // Each cell is one unit of work "heavy".
//!     .build::<GCRA>().unwrap(); // Construct a non-threadsafe GCRA decider.
//! assert_eq!(Decision::Yes, lim.check().unwrap());
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
//! ## Design and implementation
//!
//! Unlike some other token bucket algorithms, the GCRA one assumes that
//! all units of work are of the same "weight", and so allows some
//! optimizations which result in much more consise and fast code (it does
//! not even use multiplication or division in the "hot" path).
//!
//! The trade-off here this is that there is currently no support for
//! assigning different weights to incoming cells (say, particularly
//! heavy api calls vs. lightweight ones) using the same rate-limiter
//! structure.
//!
//! ## Thread-safe operation
//!
//! The default GCRA implementation can not be used across
//! threads. However, there is a wrapper struct `Threadsafe`, that wraps
//! the hot path in an atomically reference-counted mutex. It still
//! manages to be pretty fast (see the benchmarks above), but the lock
//! comes with an overhead even in single-threaded operation.
//!
//! Example:
//!
//! ```
//! use std::time::Duration;
//! use ratelimit_meter::{Limiter, Decider, GCRA, Threadsafe, Decision};
//!
//! let mut lim = Limiter::new()
//!     .time_unit(Duration::from_secs(1)) // We calculate per-second (this is the default).
//!     .capacity(50) // Allow 50 units of work per second
//!     .weight(1) // Each cell is one unit of work "heavy".
//!     .build::<Threadsafe<GCRA>>().unwrap(); // Construct a threadsafe GCRA decider.
//! assert_eq!(Decision::Yes, lim.check().unwrap());
//! ```

pub mod example_algorithms;
pub mod errors;
mod algorithms;

#[macro_use]
extern crate error_chain;

use std::time::{Instant, Duration};

pub use errors::*;
pub use self::algorithms::*;

#[derive(PartialEq, Debug)]
/// A decision on a single cell from the metered rate-limiter.
pub enum Decision<T> {
    /// The cell is conforming, allow it through.
    Yes,

    /// The cell is non-conforming. A rate-limiting algorithm
    /// implementation may return additional information for the
    /// caller, e.g. a time when the cell was expected to arrive.
    No(T),
}

impl<T> Decision<T> {
    /// Check if a decision on a cell indicates the cell is compliant
    /// or not. Returns `true` iff the cell was compliant, i.e. the
    /// decision was `Decision::Yes`.
    ///
    /// Note: This method is mostly useful in tests.
    pub fn is_compliant(&self) -> bool {
        match self {
            &Decision::Yes => true,
            &Decision::No(_) => false,
        }
    }
}

/// A builder object that can be used to construct rate-limiters as
/// meters.
pub struct Limiter {
    capacity: Option<u32>,
    weight: Option<u32>,
    time_unit: Duration,
}

/// A builder pattern implementation that can construct deciders.
/// # Basic example
/// This example constructs a decider that considers every cell
/// compliant:
///
/// ```
/// # use ratelimit_meter::{Limiter, Decider};
/// # use ratelimit_meter::example_algorithms::Allower;
///
/// let mut limiter = Limiter::new().build::<Allower>().unwrap();
/// for _i in 1..3 {
///     println!("{:?}...", limiter.check());
/// }
/// ```
impl Limiter {
    /// Returns a default (useless) limiter without a capacity or cell
    /// weight, and a time_unit of 1 second.
    pub fn new() -> Limiter {
        Limiter {
            capacity: None,
            weight: None,
            time_unit: Duration::from_secs(1),
        }
    }

    /// Sets the capacity of the limiter's "bucket" in elements per `time_unit`.
    ///
    /// See [`time_unit`](#method.time_unit).
    pub fn capacity<'a>(&'a mut self, capacity: u32) -> &'a mut Limiter {
        self.capacity = Some(capacity);
        self
    }

    /// Sets the "weight" of each cell being checked against the
    /// bucket. Each cell fills the bucket by this much.
    pub fn weight<'a>(&'a mut self, weight: u32) -> &'a mut Limiter {
        self.weight = Some(weight);
        self
    }

    /// Sets the "unit of time" within which the bucket drains.
    ///
    /// The assumption is that in a period of `time_unit` (if no cells
    /// are being checked), the bucket is fully drained.
    pub fn time_unit<'a>(&'a mut self, time_unit: Duration) -> &'a mut Limiter {
        self.time_unit = time_unit;
        self
    }

    /// Builds and returns a concrete structure that implements the Decider trait.
    pub fn build<D>(&self) -> Result<D>
        where D: Decider
    {
        D::build_with(self)
    }
}

/// The trait that implementations of the metered rate-limiter
/// interface have to implement. Users of this library should rely on
/// [Decider](trait.Decider.html) for the external interface.
pub trait DeciderImpl {
    /// The (optional) type for additional information on negative
    /// decisions.
    type T;

    /// Tests if a single cell can be accomodated in the rate limiter
    /// at the instant `at` and updates the rate-limiter to account
    /// for the weight of the cell.
    ///
    /// This method is not meant to be called by users,
    fn test_and_update(&mut self, at: Instant) -> Result<Decision<Self::T>>;

    /// Converts the limiter builder into a concrete decider structure.
    fn build_with(l: &Limiter) -> Result<Self> where Self: Sized;
}

/// The external interface offered by all rate-limiting implementations.
pub trait Decider: DeciderImpl {
    /// Tests if a single cell can be accomodated at
    /// `Instant::now()`. See [`check_at`](#method.check_at).
    fn check(&mut self) -> Result<Decision<Self::T>> {
        self.test_and_update(Instant::now())
    }

    /// Tests is a single cell can be accomodated at the given time
    /// stamp.
    fn check_at(&mut self, at: Instant) -> Result<Decision<Self::T>> {
        self.test_and_update(at)
    }
}
