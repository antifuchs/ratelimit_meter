use super::{Clock, Reference};
use crate::lib::*;
use parking_lot::Mutex;
use std::time::SystemTime;

/// A mock implementation of a clock. All it does is keep track of
/// what "now" is (relative to some point meaningful to the program),
/// and returns that.
#[derive(Debug, Clone)]
pub struct FakeClock {
    now: Arc<Mutex<Duration>>,
}

impl FakeClock {
    /// Advances the fake clock by the given amount.
    pub fn advance(&mut self, by: Duration) {
        *(self.now.lock()) += by
    }
}

impl Clock for FakeClock {
    type Instant = Duration;
    type Duration = Duration;

    fn now(&self) -> Self::Instant {
        self.now.lock().clone()
    }
}

/// The monotonic clock implemented by [`Instant`].
pub struct MonotonicClock();

impl Reference<Duration> for Instant {
    fn duration_since(&self, earlier: Self) -> Duration {
        *self - earlier
    }
}

impl Clock for MonotonicClock {
    type Instant = Instant;
    type Duration = Duration;

    fn now(&self) -> Self::Instant {
        Instant::now()
    }
}

/// The non-monotonic clock implemented by [`SystemTime`].
pub struct SystemClock();

impl Reference<Duration> for SystemTime {
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
    type Duration = Duration;

    fn now(&self) -> Self::Instant {
        SystemTime::now()
    }
}
