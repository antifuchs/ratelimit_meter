//! # Leaky Bucket Rate-Limiting (as a meter) in Rust
//! This crate implements
//! the
//! [generic cell rate algorithm](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm) (GCRA)
//! for rate-limiting and scheduling in Rust.
//!
//! ## Interface
//!
//! There is currently one rate limiter implementation in this crate,
//! the Generic Cell Rate Algorithm. Use it by creating a builder from
//! the [`GCRA`](struct.GCRA.html) struct:
//!
//! ``` rust
//! use std::time::Duration;
//! use ratelimit_meter::{Decider, GCRA, Decision};
//!
//! let mut lim = GCRA::for_capacity(50).unwrap() // Allow 50 units of work
//!     .per(Duration::from_secs(1)) // We calculate per-second (this is the default).
//!     .cell_weight(1).unwrap() // Each cell is one unit of work "heavy".
//!     .build(); // Construct a non-threadsafe GCRA decider.
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
//! ## Design and implementation of GCRA
//!
//! Unlike token bucket algorithms, the GCRA one assumes that all
//! units of work are of the same "weight", and so allows some
//! optimizations which result in much more consise and fast code (it
//! does not even use multiplication or division in the "hot" path).
//!
//! See [the documentation of the GCRA type](algorithms/gcra/struct.GCRA.html) for
//! more details on its implementation and on trade-offs that apply to
//! it.
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
//! use ratelimit_meter::{Decider, GCRA, Decision};
//!
//! let mut lim = GCRA::for_capacity(50).unwrap() // Allow 50 units of work
//!     .per(Duration::from_secs(1)) // We calculate per-second (this is the default).
//!     .cell_weight(1).unwrap() // Each cell is one unit of work "heavy".
//!     .build_sync(); // Construct a threadsafe GCRA decider.
//! assert_eq!(Decision::Yes, lim.check().unwrap());
//! ```

pub mod example_algorithms;
pub mod errors;
pub mod algorithms;
mod implementation;

#[macro_use]
extern crate error_chain;

use std::time::{Instant};

pub use errors::*;
pub use self::algorithms::*;
use implementation::*;

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

/// A prerequisite for implementing any Decider trait. It provides the
/// associated type for [`Decision`](enum.Decision.html)'s additional
/// information for negative decisions.
pub trait TypedDecider {
    /// The type for additional information on negative decisions.
    type T;
}

/// The main decision trait. It allows checking a single cell against
/// the rate-limiter, either at the current time instant, or at a
/// given instant in time, both destructively.
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

pub trait MultiDecider: MultiDeciderImpl {
    /// Tests if `n` cells can be accomodated at the given time
    /// stamp. An error `ErrorKind::CapacityError` is
    /// returned if `n` exceeds the bucket capacity.
    fn check_n_at(&mut self, n: u32, at: Instant) -> Result<Decision<Self::T>> {
        self.test_n_and_update(n, at)
    }

    /// Tests if `n` cells can be accomodated at the current time
    /// (`Instant::now()`). An error `ErrorKind::CapacityError` is
    /// returned if `n` exceeds the bucket capacity.
    fn check_n(&mut self, n: u32) -> Result<Decision<Self::T>> {
        self.test_n_and_update(n, Instant::now())
    }
}
