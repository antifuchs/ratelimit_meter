use std::fmt;
use std::num::NonZeroU32;
use std::time::Duration;
use {
    algorithms::{Algorithm, RateLimitState},
    DirectRateLimiter, InconsistentCapacity, NegativeMultiDecision,
};

use std::time::Instant;

/// The most naive implementation of a rate-limiter ever: Always
/// allows every cell through.
/// # Example
/// ```
/// use ratelimit_meter::DirectRateLimiter;
/// use ratelimit_meter::example_algorithms::Allower;
/// let mut allower = Allower::ratelimiter();
/// assert!(allower.check().is_ok());
/// ```
#[derive(Default, Copy, Clone, Debug)]
pub struct Allower {}

impl Allower {
    /// Return a rate-limiter that lies, i.e. that allows all requests
    /// through.
    pub fn ratelimiter() -> DirectRateLimiter<Allower> {
        // These numbers are fake, but we make them up for convenience:
        DirectRateLimiter::per_second(nonzero!(1u32))
    }
}

impl RateLimitState<Allower> for () {
    fn last_touched(&self, _params: &Allower) -> Instant {
        Instant::now()
    }
}

#[derive(Debug, PartialEq)]
pub struct Impossible();

impl fmt::Display for Impossible {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "can't happen")
    }
}

impl Algorithm for Allower {
    type BucketState = ();
    type NegativeDecision = Impossible;

    fn construct(
        _capacity: NonZeroU32,
        _cell_weight: NonZeroU32,
        _per_time_unit: Duration,
    ) -> Result<Self, InconsistentCapacity> {
        Ok(Allower {})
    }

    /// Allows all cells through unconditionally.
    fn test_n_and_update(
        &self,
        _state: &Self::BucketState,
        _n: u32,
        _t0: Instant,
    ) -> Result<(), NegativeMultiDecision<Impossible>> {
        Ok(())
    }
}
