use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use {algorithms::Algorithm, NegativeMultiDecision};

/// A representation of a bare in-memory algorithm, without any bucket
/// attached.
#[derive(Debug)]
pub struct AlgorithmForTest<A: Algorithm>(A);

impl<'a, A> AlgorithmForTest<A>
where
    A: Algorithm,
{
    pub fn new<U: Into<Option<NonZeroU32>>, D: Into<Option<Duration>>>(
        cap: NonZeroU32,
        weight: U,
        duration: D,
    ) -> Self {
        AlgorithmForTest(
            A::construct(
                cap,
                weight.into().unwrap_or(nonzero!(1u32)),
                duration
                    .into()
                    .unwrap_or(::std::time::Duration::from_secs(1)),
            ).unwrap(),
        )
    }

    pub fn algorithm(&'a self) -> &'a A {
        &self.0
    }

    pub fn state(&self) -> A::BucketState {
        A::BucketState::default()
    }

    pub fn check(&self, state: &A::BucketState, t0: Instant) -> Result<(), A::NegativeDecision> {
        self.0.test_and_update(state, t0)
    }

    pub fn check_n(
        &self,
        state: &A::BucketState,
        n: u32,
        t0: Instant,
    ) -> Result<(), NegativeMultiDecision<A::NegativeDecision>> {
        self.0.test_n_and_update(state, n, t0)
    }
}

impl<A> Default for AlgorithmForTest<A>
where
    A: Algorithm,
{
    fn default() -> Self {
        Self::new(nonzero!(1u32), None, None)
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! bench_with_algorithm_variants {
    ($variant:expr, $var:ident, $code:block) => {
        match $variant {
            $crate::test_utilities::variants::Variant::GCRA => {
                let mut $var =
                    $crate::test_utilities::algorithms::AlgorithmForTest::<$crate::GCRA>::default();
                $code
            }
            $crate::test_utilities::variants::Variant::LeakyBucket => {
                let mut $var = $crate::test_utilities::algorithms::AlgorithmForTest::<
                    $crate::LeakyBucket,
                >::default();
                $code
            }
        }
    };
}
