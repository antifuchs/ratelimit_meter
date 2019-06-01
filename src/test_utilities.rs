#![doc(hidden)]
//! A module for code shared between integration tests & benchmarks in this crate.

pub mod algorithms;
pub mod variants;

use crate::lib::*;

use crate::clock;
use crate::clock::Clock;

/// Returns a "current" moment that's suitable for tests.
pub fn current_moment() -> clock::DefaultReference {
    let c = clock::DefaultClock::default();
    c.now()
}
