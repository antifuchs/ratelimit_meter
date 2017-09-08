use {DeciderImpl, Decider, Decision, Threadsafe, Result, ErrorKind};

use std::time::{Instant, Duration};
use std::cmp;

#[derive(Debug, Clone)]
/// Implements the virtual scheduling description of the Generic Cell
/// Rate Algorithm, attributed to ITU-T in recommendation I.371
/// Traffic control and congestion control in B-ISDN; from
/// [Wikipedia](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm).
///
/// # Example
/// In this example, we construct a rate-limiter with the GCR
/// algorithm that can accomodate 20 cells per second. This translates
/// to the GCRA parameters τ=1s, T=50ms (that's 1s / 20 cells).
///
/// ```
/// # use ratelimit_meter::{Decider, GCRA, Decision};
/// # use std::time::{Instant, Duration};
/// let mut limiter = GCRA::for_capacity(20).unwrap().cell_weight(1).unwrap().build();
/// let now = Instant::now();
/// let ms = Duration::from_millis(1);
/// assert_eq!(Decision::Yes, limiter.check_at(now).unwrap()); // the first cell is free
/// for i in 0..20 {
///     // Spam a lot:
///     assert_eq!(Decision::Yes, limiter.check_at(now).unwrap(), "at {}", i);
/// }
/// // We have exceeded the bucket capacity:
/// assert!(!limiter.check_at(now).unwrap().is_compliant());
///
/// // After a sufficient time period, cells are allowed again:
/// assert_eq!(Decision::Yes, limiter.check_at(now + ms*50).unwrap());
pub struct GCRA {
    // The "weight" of a single packet in units of time.
    t: Duration,

    // The "capacity" of the bucket.
    tau: Duration,

    // The theoretical arrival time of the next packet.
    tat: Option<Instant>,
}

/// A builder object that can be used to construct rate-limiters as
/// meters.
pub struct Builder {
    capacity: u32,
    cell_weight: u32,
    time_unit: Duration,
}

/// Constructs a concrete GCRA instance.
impl Builder {
    /// Sets the "weight" of each cell being checked against the
    /// bucket. Each cell fills the bucket by this much.
    pub fn cell_weight<'a>(&'a mut self, weight: u32) -> Result<&'a mut Builder> {
        if self.cell_weight > self.capacity {
            return Err(ErrorKind::InconsistentCapacity(self.capacity, weight).into());
        }
        self.cell_weight = weight;
        Ok(self)
    }

    /// Sets the "unit of time" within which the bucket drains.
    ///
    /// The assumption is that in a period of `time_unit` (if no cells
    /// are being checked), the bucket is fully drained.
    pub fn per<'a>(&'a mut self, time_unit: Duration) -> &'a mut Builder {
        self.time_unit = time_unit;
        self
    }

    /// Builds and returns a thread-safe GCRA decider.
    pub fn build_sync(&self) -> Threadsafe<GCRA> {
        Threadsafe::new(self.build())
    }

    /// Builds a single-threaded GCRA decider.
    pub fn build(&self) -> GCRA {
        GCRA {
            t: (self.time_unit / self.capacity) * self.cell_weight,
            tau: self.time_unit,
            tat: None,
        }
    }
}

impl GCRA {
    /// Constructs a builder object for a GCRA rate-limiter with the
    /// given capacity per second, at cell weight=1. See
    /// [`Builder`](gcra/struct.Builder.html) for options.
    pub fn for_capacity(capacity: u32) -> Result<Builder> {
        if capacity == 0 {
            return Err(ErrorKind::InconsistentCapacity(capacity, 0).into());
        }
        Ok(Builder {
            capacity: capacity,
            cell_weight: 1,
            time_unit: Duration::from_secs(1),
        })
    }
}

impl Decider for GCRA {}

impl DeciderImpl for GCRA {
    /// In GCRA, negative decisions come with the time at which the
    /// next cell was expected to arrive; client code of GCRA can use
    /// this to decide what to do with the non-conforming cell.
    type T = Instant;

    /// Tests if a single cell can be accomodated by the
    /// rate-limiter. This is the method described directly in the
    /// GCRA algorithm, and is the fastest.
    fn test_and_update(&mut self, t0: Instant) -> Result<Decision<Instant>> {
        let tat = self.tat.unwrap_or(t0);

        if t0 < tat - self.tau {
            return Ok(Decision::No(tat));
        }
        self.tat = Some(cmp::max(tat, t0) + self.t);
        Ok(Decision::Yes)
    }

    /// Tests if a `n` cells can be accomodated by the rate-limiter
    /// and updates rate limiter state iff they can be.
    ///
    /// As this method is an extension of GCRA (using multiplication),
    /// it is likely not as fast (and not as obviously "right") as the
    /// single-cell variant.
    fn test_n_and_update(&mut self, n: u32, t0: Instant) -> Result<Decision<Instant>> {
        let tat = self.tat.unwrap_or(t0);
        let tat = match n {
            0 => t0,
            1 => tat,
            _ => {
                let weight = self.t * (n - 1);
                if (weight + self.t) > self.tau {
                    // The bucket capacity can never accomodate this request
                    return Err(ErrorKind::InsufficientCapacity(n).into());
                }
                tat + weight
            }
        };

        if t0 < tat - self.tau {
            return Ok(Decision::No(tat));
        }
        let additional_weight = match n {
            0 => Duration::new(0, 0),
            1 => self.t,
            _ => self.t * n,
        };
        self.tat = Some(cmp::max(tat, t0) + additional_weight);
        Ok(Decision::Yes)
    }
}

/// Allows converting from a GCRA builder directly into a
/// GCRA decider. Same as
/// [the borrowed implementation](#impl-From<&'a Builder>), except for
/// owned `Builder`s.
/// # Example:
/// ```
/// use ratelimit_meter::{GCRA, Decider, Decision};
/// let mut gcra: GCRA = GCRA::for_capacity(50).unwrap().into();
/// assert_eq!(Decision::Yes, gcra.check().unwrap());
/// ```
impl From<Builder> for GCRA {
    fn from(b: Builder) -> Self {
        b.build()
    }
}

/// Allows converting a GCRA builder directly into a GCRA decider.
/// # Example:
/// ```
/// use ratelimit_meter::{GCRA, Decider, Decision};
/// let mut gcra: GCRA = GCRA::for_capacity(50).unwrap().cell_weight(2).unwrap().into();
/// assert_eq!(Decision::Yes, gcra.check().unwrap());
/// ```
impl<'a> From<&'a mut Builder> for GCRA {
    fn from(b: &'a mut Builder) -> Self {
        b.build()
    }
}
