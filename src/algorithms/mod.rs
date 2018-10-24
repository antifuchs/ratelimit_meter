pub mod gcra;
pub mod leaky_bucket;

pub use self::gcra::*;
pub use self::leaky_bucket::*;

use evmap::ShallowCopy;
use failure::Fail;
use std::fmt;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use {InconsistentCapacity, NegativeMultiDecision};

/// The default rate limiting algorithm in this crate: The ["leaky
/// bucket"](leaky_bucket/struct.LeakyBucket.html).
///
/// The leaky bucket algorithm is fairly easy to understand and has
/// decent performance in most cases. If better threaded performance
/// is needed, this crate also offers the
/// [`GCRA`](gcra/struct.GCRA.html) algorithm.
pub type DefaultAlgorithm = LeakyBucket;

/// Provides additional information about non-conforming cells, most
/// importantly the earliest time until the next cell could be
/// considered conforming.
///
/// Since this does not account for effects like thundering herds,
/// users should always add random jitter to the times given.
pub trait NonConformance {
    /// Returns the earliest time at which a decision could be
    /// conforming (excluding conforming decisions made by the Decider
    /// that are made in the meantime).
    fn earliest_possible(&self) -> Instant;

    /// Returns the minimum amount of time from the time that the
    /// decision was made (relative to the `at` argument in a
    /// `Decider`'s `check_at` method) that must pass before a
    /// decision can be conforming. Since Durations can not be
    /// negative, a zero duration is returned if `from` is already
    /// after that duration.
    fn wait_time_from(&self, from: Instant) -> Duration;

    /// Returns the minimum amount of time (down to 0) that needs to
    /// pass from the current instant for the Decider to consider a
    /// cell conforming again.
    fn wait_time(&self) -> Duration {
        self.wait_time_from(Instant::now())
    }
}

/// The trait that implementations of metered rate-limiter algorithms
/// have to implement.
///
/// This is a stateless trait, which should allow for a variety of
/// rate-limiting schemes, like Redis or keyed vs. un-keyed
/// rate-limiting (one bucket per user vs. one bucket for the entire
/// API).
pub trait Algorithm {
    /// The state of a single rate limiting bucket.
    ///
    /// Every new rate limiting state is initialized as `Default`. The
    /// states must be safe to share across threads (this crate uses a
    /// `parking_lot` Mutex to allow that).
    type BucketState: RateLimitState<Self::BucketParams>
        + Default
        + Send
        + Sync
        + Eq
        + ShallowCopy
        + fmt::Debug;

    /// The immutable parameters of the rate limiting bucket (e.g.,
    /// maximum capacity). The bucket parameters are unique per rate
    /// limiter instance (there are currently no per-user/per-IP rate
    /// limiter parameters).
    type BucketParams: Send + Sync + fmt::Debug;

    /// The type returned when a rate limiting decision for a single
    /// cell is negative. Each rate limiting algorithm can decide to
    /// return the type that suits it best, but most algorithms'
    /// decisions also implement
    /// [`NonConformance`](trait.NonConformance.html), to ease
    /// handling of how long to wait.
    type NegativeDecision: PartialEq + Fail;

    /// Constructs a set of rate limiter parameters from the given
    /// parameters: `capacity` is the number of cells, weighhing
    /// `cell_weight`, to allow `per_time_unit`.
    fn params_from_constructor(
        capacity: NonZeroU32,
        cell_weight: NonZeroU32,
        per_time_unit: Duration,
    ) -> Result<Self::BucketParams, InconsistentCapacity>;

    /// Tests if `n` cells can be accommodated in the rate limiter at
    /// the instant `at` and updates the rate-limiter state to account
    /// for the weight of the cells and updates the ratelimiter state.
    ///
    /// The update is all or nothing: Unless all n cells can be
    /// accommodated, the state of the rate limiter will not be
    /// updated.
    fn test_n_and_update(
        state: &Self::BucketState,
        params: &Self::BucketParams,
        n: u32,
        at: Instant,
    ) -> Result<(), NegativeMultiDecision<Self::NegativeDecision>>;

    /// Tests if a single cell can be accommodated in the rate limiter
    /// at the instant `at` and updates the rate-limiter state to
    /// account for the weight of the cell.
    ///
    /// This method is provided by default, using the `n` test&update
    /// method.
    fn test_and_update(
        state: &Self::BucketState,
        params: &Self::BucketParams,
        at: Instant,
    ) -> Result<(), Self::NegativeDecision> {
        match Self::test_n_and_update(state, params, 1, at) {
            Ok(()) => Ok(()),
            Err(NegativeMultiDecision::BatchNonConforming(1, nc)) => Err(nc),
            Err(other) => panic!("bug: There's a non-conforming batch: {:?}", other),
        }
    }
}

/// Trait that all rate limit states have to implement around
/// housekeeping in keyed rate limiters.
pub trait RateLimitState<P> {
    /// Returns the last time instant that the state had any relevance
    /// (i.e. the rate limiter would behave exactly as if it was a new
    /// rate limiter after this time).
    ///
    /// If the state has not been touched for a given amount of time,
    /// the keyed rate limiter will expire it.
    ///
    /// # Thread safety
    /// This uses a bucket state snapshot to determine eligibility;
    /// race conditions can occur.
    fn last_touched(&self, params: &P) -> Instant;
}
