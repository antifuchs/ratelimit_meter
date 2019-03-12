use lib::*;

#[cfg(feature = "std")]
use parking_lot::Mutex;

#[cfg(not(feature = "std"))]
use spin::Mutex;

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

/// The type used to wrap the rate-limiter state.
pub type Wrapper<T> = ThreadsafeWrapper<T>;

#[derive(Clone)]
/// Wraps the atomic operations on a Decider's state in a threadsafe
/// fashion.
pub struct ThreadsafeWrapper<T>
where
    T: fmt::Debug + Default + Clone + PartialEq + Eq,
{
    data: Arc<Mutex<T>>,
}

impl<T> Default for ThreadsafeWrapper<T>
where
    T: fmt::Debug + Default + Clone + PartialEq + Eq,
{
    fn default() -> Self {
        ThreadsafeWrapper {
            data: Arc::new(Mutex::new(T::default())),
        }
    }
}

impl<T> PartialEq<Self> for ThreadsafeWrapper<T>
where
    T: fmt::Debug + Default + Clone + PartialEq + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        let mine = self.data.lock();
        let other = other.data.lock();
        *other == *mine
    }
}

impl<T> Eq for ThreadsafeWrapper<T> where T: fmt::Debug + Default + Clone + PartialEq + Eq {}

impl<T> fmt::Debug for ThreadsafeWrapper<T>
where
    T: fmt::Debug + Default + Clone + PartialEq + Eq,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let data = self.data.lock();
        data.fmt(f)
    }
}

#[cfg(feature = "std")]
mod std {
    use super::*;
    use evmap::ShallowCopy;

    impl<T> ShallowCopy for ThreadsafeWrapper<T>
    where
        T: fmt::Debug + Default + Clone + PartialEq + Eq,
    {
        unsafe fn shallow_copy(&mut self) -> Self {
            ThreadsafeWrapper {
                data: self.data.shallow_copy(),
            }
        }
    }
}

impl<T> StateWrapper for ThreadsafeWrapper<T>
where
    T: fmt::Debug + Default + Clone + PartialEq + Eq,
{
    type Wrapped = T;

    #[inline]
    /// Wraps retrieving a bucket's data, calls a function to make a
    /// decision and return a new state, and then tries to set the
    /// state on the bucket.
    ///
    /// This function can loop and call the decision closure again if
    /// the bucket state couldn't be set.
    ///
    /// # Panics
    /// Panics if an error occurs in acquiring any locks.
    fn measure_and_replace<F, E>(&self, f: F) -> Result<(), E>
    where
        F: Fn(&T) -> (Result<(), E>, Option<T>),
    {
        let mut data = self.data.lock();
        let (decision, new_data) = f(&*data);
        if let Some(new_data) = new_data {
            *data = new_data;
        }
        decision
    }

    /// Retrieves and returns a snapshot of the bucket state. This
    /// isn't thread safe, but can be used to restore an old copy of
    /// the bucket if necessary.
    ///
    /// # Thread safety
    /// This function operates threadsafely, but you're literally
    /// taking a copy of data that will change. Relying on the data
    /// that is returned *will* race.
    fn snapshot(&self) -> T {
        let data = self.data.lock();
        data.clone()
    }
}
