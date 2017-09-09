use {MultiDeciderImpl, TypedDecider, ImpliedDeciderImpl, MultiDecider, Decider, Decision, Result};

use std::time::Instant;


#[derive(Copy, Clone)]
/// The most naive implementation of a rate-limiter ever: Always
/// allows every cell through.
/// # Example
/// ```
/// use ratelimit_meter::{Decider};
/// use ratelimit_meter::example_algorithms::Allower;
/// let mut allower = Allower::new();
/// assert!(allower.check().unwrap().is_compliant());
/// ```
pub struct Allower {}

impl Allower {
    pub fn new() -> Allower {
        Allower {}
    }
}

impl TypedDecider for Allower {
    /// Allower never returns a negative answer, so negative answers
    /// don't carry information.
    type T = ();
}

impl MultiDeciderImpl for Allower {
    /// Allows all cells through unconditionally.
    fn test_n_and_update(&mut self, _n: u32, _t0: Instant) -> Result<Decision<()>> {
        Ok(Decision::Yes)
    }
}

impl ImpliedDeciderImpl for Allower {}

impl Decider for Allower {}
impl MultiDecider for Allower {}
