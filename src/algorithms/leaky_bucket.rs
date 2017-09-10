use {TypedDecider, ImpliedDeciderImpl, MultiDeciderImpl, Decider, MultiDecider, Decision,
     Threadsafe, Result, ErrorKind};

use std::time::{Instant, Duration};
use std::cmp;

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
/// wait time until the cell can be accomodated. This minimum wait
/// time does not account for thundering herd effects or other
/// problems in concurrent resource acquisition, so users of this
/// library must take care to apply positive jitter to these wait
/// times.
///
/// # Example
/// ``` rust
/// # use ratelimit_meter::{Decider, LeakyBucket, Decision};
/// let mut lb: LeakyBucket = LeakyBucket::per_second(2).unwrap();
/// assert_eq!(Decision::Yes, lb.check().unwrap());
/// ```
pub struct LeakyBucket {
    last: Option<Instant>,
    full: Duration,
    current: Duration,
    token_interval: Duration,
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
    /// assert!(lb.check_at(now).unwrap().is_compliant());
    ///
    /// assert!(!lb.check_at(now + day/2).unwrap().is_compliant()); // Can't do it half a day later
    /// assert!(lb.check_at(now + day).unwrap().is_compliant()); // Have to wait a day
    /// assert!(lb.check_at(now + day * 2).unwrap().is_compliant()); // ...and then, a day after that.
    /// ```
    pub fn new(capacity: u32, per_duration: Duration) -> Result<LeakyBucket> {
        if capacity == 0 {
            return Err(ErrorKind::InconsistentCapacity(capacity, 0).into());
        }
        let token_interval = per_duration / capacity;
        Ok(LeakyBucket {
            last: None,
            current: Duration::new(0, 0),
            token_interval: token_interval,
            full: per_duration,
        })
    }

    /// Constructs and returns a leaky-bucket rate-limiter allowing on
    /// average `capacity`/1s cells.
    pub fn per_second(capacity: u32) -> Result<LeakyBucket> {
        LeakyBucket::new(capacity, Duration::from_secs(1))
    }

    /// Wraps the current leaky bucket in a
    /// [`Threadsafe`](../struct.Threadsafe.html).
    pub fn threadsafe(self) -> Threadsafe<LeakyBucket> {
        Threadsafe::new(self)
    }
}

impl MultiDeciderImpl for LeakyBucket {
    fn test_n_and_update(&mut self, n: u32, t0: Instant) -> Result<Decision<Duration>> {
        if self.token_interval * n > self.full {
            return Err(ErrorKind::InsufficientCapacity(n).into());
        }

        let current = self.current;
        let last = match self.last {
            None => {
                self.last = Some(t0);
                t0
            }
            Some(t) => t,
        };
        // Prevent time travel: If any parallel calls get re-ordered,
        // or any tests attempt silly things, make sure to answer from
        // the last query onwards instead.
        let t0 = cmp::max(t0, last);

        self.current = current - cmp::min(t0 - last, current);
        self.last = Some(t0);

        let weight = self.token_interval * n;
        if weight + self.current <= self.full {
            self.current = self.current + weight;
            Ok(Decision::Yes)
        } else {
            let wait_period = (weight + current) - self.full;
            Ok(Decision::No(wait_period))
        }
    }
}

impl ImpliedDeciderImpl for LeakyBucket {}
