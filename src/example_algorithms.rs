use {Decider, DeciderImpl, DirectDeciderImpl, NegativeMultiDecision};

use std::time::Instant;

impl<'a> Decider<'a> for Allower {}

#[derive(Default, Copy, Clone)]
/// The most naive implementation of a rate-limiter ever: Always
/// allows every cell through.
/// # Example
/// ```
/// use ratelimit_meter::{Decider};
/// use ratelimit_meter::example_algorithms::Allower;
/// let mut allower = Allower::new();
/// assert!(allower.check().is_ok());
/// ```
pub struct Allower {}

impl Allower {
    pub fn new() -> Allower {
        Allower::default()
    }
}

impl DeciderImpl for Allower {
    type BucketState = ();
    type BucketParams = ();

    /// Allows all cells through unconditionally.
    fn test_n_and_update(
        _state: &mut Self::BucketState,
        _params: &Self::BucketParams,
        _n: u32,
        _t0: Instant,
    ) -> Result<(), NegativeMultiDecision> {
        Ok(())
    }
}

impl<'a> DirectDeciderImpl<'a> for Allower {
    fn bucket_state(
        &mut self,
    ) -> (
        &'a mut <Self as DeciderImpl>::BucketState,
        &'a <Self as DeciderImpl>::BucketParams,
    ) {
        (&mut (), &())
    }
}
