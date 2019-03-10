use lib::*;

/// The default time representation in use by rate limiters. To
/// override it, pass a different `P` type argument to the algorithm
/// and rate limiter bucket.
///
/// ## When using `std`
/// The default time source is `Instant` when using std.
///
/// ## When using `no_std`
/// In situations where `std` is not available, the fallback default
/// time source is Duration. It only allows comparisons to a relative,
/// fixed, point in time. Users are expected to determine that point
/// in time and stick to it.
#[cfg(feature = "std")]
pub type TimeSource = Instant;
#[cfg(not(feature = "std"))]
pub type TimeSource = Duration;

/// A point in time that is used as a reference for measuring a rate
/// limit. On the clock, it has meaning only relative to some other point in time.
///
/// When using `no_std`, users of this crate are expected to provide
/// an impl of `Relative` that corresponds to their system's time source.
pub trait Relative:
    Sized
    + Sub<Duration, Output = Self>
    + Add<Duration, Output = Self>
    + PartialEq
    + Eq
    + Ord
    + Copy
    + Clone
    + Send
    + Sync
    + Debug
{
    /// Returns the amount of time elapsed from an earlier point in time.
    fn duration_since(&self, earlier: Self) -> Duration;
}

/// A point in time as given by a source of time. It is assumed to be
/// monotonically moving forward.
pub trait Absolute: Relative {
    /// Returns the current moment in time, as given by the time
    /// source.
    fn now() -> Self;
}

#[cfg(feature = "std")]
mod std {
    use std::time::{Duration, Instant};

    impl super::Relative for Instant {
        fn duration_since(&self, earlier: Self) -> Duration {
            self.duration_since(earlier)
        }
    }

    impl super::Absolute for Instant {
        #[inline]
        fn now() -> Self {
            Instant::now()
        }
    }

    use std::time::SystemTime;

    impl super::Relative for SystemTime {
        /// Returns the difference in times between the two
        /// SystemTimes. Due to the fallible nature of SystemTimes,
        /// returns the zero duration if a negative duration would
        /// result (e.g. due to system clock adjustments).
        fn duration_since(&self, earlier: Self) -> Duration {
            self.duration_since(earlier)
                .unwrap_or_else(|_| Duration::new(0, 0))
        }
    }

    impl super::Absolute for SystemTime {
        #[inline]
        fn now() -> Self {
            SystemTime::now()
        }
    }
}

impl Relative for Duration {
    fn duration_since(&self, earlier: Self) -> Duration {
        *self - earlier
    }
}
