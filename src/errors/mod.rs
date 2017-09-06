use std::sync::{MutexGuard, PoisonError};

error_chain! {
    errors {
        /// Returned when attempting to acquire a "poisoned" mutex.
        ThreadingError {
            display("mutex is poisoned")
        }

        /// Returned when an internal inconsistency is detected
        /// (e.g. a bucket's capacity is too small to accomodate a
        /// single cell)
        CapacityError {
            display("bucket capacity is wrong")
        }
    }
}

/// This must discard the original PoisonError, as `error_chain` does
/// not currently support parameterizing `foreign_link`s with types
/// the way we would need to.
impl<'a, T> ::std::convert::From<PoisonError<MutexGuard<'a, T>>> for Error
{
    fn from(_err: PoisonError<MutexGuard<'a, T>>) -> Self {
        ErrorKind::ThreadingError.into()
    }
}
