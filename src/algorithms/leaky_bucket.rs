use {Decider, ImpliedDeciderImpl, MultiDecider, MultiDeciderImpl, NonConforming, TypedDecider};
use algorithms::InconsistentCapacity;

use std::sync::atomic::Ordering::{Acquire, Release};
use std::time::{Duration, Instant};
use std::cmp;
use std::sync::Arc;

use crossbeam::epoch::{self, Atomic, Owned};

impl Decider for LeakyBucket {}

impl MultiDecider for LeakyBucket {}

impl TypedDecider for LeakyBucket {
    /// The leaky bucket can provide an approximation for how long to
    /// sleep until one token is available again. (This does not
    /// account for multiple requests attempting to use the same
    /// token; schedulers relying on this must account for phenomena
    /// like thundering herds.)
    type T = Duration;
}

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
/// # use ratelimit_meter::{Decider, LeakyBucket};
/// let mut lb: LeakyBucket = LeakyBucket::per_second(2).unwrap();
/// assert_eq!(Ok(()), lb.check());
/// ```
pub struct LeakyBucket {
    state: Arc<Atomic<BucketState>>,
    full: Duration,
    token_interval: Duration,
}

#[derive(Debug, Clone)]
struct BucketState {
    level: Duration,
    last_update: Option<Instant>,
}

impl LeakyBucket {
    /// Constructs and returns a leaky-bucket rate-limiter allowing as
    /// many cells on average as the given capacity per time duration.
    /// ## Example
    /// ``` rust
    /// # use ratelimit_meter::{Decider, LeakyBucket};
    /// # use std::time::{Duration, Instant};
    /// let now = Instant::now();
    /// let day = Duration::from_secs(86400);
    /// let mut lb = LeakyBucket::new(1, day).unwrap(); // 1 per day
    /// assert!(lb.check_at(now).is_ok());
    ///
    /// assert!(!lb.check_at(now + day/2).is_ok()); // Can't do it half a day later
    /// assert!(lb.check_at(now + day).is_ok()); // Have to wait a day
    /// // ...and then, a day after that.
    /// assert!(lb.check_at(now + day * 2).is_ok());
    /// ```
    pub fn new(capacity: u32, per_duration: Duration) -> Result<LeakyBucket, InconsistentCapacity> {
        if capacity == 0 {
            return Err(InconsistentCapacity {
                capacity: capacity,
                weight: 0,
            });
        }
        let token_interval = per_duration / capacity;
        let state = Atomic::new(BucketState {
            level: Duration::new(0, 0),
            last_update: None,
        });
        Ok(LeakyBucket {
            state: Arc::new(state),
            token_interval: token_interval,
            full: per_duration,
        })
    }

    /// Constructs and returns a leaky-bucket rate-limiter allowing on
    /// average `capacity`/1s cells.
    pub fn per_second(capacity: u32) -> Result<LeakyBucket, InconsistentCapacity> {
        LeakyBucket::new(capacity, Duration::from_secs(1))
    }
}

impl MultiDeciderImpl for LeakyBucket {
    fn test_n_and_update(&mut self, n: u32, t0: Instant) -> Result<(), NonConforming<Duration>> {
        let weight = self.token_interval * n;
        if weight > self.full {
            return Err(NonConforming::InsufficientCapacity(n).into());
        }
        let mut new = Owned::new(BucketState {
            last_update: Some(t0),
            level: Duration::new(0, 0),
        });
        let guard = epoch::pin();

        loop {
            if let Some(state) = self.state.load(Acquire, &guard) {
                let last = state.last_update.unwrap_or(t0);
                // Prevent time travel: If any parallel calls get re-ordered,
                // or any tests attempt silly things, make sure to answer from
                // the last query onwards instead.
                let t0 = cmp::max(t0, last);
                // Decrement the level by the amount the bucket
                // has dripped in the meantime:
                new.level = state.level - cmp::min(t0 - last, state.level);

                // Determine if the cell fits & ensure it is recorded:
                if weight + new.level <= self.full {
                    new.level += weight;
                    match self.state.cas_and_ref(Some(state), new, Release, &guard) {
                        Ok(_) => {
                            return Ok(());
                        }
                        Err(owned) => {
                            new = owned;
                        }
                    }
                } else {
                    let wait_period = (weight + new.level) - self.full;
                    return Err(NonConforming::No(wait_period));
                }
            }
        }
    }
}

impl ImpliedDeciderImpl for LeakyBucket {}
