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
/// # use ratelimit_meter::{Limiter, Decider, Allower};
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
