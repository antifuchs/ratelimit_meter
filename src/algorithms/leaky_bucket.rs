//! A classic leaky bucket algorithm

use std::fmt;
use std::num::NonZeroU32;
use thread_safety::ThreadsafeWrapper;
use {
    algorithms::{Algorithm, RateLimitState},
    InconsistentCapacity, NegativeMultiDecision, NonConformance,
};

use evmap::ShallowCopy;

use std::cmp;
use std::time::{Duration, Instant};

/// Implements the industry-standard leaky bucket rate-limiting
/// as-a-meter. The bucket keeps a "fill height", pretending to drip
/// steadily (which reduces the fill height), and increases the fill
/// height with every cell that is found conforming. If cells would
/// make the bucket overflow, they count as non-conforming.
///
/// # Drip implementation
///
/// Instead of having a background task update the bucket's fill
/// level, this implementation re-computes the fill level of the
/// bucket on every call to [`check`](#method.check) and related
/// methods.
///
/// # Wait time calculation
///
/// If the cell does not fit, this implementation computes the minimum
/// wait time until the cell can be accommodated. This minimum wait
/// time does not account for thundering herd effects or other
/// problems in concurrent resource acquisition, so users of this
/// library must take care to apply positive jitter to these wait
/// times.
///
/// # Example
/// ``` rust
/// # use ratelimit_meter::{DirectRateLimiter, LeakyBucket};
/// # #[macro_use] extern crate nonzero_ext;
/// # extern crate ratelimit_meter;
/// # fn main () {
/// let mut lb = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(2u32));
/// assert_eq!(Ok(()), lb.check());
/// # }
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LeakyBucket {
    full: Duration,
    token_interval: Duration,
}

/// Represents the state of a single history of decisions.
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct State(ThreadsafeWrapper<BucketState>);

impl ShallowCopy for State {
    unsafe fn shallow_copy(&mut self) -> Self {
        State(self.0.shallow_copy())
    }
}

impl RateLimitState<LeakyBucket> for State {
    fn last_touched(&self, _params: &LeakyBucket) -> Instant {
        let data = self.0.snapshot();
        data.last_update.unwrap_or_else(Instant::now) + data.level
    }
}

/// Returned in case of a negative rate-limiting decision.
///
/// To avoid the thundering herd effect, client code should always add
/// some jitter to the wait time.
#[derive(Debug, PartialEq)]
pub struct TooEarly(Instant, Duration);

impl fmt::Display for TooEarly {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "rate-limited until {:?}", self.0 + self.1)
    }
}

impl NonConformance for TooEarly {
    #[inline]
    fn earliest_possible(&self) -> Instant {
        self.0 + self.1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BucketState {
    level: Duration,
    last_update: Option<Instant>,
}

impl Default for BucketState {
    fn default() -> Self {
        BucketState {
            level: Duration::new(0, 0),
            last_update: None,
        }
    }
}

impl Algorithm for LeakyBucket {
    type BucketState = State;

    type NegativeDecision = TooEarly;

    fn construct(
        capacity: NonZeroU32,
        cell_weight: NonZeroU32,
        per_time_unit: Duration,
    ) -> Result<Self, InconsistentCapacity> {
        if capacity < cell_weight {
            return Err(InconsistentCapacity::new(capacity, cell_weight));
        }
        let token_interval = (per_time_unit * cell_weight.get()) / capacity.get();
        Ok(LeakyBucket {
            full: per_time_unit,
            token_interval,
        })
    }

    fn test_n_and_update(
        &self,
        state: &Self::BucketState,
        n: u32,
        t0: Instant,
    ) -> Result<(), NegativeMultiDecision<TooEarly>> {
        let full = self.full;
        let weight = self.token_interval * n;
        if weight > self.full {
            return Err(NegativeMultiDecision::InsufficientCapacity(n));
        }
        state.0.measure_and_replace(|state| {
            let mut new = BucketState {
                last_update: Some(t0),
                level: Duration::new(0, 0),
            };
            let last = state.last_update.unwrap_or(t0);
            // Prevent time travel: If any parallel calls get re-ordered,
            // or any tests attempt silly things, make sure to answer from
            // the last query onwards instead.
            let t0 = cmp::max(t0, last);
            // Decrement the level by the amount the bucket
            // has dripped in the meantime:
            new.level = state.level - cmp::min(t0 - last, state.level);
            if weight + new.level <= full {
                new.level += weight;
                (Ok(()), Some(new))
            } else {
                let wait_period = (weight + new.level) - full;
                (
                    Err(NegativeMultiDecision::BatchNonConforming(
                        n,
                        TooEarly(t0, wait_period),
                    )),
                    None,
                )
            }
        })
    }
}
