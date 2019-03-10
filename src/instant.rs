use lib::*;

/// A point in time that is used as a reference for measuring a rate
/// limit. On the clock, it has meaning only relative to some other point in time.
///
/// When using `no_std`, users of this crate are expected to provide
/// an impl of `RelativeInstant` that corresponds to their system's time source.
pub trait RelativeInstant:
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
pub trait AbsoluteInstant: RelativeInstant {
    /// Returns the current moment in time, as given by the time
    /// source.
    fn now() -> Self;
}

#[cfg(feature = "std")]
mod std {
    use std::time::{Duration, Instant};

    impl super::RelativeInstant for Instant {
        fn duration_since(&self, earlier: Self) -> Duration {
            self.duration_since(earlier)
        }
    }

    impl super::AbsoluteInstant for Instant {
        #[inline]
        fn now() -> Self {
            Instant::now()
        }
    }

    use std::time::SystemTime;

    impl super::RelativeInstant for SystemTime {
        /// Returns the difference in times between the two
        /// SystemTimes. Due to the fallible nature of SystemTimes,
        /// returns the zero duration if a negative duration would
        /// result (e.g. due to system clock adjustments).
        fn duration_since(&self, earlier: Self) -> Duration {
            self.duration_since(earlier)
                .unwrap_or_else(|_| Duration::new(0, 0))
        }
    }

    impl super::AbsoluteInstant for SystemTime {
        #[inline]
        fn now() -> Self {
            SystemTime::now()
        }
    }
}

impl RelativeInstant for Duration {
    fn duration_since(&self, earlier: Self) -> Duration {
        *self - earlier
    }
}
