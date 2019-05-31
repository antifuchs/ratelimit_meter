use super::FakeRelativeClock;
use crate::lib::*;

/// The default time reference in `no_std` is [`Duration`].
pub type DefaultReference = Duration;

/// The default `no_std` clock that reports [`Durations`] must be advanced by the program.
pub type DefaultClock = FakeRelativeClock;
