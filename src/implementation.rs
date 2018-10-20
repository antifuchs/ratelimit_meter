use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use {InconsistentCapacity, NegativeMultiDecision, NonConformance};

/// The trait that constructable implementations of the Decider trait
/// have to implement.
pub trait NewImpl: Sized {
    /// Check that the parameters are in order and return a new
    /// Decider instance.
    fn from_construction_parameters(
        capacity: NonZeroU32,
        cell_weight: NonZeroU32,
        per_time_unit: Duration,
    ) -> Result<Self, InconsistentCapacity>;
}

/// The trait that implementations of the metered rate-limiter
/// interface have to implement. Users of this library should rely on
/// [Decider](trait.Decider.html) for the external interface.
pub trait DeciderImpl: Send + Sync {
    /// Tests if a single cell can be accommodated in the rate limiter
    /// at the instant `at` and updates the rate-limiter to account
    /// for the weight of the cell.
    ///
    /// This method is not meant to be called by users, see instead
    /// the [Decider trait](trait.Decider.html). The default
    /// implementation only calls
    /// [`test_n_and_update`](#test_n_and_update).
    fn test_and_update(&mut self, at: Instant) -> Result<(), NonConformance>;
}

/// The trait that a metered rate-limiter interface has to implement
/// to support decisions on multiple cells in a batch.
pub trait MultiDeciderImpl: Send + Sync {
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
    fn test_n_and_update(&mut self, n: u32, at: Instant) -> Result<(), NegativeMultiDecision>;
}

/// A trait that some implementations can opt into, to get a default
/// implementation of the `DeciderImpl` trait.
pub trait ImpliedDeciderImpl: MultiDeciderImpl {}

/// A default implementation of the Decider trait, using the
/// `MultiDeciderImpl` trait's methods with `n=1`.
impl<T> DeciderImpl for T
where
    T: ImpliedDeciderImpl,
{
    /// Default implementation of
    /// [trait.DeciderImpl.html#tymethod.test_and_update]`test_and_update`,
    /// calling [`test_n_and_update`](tymethod.test_n_and_update) with
    /// `n=1`.
    fn test_and_update(&mut self, at: Instant) -> Result<(), NonConformance> {
        match self.test_n_and_update(1, at) {
            Ok(()) => Ok(()),
            Err(NegativeMultiDecision::BatchNonConforming(1, nc)) => Err(nc),
            Err(other) => panic!("bug: There's a non-conforming batch: {:?}", other),
        }
    }
}
