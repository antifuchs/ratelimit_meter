use std::num::NonZeroU32;
use thread_safety::ThreadsafeWrapper;
use {
    Decider, ImpliedDeciderImpl, MultiDecider, MultiDeciderImpl, NegativeMultiDecision,
    NonConformance,
};

use std::cmp;
use std::time::{Duration, Instant};

impl Decider for LeakyBucket {}

impl MultiDecider for LeakyBucket {}

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
/// # Thread safety
///
/// This implementation uses lock-free techniques to safely update the
/// bucket state in-place.
///
/// # Example
/// ``` rust
/// # use std::num::NonZeroU32;
/// # use ratelimit_meter::{Decider, LeakyBucket};
/// let mut lb: LeakyBucket = LeakyBucket::per_second(NonZeroU32::new(2).unwrap());
/// assert_eq!(Ok(()), lb.check());
/// ```
pub struct LeakyBucket {
    state: ThreadsafeWrapper<BucketState>,
    full: Duration,
    token_interval: Duration,
}

#[derive(Debug, Clone)]
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

impl LeakyBucket {
    /// Constructs and returns a leaky-bucket rate-limiter allowing as
    /// many cells on average as the given capacity per time duration.
    /// ## Example
    /// ``` rust
    /// # use std::num::NonZeroU32;
    /// # use ratelimit_meter::{Decider, LeakyBucket};
    /// # use std::time::{Duration, Instant};
    /// let now = Instant::now();
    /// let day = Duration::from_secs(86400);
    /// let mut lb = LeakyBucket::new(NonZeroU32::new(1).unwrap(), day); // 1 per day
    /// assert!(lb.check_at(now).is_ok());
    ///
    /// assert!(!lb.check_at(now + day/2).is_ok()); // Can't do it half a day later
    /// assert!(lb.check_at(now + day).is_ok()); // Have to wait a day
    /// // ...and then, a day after that.
    /// assert!(lb.check_at(now + day * 2).is_ok());
    /// ```
    pub fn new(capacity: NonZeroU32, per_duration: Duration) -> LeakyBucket {
        let token_interval = per_duration / capacity.get();
        LeakyBucket {
            state: ThreadsafeWrapper::new(BucketState::default()),
            token_interval,
            full: per_duration,
        }
    }

    /// Constructs and returns a leaky-bucket rate-limiter allowing on
    /// average `capacity`/1s cells.
    pub fn per_second(capacity: NonZeroU32) -> LeakyBucket {
        LeakyBucket::new(capacity, Duration::from_secs(1))
    }
}

impl MultiDeciderImpl for LeakyBucket {
    fn test_n_and_update(&mut self, n: u32, t0: Instant) -> Result<(), NegativeMultiDecision> {
        let full = self.full;
        let weight = self.token_interval * n;
        if weight > self.full {
            return Err(NegativeMultiDecision::InsufficientCapacity(n));
        }
        self.state.measure_and_replace(|state| {
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

impl ImpliedDeciderImpl for LeakyBucket {}
