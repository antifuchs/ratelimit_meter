//! Time sources for the rate limiter.
//!
//! The time sources contained in this module allow the rate limiter
//! to be (optionally) independent of std, and should additionally
//! allow mocking the passage of time.

use crate::lib::*;

/// A point in time that is used as a reference for measuring a rate
/// limit. On a clock, it has meaning only relative to some other
/// point in time.
pub trait Relative:
    Sized
    + Sub<Self, Output = Self>
    + Add<Self, Output = Self>
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
    fn duration_since(&self, earlier: Self) -> Self {
        *self - earlier
    }
}

/// A measurement from a clock.
pub trait Reference<Relative>:
    Sized
    + Sub<Relative, Output = Self>
    + Add<Relative, Output = Self>
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
    fn duration_since(&self, earlier: Self) -> Relative;
}

/// A time source used by rate limiters.
pub trait Clock {
    /// A measurement of a monotonically increasing clock.
    type Instant: Reference<Self::Duration>;

    /// An interval between two measurements.
    type Duration: Relative;

    /// Returns a measurement of the clock.
    fn now(&self) -> Self::Instant;
}

impl Reference<Duration> for Duration {
    fn duration_since(&self, earlier: Self) -> Duration {
        *self - earlier
    }
}
impl Relative for Duration {}

#[cfg(not(feature = "std"))]
mod no_std;
#[cfg(not(feature = "std"))]
pub use no_std::*;

#[cfg(feature = "std")]
mod with_std;
#[cfg(feature = "std")]
pub use with_std::*;
