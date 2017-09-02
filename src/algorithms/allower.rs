use {Decider, Decision, Limiter, Result};

use std::time::{Instant};

/// The most naive implementation of a rate-limiter ever: Always
/// allows every cell through.
pub struct Allower {}

impl Decider for Allower {
    /// Allower never returns a negative answer, so negative answers
    /// don't carry information.
    type T = ();

    /// Allows the cell through unconditionally.
    fn test_and_update(&mut self, _t0: Instant) -> Decision<()> {
        Decision::Yes
    }

    /// Builds the most useless rate-limiter in existence.
    fn build_with(_l: &Limiter) -> Result<Self> {
        Ok(Allower{})
    }
}
