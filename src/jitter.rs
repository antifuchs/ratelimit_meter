//! Random additional wait time.
//!
//! With Jitter, rate limiters will return a time estimate that is artificially inflated
//! by a random duration (capped at a maximum, with an optional minimum). This helps avoid
//! thundering herds when many concurrent rate limit requests are being made.

use crate::lib::*;
use crate::{clock, NegativeMultiDecision, NonConformance};
use once_cell::sync::OnceCell;
use std::default::Default;

/// A time interval specification that gets added to the wait time returned by the rate limiter's
/// non-conformance results.
#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub struct Jitter {
    min: Duration,
    interval: Duration,
}

impl Jitter {
    /// Constructs a new Jitter interval, waiting at most a duration of `max`.
    pub fn up_to(max: Duration) -> Jitter {
        Jitter {
            min: Duration::new(0, 0),
            interval: max,
        }
    }

    /// Constructs a new Jitter interval, waiting at least `min` and at most `min+interval`.
    pub fn new(min: Duration, interval: Duration) -> Jitter {
        Jitter { min, interval }
    }

    /// Returns a random amount of jitter within the configured interval.
    pub(crate) fn get(&self) -> Duration {
        let range = rand::random::<f32>();
        self.min + self.interval.mul_f32(range)
    }
}

#[derive(PartialEq, Debug)]
/// A non-conforming result that has had random jitter applied.
pub struct NonConformanceWithJitter<NC: NonConformance<P>, P: clock::Reference> {
    inner: NC,
    jitter: Jitter,
    additional: OnceCell<Duration>,
    phantom: PhantomData<P>,
}

impl<NC: NonConformance<P>, P: clock::Reference> NonConformanceWithJitter<NC, P> {
    pub(crate) fn new(inner: NC, jitter: Jitter) -> NonConformanceWithJitter<NC, P> {
        NonConformanceWithJitter {
            inner,
            jitter,
            additional: OnceCell::default(),
            phantom: PhantomData,
        }
    }

    fn additional_wait_time(&self) -> Duration {
        *(self.additional.get_or_init(|| self.jitter.get()))
    }
}

impl<NC: NonConformance<P>, P: clock::Reference> NonConformance<P>
    for NonConformanceWithJitter<NC, P>
{
    fn earliest_possible(&self) -> P {
        self.inner.earliest_possible() + self.additional_wait_time()
    }

    fn wait_time_from(&self, from: P) -> Duration {
        self.inner.wait_time_from(from) + self.additional_wait_time()
    }
}

impl<NC: NonConformance<P>, P: clock::Reference> fmt::Display for NonConformanceWithJitter<NC, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "rate-limited until {:?}", self.earliest_possible())
    }
}

/// Apply random jitter to a negative single-cell decision.
///
/// See [`Jitter`].
pub trait JitterResultExt<P: clock::Reference, NC: NonConformance<P>> {
    /// Adjusts a potential negative decision so that it includes random jitter.
    fn jitter(self, jitter: &Jitter) -> Result<(), NonConformanceWithJitter<NC, P>>;
}

/// Blanket implementation for applying jitter to any single-cell negative decision.
impl<P: clock::Reference, NC: NonConformance<P>> JitterResultExt<P, NC> for Result<(), NC> {
    fn jitter(self, jitter: &Jitter) -> Result<(), NonConformanceWithJitter<NC, P>> {
        self.map_err(|nc| NonConformanceWithJitter::new(nc, *jitter))
    }
}

/// Apply random jitter to a negative multi-cell decision.
///
/// See [`Jitter`].
pub trait JitterMultiResultExt<P: clock::Reference, NC: NonConformance<P>> {
    /// Adjusts a potential negative decision so that it includes random jitter.
    fn jitter(
        self,
        jitter: &Jitter,
    ) -> Result<(), NegativeMultiDecision<NonConformanceWithJitter<NC, P>>>;
}

/// Blanket implementation for applying jitter to any multi-cell negative decision.
impl<P: clock::Reference, NC: NonConformance<P>> JitterMultiResultExt<P, NC>
    for Result<(), NegativeMultiDecision<NC>>
{
    fn jitter(
        self,
        jitter: &Jitter,
    ) -> Result<(), NegativeMultiDecision<NonConformanceWithJitter<NC, P>>> {
        self.map_err(|decision| match decision {
            NegativeMultiDecision::BatchNonConforming(n, nc) => {
                NegativeMultiDecision::BatchNonConforming(
                    n,
                    NonConformanceWithJitter::new(nc, *jitter),
                )
            }
            NegativeMultiDecision::InsufficientCapacity(n) => {
                NegativeMultiDecision::InsufficientCapacity(n)
            }
        })
    }
}
