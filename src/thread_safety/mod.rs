use lib::*;

/// A trait providing interior mutability for rate-limiter
/// states. This can be backed by a Cell (for single-threaded
/// operation), or an `Arc`-wrapped `Mutex` for operations where the
/// rate limiter states are expected to be `Sync`.
pub trait StateWrapper: Default {
    /// The rate limiter state type.
    type Wrapped: Default;

    /// Wraps retrieving a bucket's data, calls a function to make a
    /// decision and return a new state, and then tries to set the
    /// state on the bucket.
    ///
    /// This function is allowed to loop and call the decision closure
    /// again if the bucket state couldn't be set.
    ///
    /// # Panics
    /// Panics if an error occurs in acquiring any locks.
    fn measure_and_replace<F, E>(&self, f: F) -> Result<(), E>
    where
        F: Fn(&Self::Wrapped) -> (Result<(), E>, Option<Self::Wrapped>);

    /// Retrieves and returns a snapshot of the bucket state. This
    /// isn't thread safe, but can be used to restore an old copy of
    /// the bucket if necessary.
    ///
    /// # Thread safety
    /// This function operates threadsafely, but you're literally
    /// taking a copy of data that will change. Relying on the data
    /// that is returned *will* race.
    fn snapshot(&self) -> Self::Wrapped;
}

#[cfg(feature = "sync")]
mod sync;

#[cfg(feature = "sync")]
pub use self::sync::*;

#[cfg(feature = "sync")]
/// The type used to wrap the rate-limiter state.
pub type Wrapper<T> = ThreadsafeWrapper<T>;

#[cfg(not(feature = "sync"))]
mod no_sync;

#[cfg(not(feature = "sync"))]
pub use self::no_sync::*;

#[cfg(not(feature = "sync"))]
/// The type used to wrap the rate-limiter state.
pub type Wrapper<T> = SingleThreadedWrapper<T>;
