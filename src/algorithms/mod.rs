pub mod gcra;
pub mod leaky_bucket;

pub use self::gcra::*;
pub use self::leaky_bucket::*;

use evmap::ShallowCopy;
use std::fmt;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use {InconsistentCapacity, NegativeMultiDecision, NonConformance};

/// The trait that implementations of metered rate-limiter algorithms
/// have to implement.
///
/// This is a stateless trait, which should allow for a variety of
/// rate-limiting schemes, like Redis or keyed vs. un-keyed
/// rate-limiting (one bucket per user vs. one bucket for the entire
/// API).
pub trait Algorithm {
    /// The state of a single rate limiting bucket.
    type BucketState: Default
        + Send
        + Sync
        + Eq
        + ShallowCopy
        + fmt::Debug
        + RateLimitState<Self::BucketParams>;

    /// The immutable parameters of the rate limiting bucket (e.g.,
    /// maximum capacity).
    type BucketParams: Send + Sync + fmt::Debug;

    fn params_from_constructor(
        capacity: NonZeroU32,
        cell_weight: NonZeroU32,
        per_time_unit: Duration,
    ) -> Result<Self::BucketParams, InconsistentCapacity>;

    /// Tests if `n` cells can be accommodated in the rate limiter at
    /// the instant `at` and updates the rate-limiter to account for
    /// the weight of the cells and updates the ratelimiter state.
    ///
    /// The update is all or nothing: Unless all n cells can be
    /// accommodated, the state of the rate limiter will not be
    /// updated.
    fn test_n_and_update(
        state: &Self::BucketState,
        params: &Self::BucketParams,
        n: u32,
        at: Instant,
    ) -> Result<(), NegativeMultiDecision>;

    /// Tests if a single cell can be accommodated in the rate limiter
    /// at the instant `at` and updates the rate-limiter to account
    /// for the weight of the cell.
    ///
    /// This method is provided by default, using the `n` test&update
    /// method.
    fn test_and_update(
        state: &Self::BucketState,
        params: &Self::BucketParams,
        at: Instant,
    ) -> Result<(), NonConformance> {
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
