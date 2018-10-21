//! An in-memory rate limiter that can make decisions for a single
//! situation.

use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use {Algorithm, InconsistentCapacity, NegativeMultiDecision, NonConformance};

/// An in-memory rate limiter that makes direct (un-keyed) rate-limiting
/// decisions. This kind of rate limiter can be used to regulate the
/// number of packets per connection.
#[derive(Debug, Clone)]
pub struct DirectRateLimiter<A: Algorithm> {
    algorithm: PhantomData<A>,
    state: A::BucketState,
    params: A::BucketParams,
}

impl<A> DirectRateLimiter<A>
where
    A: Algorithm,
{
    /// Construct a new Decider that allows `capacity` cells per time
    /// unit through.
    /// # Examples
    /// You can construct a GCRA decider like so:
    /// ```
    /// # use std::num::NonZeroU32;
    /// # use std::time::Duration;
    /// use ratelimit_meter::{DirectRateLimiter, GCRA};
    /// let _gcra = DirectRateLimiter::<GCRA>::new(NonZeroU32::new(100).unwrap(),
    ///                                          Duration::from_secs(5));
    /// ```
    ///
    /// and similarly, for a leaky bucket:
    /// ```
    /// # use std::num::NonZeroU32;
    /// # use std::time::Duration;
    /// use ratelimit_meter::{DirectRateLimiter, LeakyBucket};
    /// let _lb = DirectRateLimiter::<LeakyBucket>::new(NonZeroU32::new(100).unwrap(),
    ///                                               Duration::from_secs(5));
    /// ```
    pub fn new(capacity: NonZeroU32, per_time_unit: Duration) -> Self {
        DirectRateLimiter {
            algorithm: PhantomData,
            state: <A as Algorithm>::BucketState::default(),
            params: <A as Algorithm>::params_from_constructor(
                capacity,
                NonZeroU32::new(1).unwrap(),
                per_time_unit,
            ).unwrap(),
        }
    }

    /// Construct a new Decider that allows `capacity` cells per
    /// second.
    /// # Examples
    /// Constructing a GCRA decider that lets through 100 cells per second:
    /// ```
    /// # use std::num::NonZeroU32;
    /// # use std::time::Duration;
    /// use ratelimit_meter::{DirectRateLimiter, GCRA};
    /// let _gcra = DirectRateLimiter::<GCRA>::per_second(NonZeroU32::new(100).unwrap());
    /// ```
    ///
    /// and a leaky bucket:
    /// ```
    /// # use std::num::NonZeroU32;
    /// # use std::time::Duration;
    /// use ratelimit_meter::{DirectRateLimiter, LeakyBucket};
    /// let _gcra = DirectRateLimiter::<LeakyBucket>::per_second(NonZeroU32::new(100).unwrap());
    /// ```
    pub fn per_second(capacity: NonZeroU32) -> Self {
        Self::new(capacity, Duration::from_secs(1))
    }

    /// Return a builder that can be used to construct a Decider using
    /// the parameters passed to the Builder.
    pub fn build_with_capacity(capacity: NonZeroU32) -> Builder<A> {
        Builder {
            capacity,
            cell_weight: NonZeroU32::new(1).unwrap(),
            time_unit: Duration::from_secs(1),
            end_result: PhantomData,
        }
    }

    /// Tests if a single cell can be accommodated at
    /// `Instant::now()`. If it can be, `check` updates the `Decider`
    /// to account for the conforming cell and returns `Ok(())`.
    ///
    /// If the cell is non-conforming (i.e., it can't be accomodated
    /// at this time stamp), `check_at` returns `Err` with information
    /// about the earliest time at which a cell could be considered
    /// conforming (see [`NonConformance`](struct.NonConformance.html)).
    pub fn check(&mut self) -> Result<(), NonConformance> {
        <A as Algorithm>::test_and_update(&mut self.state, &self.params, Instant::now())
    }

    /// Tests if `n` cells can be accommodated at the current time
    /// stamp. If (and only if) all cells in the batch can be
    /// accomodated, the `MultiDecider` updates the internal state to
    /// account for all cells and returns `Ok(())`.
    ///
    /// If the entire batch of cells would not be conforming but the
    /// `MultiDecider` has the capacity to accomodate the cells at any
    /// point in time, `check_n_at` returns error
    /// [`NegativeMultiDecision::BatchNonConforming`](enum.NegativeMultiDecision.html#variant.BatchNonConforming),
    /// holding the number of cells and
    /// [`NonConformance`](struct.NonConformance.html) information.
    ///
    /// If `n` exceeds the bucket capacity, `check_n_at` returns
    /// [`NegativeMultiDecision::InsufficientCapacity`](enum.NegativeMultiDecision.html#variant.InsufficientCapacity),
    /// indicating that a batch of this many cells can never succeed.
    pub fn check_n(&mut self, n: u32) -> Result<(), NegativeMultiDecision> {
        <A as Algorithm>::test_n_and_update(&mut self.state, &self.params, n, Instant::now())
    }

    /// Tests whether a single cell can be accommodated at the given
    /// time stamp. See [`check`](#method.check).
    pub fn check_at(&mut self, at: Instant) -> Result<(), NonConformance> {
        <A as Algorithm>::test_and_update(&mut self.state, &self.params, at)
    }

    /// Tests if `n` cells can be accommodated at the given time
    /// (`Instant::now()`), using [`check_n`](#method.check_n)
    pub fn check_n_at(&mut self, n: u32, at: Instant) -> Result<(), NegativeMultiDecision> {
        <A as Algorithm>::test_n_and_update(&mut self.state, &self.params, n, at)
    }
}

/// An object that allows incrementally constructing Decider objects.
pub struct Builder<T>
where
    T: Algorithm + Sized,
{
    capacity: NonZeroU32,
    cell_weight: NonZeroU32,
    time_unit: Duration,
    end_result: PhantomData<T>,
}

impl<A> Builder<A>
where
    A: Algorithm + Sized,
{
    /// Sets the "weight" of each cell being checked against the
    /// bucket. Each cell fills the bucket by this much.
    pub fn cell_weight(
        &mut self,
        weight: NonZeroU32,
    ) -> Result<&mut Builder<A>, InconsistentCapacity> {
        if self.cell_weight > self.capacity {
            return Err(InconsistentCapacity {
                capacity: self.capacity,
                cell_weight: self.cell_weight,
            });
        }
        self.cell_weight = weight;
        Ok(self)
    }

    /// Sets the "unit of time" within which the bucket drains.
    ///
    /// The assumption is that in a period of `time_unit` (if no cells
    /// are being checked), the bucket is fully drained.
    pub fn per(&mut self, time_unit: Duration) -> &mut Builder<A> {
        self.time_unit = time_unit;
        self
    }

    /// Builds a decider of the specified type.
    pub fn build(&self) -> Result<DirectRateLimiter<A>, InconsistentCapacity> {
        Ok(DirectRateLimiter {
            algorithm: PhantomData,
            state: <A as Algorithm>::BucketState::default(),
            params: <A as Algorithm>::params_from_constructor(
                self.capacity,
                self.cell_weight,
                self.time_unit,
            )?,
        })
    }
}
