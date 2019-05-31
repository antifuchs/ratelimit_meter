use super::{Clock, Reference};
use crate::lib::*;
use parking_lot::Mutex;
use std::time::SystemTime;

/// The default clock reference point in time: [`Instant`].
pub type DefaultReference = Instant;

/// The default clock that reports [`Instant`]s.
pub type DefaultClock = MonotonicClock;

/// A mock implementation of a clock tracking [`Instant`]s. All it
/// does is keep track of what "now" is by allowing the program to
/// increment the current time (taken at time of construction) by some
/// arbitrary [`Duration`].
#[derive(Debug, Clone)]
pub struct FakeAbsoluteClock {
    now: Arc<Mutex<Instant>>,
}

impl Default for FakeAbsoluteClock {
    fn default() -> Self {
        FakeAbsoluteClock {
            now: Arc::new(Mutex::new(Instant::now())),
        }
    }
}

impl FakeAbsoluteClock {
    /// Advances the fake clock by the given amount.
    pub fn advance(&mut self, by: Duration) {
        *(self.now.lock()) += by
    }
}

impl Clock for FakeAbsoluteClock {
    type Instant = Instant;

    fn now(&self) -> Self::Instant {
        *self.now.lock()
    }
}

/// The monotonic clock implemented by [`Instant`].
pub struct MonotonicClock();

impl Default for MonotonicClock {
    fn default() -> Self {
        MonotonicClock()
    }
}

impl Reference for Instant {
    fn duration_since(&self, earlier: Self) -> Duration {
        *self - earlier
    }
}

impl Clock for MonotonicClock {
    type Instant = Instant;

    fn now(&self) -> Self::Instant {
        Instant::now()
    }
}

/// The non-monotonic clock implemented by [`SystemTime`].
pub struct SystemClock();

impl Default for SystemClock {
    fn default() -> Self {
        SystemClock()
    }
}

impl Reference for SystemTime {
    /// Returns the difference in times between the two
    /// SystemTimes. Due to the fallible nature of SystemTimes,
    /// returns the zero duration if a negative duration would
    /// result (e.g. due to system clock adjustments).
    fn duration_since(&self, earlier: Self) -> Duration {
        self.duration_since(earlier)
            .unwrap_or_else(|_| Duration::new(0, 0))
    }
}

impl Clock for SystemClock {
    type Instant = SystemTime;

    fn now(&self) -> Self::Instant {
        SystemTime::now()
    }
}
