use super::StateWrapper;
use lib::*;

#[derive(Default, PartialEq, Eq)]
pub struct SingleThreadedWrapper<T>
where
    T: fmt::Debug + Default + Clone + PartialEq + Eq,
{
    data: RefCell<T>,
}

impl<T> fmt::Debug for SingleThreadedWrapper<T>
where
    T: fmt::Debug + Default + Clone + PartialEq + Eq,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.data.borrow().fmt(f)
    }
}

impl<T> StateWrapper for SingleThreadedWrapper<T>
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
        let (decision, new_data) = f(&self.data.borrow());
        if let Some(new_data) = new_data {
            self.data.replace(new_data);
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
        self.data.borrow().clone()
    }
}
