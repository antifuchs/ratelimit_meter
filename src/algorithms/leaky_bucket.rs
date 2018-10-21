//! A classic leaky bucket algorithm

use std::num::NonZeroU32;
use thread_safety::ThreadsafeWrapper;
use {algorithms::Algorithm, InconsistentCapacity, NegativeMultiDecision, NonConformance};

use evmap::ShallowCopy;

use std::cmp;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
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
/// # use std::num::NonZeroU32;
/// # use ratelimit_meter::{DirectRateLimiter, LeakyBucket};
/// let mut lb = DirectRateLimiter::<LeakyBucket>::per_second(NonZeroU32::new(2).unwrap());
/// assert_eq!(Ok(()), lb.check());
/// ```
pub struct LeakyBucket {}

/// Represents the state of a single history of decisions.
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct State(ThreadsafeWrapper<BucketState>);

impl ShallowCopy for State {
    unsafe fn shallow_copy(&mut self) -> Self {
        State(self.0.shallow_copy())
    }
}

/// Represents the parameters affecting all decisions made using a
/// single rate limiter - the total capacity of the bucket, and the
/// interval during which a full new token's "volume" drips out.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Params {
    full: Duration,
    token_interval: Duration,
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
    type BucketParams = Params;

    fn params_from_constructor(
        capacity: NonZeroU32,
        cell_weight: NonZeroU32,
        per_time_unit: Duration,
    ) -> Result<Self::BucketParams, InconsistentCapacity> {
        if capacity < cell_weight {
            return Err(InconsistentCapacity {
                capacity,
                cell_weight,
            });
        }
        let token_interval = (per_time_unit * cell_weight.get()) / capacity.get();
        Ok(Params {
            full: per_time_unit,
            token_interval,
        })
    }

    fn test_n_and_update(
        state: &mut Self::BucketState,
        params: &Self::BucketParams,
        n: u32,
        t0: Instant,
    ) -> Result<(), NegativeMultiDecision> {
        let full = params.full;
        let weight = params.token_interval * n;
        if weight > params.full {
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
                        NonConformance::new(t0, wait_period),
                    )),
                    None,
                )
            }
        })
    }
}
