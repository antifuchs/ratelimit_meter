use std::sync::{MutexGuard, PoisonError};
use std::time::Duration;

error_chain! {
    errors {
        /// Returned if the rate limiter implementation requires a
        /// capacity for the "bucket".
        CapacityRequired {
            display("a capacity is required")
        }

        /// Returned if the rate limiter implementation requires a
        /// weight per unit of work.
        WeightRequired {
            display("a weight is required")
        }

        /// Returned if the drainage time unit is wrong (e.g. it's negative).
        InvalidTimeUnit(u: Duration) {
            display("time unit {:?} is invalid", u)
        }

        /// Returned when attempting to acquire a "poisoned" mutex.
        ThreadingError {
            display("mutex is poisoned")
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
