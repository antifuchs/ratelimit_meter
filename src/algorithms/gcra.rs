//! The Generic Cell Rate Algorithm

use thread_safety::ThreadsafeWrapper;
use {
    algorithms::{Algorithm, NonConformance, RateLimitState},
    instant::Point,
    InconsistentCapacity, NegativeMultiDecision,
};

use evmap::ShallowCopy;

use std::cmp;
use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};

#[derive(Debug, Eq, PartialEq, Clone)]
struct Tat<P: Point>(Option<P>);

impl<P: Point> Default for Tat<P> {
    fn default() -> Self {
        Tat(None)
    }
}

/// The GCRA's state about a single rate limiting history.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct State<P: Point>(ThreadsafeWrapper<Tat<P>>);

impl<P: Point> Default for State<P> {
    fn default() -> Self {
        State(Default::default())
    }
}

impl<P: Point> ShallowCopy for State<P> {
    unsafe fn shallow_copy(&mut self) -> Self {
        State(self.0.shallow_copy())
    }
}

impl<P: Point> RateLimitState<GCRA<P>, P> for State<P> {
    fn last_touched(&self, params: &GCRA<P>) -> P {
        let data = self.0.snapshot();
        data.0.unwrap_or_else(P::now) + params.tau
    }
}

/// Returned in case of a negative rate-limiting decision. Indicates
/// the earliest instant that a cell might get accepted again.
///
/// To avoid thundering herd effects, client code should always add a
/// random amount of jitter to wait time estimates.
#[derive(Debug, PartialEq)]
pub struct NotUntil<P: Point>(P);

impl<P: Point> fmt::Display for NotUntil<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "rate-limited until {:?}", self.0)
    }
}

impl<P: Point> NonConformance<P> for NotUntil<P> {
    #[inline]
    fn earliest_possible(&self) -> P {
        self.0
    }
}

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
/// # A note about batch decisions
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
/// # use ratelimit_meter::{DirectRateLimiter, GCRA};
/// # use std::num::NonZeroU32;
/// # use std::time::{Instant, Duration};
/// # #[macro_use] extern crate nonzero_ext;
/// # extern crate ratelimit_meter;
/// # fn main () {
/// let mut limiter = DirectRateLimiter::<GCRA>::per_second(nonzero!(20u32));
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
/// # }
#[derive(Debug, Clone)]
pub struct GCRA<P: Point = Instant> {
    // The "weight" of a single packet in units of time.
    t: Duration,

    // The "capacity" of the bucket.
    tau: Duration,

    point: PhantomData<P>,
}

impl<P: Point> Algorithm<P> for GCRA<P> {
    type BucketState = State<P>;

    type NegativeDecision = NotUntil<P>;

    fn construct(
        capacity: NonZeroU32,
        cell_weight: NonZeroU32,
        per_time_unit: Duration,
    ) -> Result<Self, InconsistentCapacity> {
        if capacity < cell_weight {
            return Err(InconsistentCapacity::new(capacity, cell_weight));
        }
        Ok(GCRA {
            t: (per_time_unit / capacity.get()) * cell_weight.get(),
            tau: per_time_unit,
            point: PhantomData,
        })
    }

    /// Tests if a single cell can be accommodated by the
    /// rate-limiter. This is a threadsafe implementation of the
    /// method described directly in the GCRA algorithm.
    fn test_and_update(
        &self,
        state: &Self::BucketState,
        t0: P,
    ) -> Result<(), Self::NegativeDecision> {
        let tau = self.tau;
        let t = self.t;
        state.0.measure_and_replace(|tat| {
            let tat = tat.0.unwrap_or(t0);
            if t0 < tat - tau {
                (Err(NotUntil(tat)), None)
            } else {
                (Ok(()), Some(Tat(Some(cmp::max(tat, t0) + t))))
            }
        })
    }

    /// Tests if `n` cells can be accommodated by the rate-limiter
    /// and updates rate limiter state iff they can be.
    ///
    /// As this method is an extension of GCRA (using multiplication),
    /// it is likely not as fast (and not as obviously "right") as the
    /// single-cell variant.
    fn test_n_and_update(
        &self,
        state: &Self::BucketState,
        n: u32,
        t0: P,
    ) -> Result<(), NegativeMultiDecision<Self::NegativeDecision>> {
        let tau = self.tau;
        let t = self.t;
        state.0.measure_and_replace(|tat| {
            let tat = tat.0.unwrap_or(t0);
            let tat = match n {
                0 => t0,
                1 => tat,
                _ => {
                    let weight = t * (n - 1);
                    if (weight + t) > tau {
                        // The bucket capacity can never accommodate this request
                        return (Err(NegativeMultiDecision::InsufficientCapacity(n)), None);
                    }
                    tat + weight
                }
            };

            let additional_weight = match n {
                0 => Duration::new(0, 0),
                1 => t,
                _ => t * n,
            };
            if t0 < tat - tau {
                (
                    Err(NegativeMultiDecision::BatchNonConforming(n, NotUntil(tat))),
                    None,
                )
            } else {
                (
                    Ok(()),
                    Some(Tat(Some(cmp::max(tat, t0) + additional_weight))),
                )
            }
        })
    }
}
