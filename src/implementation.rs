use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use {InconsistentCapacity, NegativeMultiDecision, NonConformance};

/// The trait that implementations of the metered rate-limiter
/// interface have to implement. Users of this library should rely on
/// [Decider](trait.Decider.html) for the external interface instead.
pub trait DeciderImpl {
    /// The state of a single rate limiting bucket.
    type BucketState: Default + Send + Sync;

    /// The immutable parameters of the rate limiting bucket (e.g.,
    /// maximum capacity).
    type BucketParams: Send + Sync;

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
    ///
    /// This method is not meant to be called by users, see instead
    /// [the `Decider` trait](trait.Decider.html).
    fn test_n_and_update(
        state: &mut Self::BucketState,
        params: &Self::BucketParams,
        n: u32,
        at: Instant,
    ) -> Result<(), NegativeMultiDecision>;

    /// Tests if a single cell can be accommodated in the rate limiter
    /// at the instant `at` and updates the rate-limiter to account
    /// for the weight of the cell.
    ///
    /// This method is not meant to be called by users, see instead
    /// the [Decider trait](trait.Decider.html). The default
    /// implementation only calls
    /// [`test_n_and_update`](#test_n_and_update).
    fn test_and_update(
        state: &mut Self::BucketState,
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
