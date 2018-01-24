use {Decider, DeciderImpl, MultiDecider, MultiDeciderImpl, NegativeMultiDecision, NonConformance};
use algorithms::InconsistentCapacity;

use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::time::{Duration, Instant};
use std::cmp;
use std::sync::Arc;

use crossbeam::epoch::{self, Atomic, Owned};

impl Decider for GCRA {}

/// This crate's GCRA implementation also allows checking multiple
/// cells at once, assuming that (counter the traffic-shaping
/// properties of GCRA) if a sufficiently long pause (`n*t`) has
/// occurred between cells, the algorithm can accommodate `n` cells.
///
/// As this assumption does not necessarily hold in all circumstances,
/// users of this trait on GCRA limiters should ensure that this is
/// ok.
impl MultiDecider for GCRA {}

#[derive(Debug, Clone)]
/// Implements the virtual scheduling description of the Generic Cell
/// Rate Algorithm, attributed to ITU-T in recommendation I.371
/// Traffic control and congestion control in B-ISDN; from
/// [Wikipedia](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm).
///
///
/// While algorithms like leaky-bucket rate limiters allow cells to be
/// distributed across time in any way, GCRA is a rate-limiting *and*
/// traffic-shaping algorithm. It mandates that a minimum amount of
/// time passes between cells being measured. For example, if your API
/// mandates that only 20 requests can be made per second, GCRA will
/// ensure that each request is at least 50ms apart from the previous
/// request. This makes GCRA suitable for shaping traffic in
/// networking and telecom equipment (it was initially made for
/// asynchronous transfer mode networks), or for outgoing workloads on
/// *consumers* of attention, e.g. distributing outgoing emails across
/// a day.
///
/// In a blatant side-stepping of the above traffic-shaping criteria,
/// this implementation of GCRA comes with an extension that allows
/// measuring multiple cells at once, assuming that if a pause of
/// `n*(the minimum time between cells)` has passed, we can allow a
/// single big batch of `n` cells through. This assumption may not be
/// correct for your application, but if you depend on GCRA's
/// traffic-shaping properties, it's better to not use the `_n`
/// suffixed check functions.
///
/// # Example
/// In this example, we construct a rate-limiter with the GCR
/// algorithm that can accommodate 20 cells per second. This translates
/// to the GCRA parameters τ=1s, T=50ms (that's 1s / 20 cells).
///
/// ```
/// # use ratelimit_meter::{Decider, GCRA};
/// # use std::time::{Instant, Duration};
/// let mut limiter = GCRA::for_capacity(20).unwrap().cell_weight(1).unwrap().build();
/// let now = Instant::now();
/// let ms = Duration::from_millis(1);
/// assert_eq!(Ok(()), limiter.check_at(now)); // the first cell is free
/// for i in 0..20 {
///     // Spam a lot:
///     assert!(limiter.check_at(now).is_ok(), "at {}", i);
/// }
/// // We have exceeded the bucket capacity:
/// assert!(limiter.check_at(now).is_err());
///
/// // After a sufficient time period, cells are allowed again:
/// assert_eq!(Ok(()), limiter.check_at(now + ms*50));
pub struct GCRA {
    // The "weight" of a single packet in units of time.
    t: Duration,

    // The "capacity" of the bucket.
    tau: Duration,

    // The theoretical arrival time of the next packet.
    tat: Arc<Atomic<Instant>>,
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
    pub fn cell_weight(&mut self, weight: u32) -> Result<&mut Builder, InconsistentCapacity> {
        if self.cell_weight > self.capacity {
            return Err(InconsistentCapacity {
                capacity: self.capacity,
                weight: weight,
            });
        }
        self.cell_weight = weight;
        Ok(self)
    }

    /// Sets the "unit of time" within which the bucket drains.
    ///
    /// The assumption is that in a period of `time_unit` (if no cells
    /// are being checked), the bucket is fully drained.
    pub fn per(&mut self, time_unit: Duration) -> &mut Builder {
        self.time_unit = time_unit;
        self
    }

    /// Builds a lock-free, threadsafe GCRA decider.
    pub fn build(&self) -> GCRA {
        GCRA {
            t: (self.time_unit / self.capacity) * self.cell_weight,
            tau: self.time_unit,
            tat: Arc::new(Atomic::null()),
        }
    }
}

impl GCRA {
    /// Constructs a builder object for a GCRA rate-limiter with the
    /// given capacity per second, at cell weight=1. See
    /// [`Builder`](struct.Builder.html) for options.
    pub fn for_capacity(capacity: u32) -> Result<Builder, InconsistentCapacity> {
        if capacity == 0 {
            return Err(InconsistentCapacity {
                capacity: capacity,
                weight: 0,
            });
        }
        Ok(Builder {
            capacity: capacity,
            cell_weight: 1,
            time_unit: Duration::from_secs(1),
        })
    }

    /// Constructs a GCRA rate-limiter with the parameters T (the
    /// minimum amount of time that single cells are spaced apart),
    /// tau (τ, the number of cells that fit into this buffer), and an
    /// optional t_at (the earliest instant that the rate-limiter
    /// would accept another cell).
    pub fn with_parameters(t: Duration, tau: Duration, tat: Option<Instant>) -> GCRA {
        match tat {
            Some(ts) => GCRA {
                t: t,
                tau: tau,
                tat: Arc::new(Atomic::new(ts)),
            },
            None => GCRA {
                t: t,
                tau: tau,
                tat: Arc::new(Atomic::null()),
            },
        }
    }
}

impl DeciderImpl for GCRA {
    /// Tests if a single cell can be accommodated by the
    /// rate-limiter. This is a threadsafe, lock-free implementation
    /// of the method described directly in the GCRA algorithm, and is
    /// the fastest.
    fn test_and_update(&mut self, t0: Instant) -> Result<(), NonConformance> {
        let guard = epoch::pin();
        loop {
            let tat_there = self.tat.load(Acquire, &guard);
            let tat = match tat_there {
                Some(sh) => **sh,
                None => t0,
            };

            if t0 < tat - self.tau {
                return Err(NonConformance::new(t0, tat - t0));
            }
            if self.tat
                .cas_and_ref(
                    tat_there,
                    Owned::new(cmp::max(tat, t0) + self.t),
                    Release,
                    &guard,
                )
                .is_ok()
            {
                return Ok(());
            }
        }
    }
}

impl MultiDeciderImpl for GCRA {
    /// Tests if `n` cells can be accommodated by the rate-limiter
    /// and updates rate limiter state iff they can be.
    ///
    /// As this method is an extension of GCRA (using multiplication),
    /// it is likely not as fast (and not as obviously "right") as the
    /// single-cell variant.
    fn test_n_and_update(&mut self, n: u32, t0: Instant) -> Result<(), NegativeMultiDecision> {
        let guard = epoch::pin();
        loop {
            let tat_there = self.tat.load(Acquire, &guard);
            let tat = match tat_there {
                Some(sh) => **sh,
                None => t0,
            };
            let tat = match n {
                0 => t0,
                1 => tat,
                _ => {
                    let weight = self.t * (n - 1);
                    if (weight + self.t) > self.tau {
                        // The bucket capacity can never accommodate this request
                        return Err(NegativeMultiDecision::InsufficientCapacity(n));
                    }
                    tat + weight
                }
            };
            if t0 < tat - self.tau {
                return Err(NegativeMultiDecision::BatchNonConforming(
                    n,
                    NonConformance::new(t0, tat - t0),
                ));
            }
            let additional_weight = match n {
                0 => Duration::new(0, 0),
                1 => self.t,
                _ => self.t * n,
            };
            if self.tat
                .cas_and_ref(
                    tat_there,
                    Owned::new(cmp::max(tat, t0) + additional_weight),
                    Release,
                    &guard,
                )
                .is_ok()
            {
                return Ok(());
            }
        }
    }
}

/// Allows converting from a GCRA builder directly into a
/// GCRA decider. Same as
/// [the borrowed implementation](#impl-From%3C%26%27a%20mut%20Builder%3E), except for
/// owned `Builder`s.
/// # Example:
/// ```
/// use ratelimit_meter::{GCRA, Decider, NonConformance};
/// let mut gcra: GCRA = GCRA::for_capacity(50).unwrap().into();
/// assert_eq!(Ok(()), gcra.check());
/// ```
impl From<Builder> for GCRA {
    fn from(b: Builder) -> Self {
        b.build()
    }
}

/// Allows converting a GCRA builder directly into a GCRA decider.
/// # Example:
/// ```
/// use ratelimit_meter::{GCRA, Decider, NonConformance};
/// let mut gcra: GCRA = GCRA::for_capacity(50).unwrap().cell_weight(2).unwrap().into();
/// assert_eq!(Ok(()), gcra.check());
/// ```
impl<'a> From<&'a mut Builder> for GCRA {
    fn from(b: &'a mut Builder) -> Self {
        b.build()
    }
}

/// Allows converting a GCRA into a tuple containing its T (the
/// minimum amount of time that single cells are spaced apart), tau
/// (τ, the number of cells that fit into this buffer), and an
/// optional `t_at` (the earliest instant that the rate-limiter would
/// accept another cell).
///
/// These parameters can be used with
/// [`.with_parameters`](#method.with_parameters) to persist and
/// construct a copy of the GCRA rate-limiter.
impl<'a> Into<(Duration, Duration, Option<Instant>)> for &'a GCRA {
    fn into(self) -> (Duration, Duration, Option<Instant>) {
        let guard = epoch::pin();
        let tat = match self.tat.load(Relaxed, &guard) {
            None => None,
            Some(sh) => Some(**sh),
        };
        (self.t, self.tau, tat)
    }
}

/// Allows converting the parameters returned from
/// [`Into<(Duration, Duration,
/// Option<Instant>)>`](#impl-Into<(Duration, Duration, Option<Instant>)>)
/// back into a GCRA.
impl From<(Duration, Duration, Option<Instant>)> for GCRA {
    fn from(params: (Duration, Duration, Option<Instant>)) -> GCRA {
        let (t, tau, tat) = params;
        GCRA::with_parameters(t, tau, tat)
    }
}
