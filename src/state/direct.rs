//! An in-memory rate limiter that can make decisions for a single
//! situation.

use crate::lib::*;

use crate::{
    algorithms::{Algorithm, DefaultAlgorithm},
    clock, InconsistentCapacity, NegativeMultiDecision,
};

/// An in-memory rate limiter that makes direct (un-keyed)
/// rate-limiting decisions. Direct rate limiters can be used to
/// e.g. regulate the transmission of packets on a single connection,
/// or to ensure that an API client stays within a server's rate
/// limit.
#[derive(Debug, Clone)]
pub struct DirectRateLimiter<
    A: Algorithm<C::Instant> = DefaultAlgorithm,
    C: clock::Clock = clock::DefaultClock,
> {
    state: A::BucketState,
    algorithm: A,
    clock: C,
}

impl<A, C> DirectRateLimiter<A, C>
where
    C: clock::Clock,
    A: Algorithm<C::Instant>,
{
    /// Construct a new rate limiter that allows `capacity` cells per
    /// time unit through.
    /// # Examples
    /// You can construct a GCRA rate limiter like so:
    /// ```
    /// # use std::num::NonZeroU32;
    /// # use std::time::Duration;
    /// use ratelimit_meter::{DirectRateLimiter, GCRA};
    /// # #[macro_use] extern crate nonzero_ext;
    /// # extern crate ratelimit_meter;
    /// # fn main () {
    /// let _gcra = DirectRateLimiter::<GCRA>::new(nonzero!(100u32), Duration::from_secs(5));
    /// # }
    /// ```
    ///
    /// and similarly, for a leaky bucket:
    /// ```
    /// # use std::time::Duration;
    /// use ratelimit_meter::{DirectRateLimiter, LeakyBucket};
    /// # #[macro_use] extern crate nonzero_ext;
    /// # extern crate ratelimit_meter;
    /// # fn main () {
    /// let _lb = DirectRateLimiter::<LeakyBucket>::new(nonzero!(100u32), Duration::from_secs(5));
    /// # }
    /// ```
    pub fn new(capacity: NonZeroU32, per_time_unit: Duration) -> Self {
        DirectRateLimiter {
            state: <A as Algorithm<C::Instant>>::BucketState::default(),
            algorithm: <A as Algorithm<C::Instant>>::construct(
                capacity,
                nonzero!(1u32),
                per_time_unit,
            )
            .unwrap(),
            clock: Default::default(),
        }
    }

    /// Construct a new rate limiter that allows `capacity` cells per
    /// second.
    /// # Examples
    /// Constructing a GCRA rate limiter that lets through 100 cells per second:
    /// ```
    /// # use std::time::Duration;
    /// use ratelimit_meter::{DirectRateLimiter, GCRA};
    /// # #[macro_use] extern crate nonzero_ext;
    /// # extern crate ratelimit_meter;
    /// # fn main () {
    /// let _gcra = DirectRateLimiter::<GCRA>::per_second(nonzero!(100u32));
    /// # }
    /// ```
    ///
    /// and a leaky bucket:
    /// ```
    /// # use std::time::Duration;
    /// use ratelimit_meter::{DirectRateLimiter, LeakyBucket};
    /// # #[macro_use] extern crate nonzero_ext;
    /// # extern crate ratelimit_meter;
    /// # fn main () {
    /// let _gcra = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(100u32));
    /// # }
    /// ```
    pub fn per_second(capacity: NonZeroU32) -> Self {
        Self::new(capacity, Duration::from_secs(1))
    }

    /// Return a builder that can be used to construct a rate limiter using
    /// the parameters passed to the Builder.
    pub fn build_with_capacity(capacity: NonZeroU32) -> Builder<C, A> {
        Builder {
            capacity,
            cell_weight: nonzero!(1u32),
            time_unit: Duration::from_secs(1),
            end_result: PhantomData,
            clock: Default::default(),
        }
    }

    /// Tests whether a single cell can be accommodated at the given
    /// time stamp. See [`check`](#method.check).
    pub fn check_at(
        &mut self,
        at: C::Instant,
    ) -> Result<(), <A as Algorithm<C::Instant>>::NegativeDecision> {
        self.algorithm.test_and_update(&self.state, at)
    }

    /// Tests if `n` cells can be accommodated at the given time
    /// (`Instant::now()`), using [`check_n`](#method.check_n)
    pub fn check_n_at(
        &mut self,
        n: u32,
        at: C::Instant,
    ) -> Result<(), NegativeMultiDecision<<A as Algorithm<C::Instant>>::NegativeDecision>> {
        self.algorithm.test_n_and_update(&self.state, n, at)
    }

    /// Tests if a single cell can be accommodated at the clock's
    /// current reading. If it can be, `check` updates the rate
    /// limiter state to account for the conforming cell and returns
    /// `Ok(())`.
    ///
    /// If the cell is non-conforming (i.e., it can't be accomodated
    /// at this time stamp), `check_at` returns `Err` with information
    /// about the earliest time at which a cell could be considered
    /// conforming.
    pub fn check(&mut self) -> Result<(), <A as Algorithm<C::Instant>>::NegativeDecision> {
        self.algorithm
            .test_and_update(&self.state, self.clock.now())
    }

    /// Tests if `n` cells can be accommodated at the clock's current
    /// reading. If (and only if) all cells in the batch can be
    /// accomodated, the `MultiDecider` updates the internal state to
    /// account for all cells and returns `Ok(())`.
    ///
    /// If the entire batch of cells would not be conforming but the
    /// rate limiter has the capacity to accomodate the cells at any
    /// point in time, `check_n_at` returns error
    /// [`NegativeMultiDecision::BatchNonConforming`](../../enum.NegativeMultiDecision.html#variant.BatchNonConforming),
    /// holding the number of cells the rate limiter's negative
    /// outcome result.
    ///
    /// If `n` exceeds the bucket capacity, `check_n_at` returns
    /// [`NegativeMultiDecision::InsufficientCapacity`](../../enum.NegativeMultiDecision.html#variant.InsufficientCapacity),
    /// indicating that a batch of this many cells can never succeed.
    pub fn check_n(
        &mut self,
        n: u32,
    ) -> Result<(), NegativeMultiDecision<<A as Algorithm<C::Instant>>::NegativeDecision>> {
        self.algorithm
            .test_n_and_update(&self.state, n, self.clock.now())
    }
}

/// An object that allows incrementally constructing rate Limiter
/// objects.
pub struct Builder<C, A>
where
    C: clock::Clock,
    A: Algorithm<C::Instant> + Sized,
{
    capacity: NonZeroU32,
    cell_weight: NonZeroU32,
    time_unit: Duration,
    end_result: PhantomData<A>,
    clock: C,
}

impl<C, A> Builder<C, A>
where
    C: clock::Clock,
    A: Algorithm<C::Instant> + Sized,
{
    /// Sets the "weight" of each cell being checked against the
    /// bucket. Each cell fills the bucket by this much.
    pub fn cell_weight(
        &mut self,
        weight: NonZeroU32,
    ) -> Result<&mut Builder<C, A>, InconsistentCapacity> {
        if self.cell_weight > self.capacity {
            return Err(InconsistentCapacity::new(self.capacity, self.cell_weight));
        }
        self.cell_weight = weight;
        Ok(self)
    }

    /// Sets the "unit of time" within which the bucket drains.
    ///
    /// The assumption is that in a period of `time_unit` (if no cells
    /// are being checked), the bucket is fully drained.
    pub fn per(&mut self, time_unit: Duration) -> &mut Builder<C, A> {
        self.time_unit = time_unit;
        self
    }

    /// Sets the clock used by the bucket.
    pub fn using_clock(&mut self, clock: C) -> &mut Builder<C, A> {
        self.clock = clock;
        self
    }

    /// Builds a rate limiter of the specified type.
    pub fn build(&self) -> Result<DirectRateLimiter<A, C>, InconsistentCapacity> {
        Ok(DirectRateLimiter {
            state: <A as Algorithm<C::Instant>>::BucketState::default(),
            algorithm: <A as Algorithm<C::Instant>>::construct(
                self.capacity,
                self.cell_weight,
                self.time_unit,
            )?,
            clock: self.clock.clone(),
        })
    }
}
