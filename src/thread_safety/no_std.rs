use lib::*;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub(crate) struct ThreadsafeWrapper<T>
where
    T: fmt::Debug + Default + Clone + PartialEq + Eq,
{
    data: Option<T>,
}

impl<T> ThreadsafeWrapper<T>
where
    T: fmt::Debug + Default + Clone + PartialEq + Eq,
{
    /// Wraps retrieving a bucket's data, calls a function to make a
    /// decision and return a new state, and then tries to set the
    /// state on the bucket.
    ///
    /// This function can loop and call the decision closure again if
    /// the bucket state couldn't be set.
    #[inline]
    pub(crate) fn measure_and_replace<F, E>(&self, f: F) -> Result<(), E>
    where
        F: Fn(&T) -> (Result<(), E>, Option<T>),
    {
        let data = self.data.unwrap_or_else(Default::default);
        let (decision, new_data) = f(&data);
        if let Some(new_data) = new_data {
            self.data.replace(new_data);
        }
        decision
    }

    /// Retrieves and returns a snapshot of the single-threaded bucket
    /// state. This can be used to restore an old copy of the bucket
    /// if necessary.
    pub(crate) fn snapshot(&self) -> T {
        self.data.unwrap_or_else(Default::default).clone()
    }
}
