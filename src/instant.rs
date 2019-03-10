use self::lib::{Add, Clone, Copy, Debug, Duration, Eq, Ord, PartialEq, Send, Sized, Sub, Sync};

/// A point in time that is used as a reference for measuring a rate
/// limit. On the clock, it has meaning only relative to some other point in time.
///
/// When using `no_std`, users of this crate are expected to provide
/// an impl of `RelativeInstant` that corresponds to their system's time source.
pub trait RelativeInstant:
    Sized
    + Sub<Duration, Output = Self>
    + Sub<Self, Output = Duration>
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

    // TODO: would love to have this but duration_since is not infallible:
    // use std::time::SystemTime
    // impl super::RelativeInstant for SystemTime {
    //     #[inline]
    //     fn now() -> Self {
    //         SystemTime::now()
    //     }
    // }
}

impl RelativeInstant for Duration {
    fn duration_since(&self, earlier: Self) -> Duration {
        *self - earlier
    }
}

mod lib {
    mod core {
        #[cfg(not(feature = "std"))]
        pub use core::*;

        #[cfg(feature = "std")]
        pub use std::*;
    }

    pub use self::core::borrow::Borrow;
    pub use self::core::clone::Clone;
    pub use self::core::cmp::{Eq, Ord, PartialEq};
    pub use self::core::default::Default;
    pub use self::core::fmt::Debug;
    pub use self::core::marker::Copy;
    pub use self::core::marker::Send;
    pub use self::core::marker::Sized;
    pub use self::core::marker::Sync;
    pub use self::core::ops::Add;
    pub use self::core::ops::Sub;
    pub use self::core::time::Duration;
}
