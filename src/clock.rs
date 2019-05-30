//! Time sources for the rate limiter.
//!
//! The time sources contained in this module allow the rate limiter
//! to be (optionally) independent of std, and should additionally
//! allow mocking the passage of time.

use crate::lib::*;

/// A measurement from a clock.
pub trait Reference:
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
    /// Determines the time that separates two measurements of a clock.
    fn duration_since(&self, earlier: Self) -> Duration;
}

/// A time source used by rate limiters.
pub trait Clock: Default {
    /// A measurement of a monotonically increasing clock.
    type Instant: Reference;

    /// Returns a measurement of the clock.
    fn now(&self) -> Self::Instant;
}

impl Reference for Duration {
    fn duration_since(&self, earlier: Self) -> Duration {
        *self - earlier
    }
}

#[cfg(not(feature = "std"))]
mod no_std;
#[cfg(not(feature = "std"))]
pub use no_std::*;

#[cfg(feature = "std")]
mod with_std;
#[cfg(feature = "std")]
pub use with_std::*;
