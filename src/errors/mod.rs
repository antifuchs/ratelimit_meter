use std::sync::{MutexGuard, PoisonError};

error_chain! {
    errors {
        /// Returned when attempting to acquire a "poisoned"
        /// mutex. Due to limitations in error_chain, this error kind
        /// does not contain the original mutex guard or the piece of
        /// data that was meant to be locked.
        ThreadingError {
            display("mutex is poisoned")
        }

        /// Returned when constructing a bucket is impossible: when
        /// the capacity is 0, or the weight of a single cell is
        /// larger than the bucket's capacity.
        InconsistentCapacity(capacity: u32, weight: u32) {
            display("bucket capacity {} is not enough to accomodate even a single cell with weight {}",
                    capacity, weight)
        }

        /// Returned when trying to check more cells than the bucket
        /// can accomodate, given its capacity and per-cell weight.
        InsufficientCapacity(n: u32) {
            display("bucket does not have enough capacity to accomodate {} cells", n)
        }
    }
}

/// This must discard the original PoisonError, as `error_chain` does
/// not currently support parameterizing `foreign_link`s with types
/// the way we would need to.
impl<'a, T> ::std::convert::From<PoisonError<MutexGuard<'a, T>>> for Error {
    fn from(_err: PoisonError<MutexGuard<'a, T>>) -> Self {
        ErrorKind::ThreadingError.into()
    }
}
