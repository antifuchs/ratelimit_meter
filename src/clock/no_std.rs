use super::{Clock, Reference, Relative};
use crate::lib::*;

/// A mock implementation of a clock. All it does is keep track of
/// what "now" is (relative to some point meaningful to the program),
/// and returns that.
#[derive(Debug, PartialEq, Clone)]
pub struct FakeClock {
    now: Duration,
}

impl Default for FakeClock {
    fn default() -> Self {
        FakeClock {
            now: Duration::from_nanos(0),
        }
    }
}

impl FakeClock {
    /// Advances the fake clock by the given amount.
    pub fn advance(&mut self, by: Duration) {
        self.now += by
    }
}

impl Clock for FakeClock {
    type Instant = Duration;
    type Duration = Duration;

    fn now(&self) -> Self::Instant {
        self.now
    }
}
