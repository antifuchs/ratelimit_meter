use {DeciderImpl, Decider, Decision, Result};

use std::time::Instant;


#[derive(Copy, Clone)]
/// The most naive implementation of a rate-limiter ever: Always
/// allows every cell through.
pub struct Allower {}

impl Allower {
    pub fn new() -> Allower {
        Allower {}
    }
}

impl DeciderImpl for Allower {
    /// Allower never returns a negative answer, so negative answers
    /// don't carry information.
    type T = ();

    /// Allows the cell through unconditionally.
    fn test_and_update(&mut self, _t0: Instant) -> Result<Decision<()>> {
        Ok(Decision::Yes)
    }
}

impl Decider for Allower {}
