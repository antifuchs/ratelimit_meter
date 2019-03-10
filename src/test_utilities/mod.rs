#![doc(hidden)]
//! A module for code shared between integration tests & benchmarks in this crate.

pub mod algorithms;
pub mod variants;

use instant::DefaultRelativeInstant;
use lib::*;

/// Returns a "current" moment that's suitable for tests.
pub fn current_moment() -> DefaultRelativeInstant {
    #[cfg(feature = "std")]
    return Instant::now();

    #[cfg(not(feature = "std"))]
    return Duration::from_secs(90);
}
