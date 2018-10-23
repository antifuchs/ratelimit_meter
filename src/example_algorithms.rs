use failure::_core::time::Duration;
use std::num::NonZeroU32;
use {
    algorithms::{Algorithm, RateLimitState},
    DirectRateLimiter, InconsistentCapacity, NegativeMultiDecision,
};

use std::time::Instant;

#[derive(Default, Copy, Clone)]
/// The most naive implementation of a rate-limiter ever: Always
/// allows every cell through.
/// # Example
/// ```
/// use ratelimit_meter::DirectRateLimiter;
/// use ratelimit_meter::example_algorithms::Allower;
/// let mut allower = Allower::ratelimiter();
/// assert!(allower.check().is_ok());
/// ```
pub struct Allower {}

impl Allower {
    /// Return a rate-limiter that lies, i.e. that allows all requests
    /// through.
    pub fn ratelimiter() -> DirectRateLimiter<Allower> {
        // These numbers are fake, but we make them up for convenience:
        DirectRateLimiter::per_second(nonzero!(1u32))
    }
}

impl RateLimitState<()> for () {
    fn last_touched(&self, _params: &()) -> Instant {
        Instant::now()
    }
}

#[derive(Fail, Debug, PartialEq)]
#[fail(display = "Should never happen")]
pub struct Impossible();

impl Algorithm for Allower {
    type BucketState = ();
    type BucketParams = ();
    type NegativeDecision = Impossible;

    fn params_from_constructor(
        _capacity: NonZeroU32,
        _cell_weight: NonZeroU32,
        _per_time_unit: Duration,
    ) -> Result<Self::BucketParams, InconsistentCapacity> {
        Ok(())
    }

    /// Allows all cells through unconditionally.
    fn test_n_and_update(
        _state: &Self::BucketState,
        _params: &Self::BucketParams,
        _n: u32,
        _t0: Instant,
    ) -> Result<(), NegativeMultiDecision<Impossible>> {
        Ok(())
    }
}
