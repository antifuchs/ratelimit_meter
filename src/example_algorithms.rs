use crate::lib::*;
use crate::{
    algorithms::{Algorithm, RateLimitState, RateLimitStateWithClock},
    instant,
    instant::Absolute,
    DirectRateLimiter, InconsistentCapacity, NegativeMultiDecision,
};

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
    pub fn ratelimiter() -> DirectRateLimiter<Allower, Always> {
        // These numbers are fake, but we make them up for convenience:
        DirectRateLimiter::per_second(nonzero!(1u32))
    }
}

impl RateLimitState<Allower, Always> for () {}

impl RateLimitStateWithClock<Allower, Always> for () {
    fn last_touched(&self, _params: &Allower) -> Always {
        Always::now()
    }
}

/// A non-error - the Allower example rate-limiter always returns a
/// positive result, so this error is never returned.
#[derive(Debug, PartialEq)]
pub enum Impossible {}

impl fmt::Display for Impossible {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "can't happen")
    }
}

impl Algorithm<Always> for Allower {
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
        _t0: Always,
    ) -> Result<(), NegativeMultiDecision<Impossible>> {
        Ok(())
    }
}

/// A pseudo-instant that never changes.
///
/// It is used to implement the `Allower` rate-limiter type, which
/// never denies any requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Always();
impl instant::Relative for Always {
    fn duration_since(&self, _other: Self) -> Duration {
        Duration::new(0, 0)
    }
}

impl instant::Absolute for Always {
    fn now() -> Self {
        Always()
    }
}

impl Add<Duration> for Always {
    type Output = Always;
    fn add(self, _rhs: Duration) -> Always {
        Always()
    }
}

impl Sub<Duration> for Always {
    type Output = Always;
    fn sub(self, _rhs: Duration) -> Always {
        Always()
    }
}
