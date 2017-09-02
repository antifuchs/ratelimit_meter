use std::time::{Instant, Duration};
use std::cmp;

pub struct Limiter {
    capacity: u32,
    weight: u32,
    time_unit: Duration,
}

#[derive(PartialEq, Debug)]
/// A decision on a single cell from the metered rate-limiter.
pub enum Decision<T> {
    /// The cell is conforming, allow it through.
    Yes,

    /// The cell is non-conforming. A rate-limiting algorithm
    /// implementation may return additional information for the
    /// caller, e.g. a time when the cell was expected to arrive.
    No(T)
}

/// A builder pattern implementation that can construct deciders.
/// # Basic example
/// This example constructs a decider that considers every cell
/// compliant:
///
/// ```
/// # use ratelimit_meter::{Limiter, Decider, Allower};
///
/// let mut limiter = Limiter::new().build::<Allower>();
/// for _i in 1..3 {
///     println!("{:?}...", limiter.check());
/// }
/// ```
impl Limiter {
    /// Returns a default (useless) limiter with a capacity of zero, a
    /// cell weight of zero, and a time_unit of zero.
    pub fn new() -> Limiter {
        Limiter{
            capacity: 0,
            weight: 0,
            time_unit: Duration::from_secs(1),
        }
    }

    /// Sets the capacity of the limiter's "bucket" in elements per `time_unit`.
    pub fn capacity<'a>(&'a mut self, capacity: u32) -> &'a mut Limiter {
        self.capacity = capacity;
        self
    }

    /// Sets the "weight" of each cell being checked against the
    /// bucket. Each cell fills the bucket by this much.
    pub fn weight<'a>(&'a mut self, weight: u32) -> &'a mut Limiter {
        self.weight = weight;
        self
    }

    /// Sets the "unit of time" that the bucket drains at.
    pub fn time_unit<'a>(&'a mut self, time_unit: Duration) -> &'a mut Limiter {
        self.time_unit = time_unit;
        self
    }

    /// Builds and returns a concrete structure that implements the Decider trait.
    pub fn build<D>(&self) -> D where D: Decider {
        D::build_with(self)
    }
}

pub trait Decider {
    /// The (optional) type for additional information on negative
    /// decisions.
    type T;

    /// Tests if a single cell can be accomodated in the rate limiter
    /// at the instant `at` and updates the rate-limiter to account
    /// for the weight of the cell.
    fn test_and_update(&mut self, at: Instant) -> Decision<Self::T>;

    /// Converts the limiter builder into a concrete decider structure.
    fn build_with(l: &Limiter) -> Self;

    /// Tests if a single cell can be accomodated now. See `test_and_update`.
    fn check(&mut self) -> Decision<Self::T> {
        self.test_and_update(Instant::now())
    }
}

#[derive(Debug)]
/// Implements the virtual scheduling description of the Generic Cell
/// Rate Algorithm, attributed to ITU-T in recommendation I.371
/// Traffic control and congestion control in B-ISDN; from
/// [Wikipedia](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm).
///
/// # Example
/// In this example, we construct a rate-limiter with the GCR
/// algorithm that can accomodate 20 requests per second. This
/// translates to the GCRA parameters τ=1s, T=50ms.
///
/// ```
/// # use ratelimit_meter::{Limiter, Decider, GCRA, Decision};
/// # use std::time::{Instant, Duration};
/// let mut limiter = Limiter::new().capacity(20).weight(1).build::<GCRA>();
/// let now = Instant::now();
/// let ms = Duration::from_millis(1);
/// assert_eq!(Decision::Yes, limiter.test_and_update(now)); // the first cell is free
/// for i in 0..20 {
///     // Spam a lot:
///     assert_eq!(Decision::Yes, limiter.test_and_update(now));
/// }
/// // We have exceeded the bucket capacity:
/// assert_ne!(Decision::Yes, limiter.test_and_update(now));
///
/// // After a sufficient time period, cells are allowed again:
/// assert_eq!(Decision::Yes, limiter.test_and_update(now + ms*50));
pub struct GCRA {
    // The "weight" of a single packet in units of time.
    t: Duration,

    // The "capacity" of the bucket.
    tau: Duration,

    // The theoretical arrival time of the next packet.
    tat: Option<Instant>,
}

impl Decider for GCRA {
    /// In GCRA, negative decisions come with the time at which the
    /// next cell was expected to arrive; client code of GCRA can use
    /// this to decide what to do with the non-conforming cell.
    type T = Instant;

    fn test_and_update(&mut self, t0: Instant) -> Decision<Instant> {
        let tat = self.tat.unwrap_or(t0);
        if t0 < tat - self.tau {
            return Decision::No(tat)
        }
        self.tat = Some(cmp::max(tat, t0) + self.t);
        Decision::Yes
    }

    fn build_with(l: &Limiter) -> Self {
        GCRA {
            t: (l.time_unit / l.capacity) * l.weight,
            tau: l.time_unit,
            tat: None,
        }
    }
}

/// The most naive implementation of a rate-limiter ever: Always
/// allows every cell through.
pub struct Allower {}

impl Decider for Allower {
    /// Allower never returns a negative answer, so negative answers
    /// don't carry information.
    type T = ();

    /// Allows the cell through unconditionally.
    fn test_and_update(&mut self, _t0: Instant) -> Decision<()> {
        Decision::Yes
    }

    /// Builds the most useless rate-limiter in existence.
    fn build_with(_l: &Limiter) -> Self {
        Allower{}
    }
}

#[cfg(test)]
mod tests {
    use {GCRA, Limiter, Decider, Decision};
    use std::time::{Instant, Duration};

    #[test]
    fn accepts_first_cell() {
        let mut gcra = Limiter::new().capacity(5).weight(1).build::<GCRA>();
        assert_eq!(Decision::Yes, gcra.check());
    }
    #[test]
    fn rejects_too_many() {
        let mut gcra = Limiter::new().capacity(1).weight(1).build::<GCRA>();
        let now = Instant::now();
        gcra.test_and_update(now);
        gcra.test_and_update(now);
        assert_ne!(Decision::Yes, gcra.test_and_update(now));
    }
    #[test]
    fn allows_after_interval() {
        let mut gcra = Limiter::new().capacity(1).weight(1).build::<GCRA>();
        let now = Instant::now();
        let ms = Duration::from_millis(1);
        gcra.test_and_update(now);
        gcra.test_and_update(now+ms*1);
        gcra.test_and_update(now+ms*2);
        // should be ok again in 1s:
        let next = now + Duration::from_secs(1);
        assert_eq!(Decision::Yes, gcra.test_and_update(next));
    }
}
