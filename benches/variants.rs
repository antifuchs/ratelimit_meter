use ratelimit_meter::algorithms::Algorithm;
use ratelimit_meter::{DirectRateLimiter, KeyedRateLimiter, NegativeMultiDecision};

#[derive(Debug)]
pub enum Variant {
    GCRA,
    LeakyBucket,
}

impl Variant {
    pub const ALL: &'static [Variant; 2] = &[Variant::GCRA, Variant::LeakyBucket];
}

pub struct BenchmarkDirectBucket<A: Algorithm>(DirectRateLimiter<A>);
impl<A> Default for BenchmarkDirectBucket<A>
where
    A: Algorithm,
{
    fn default() -> Self {
        BenchmarkDirectBucket(DirectRateLimiter::per_second(nonzero!(50u32)))
    }
}
impl<A> BenchmarkDirectBucket<A>
where
    A: Algorithm,
{
    pub fn limiter(self) -> DirectRateLimiter<A> {
        self.0
    }
}

pub struct BenchmarkKeyedBucket<A: Algorithm>(KeyedRateLimiter<u32, A>);
impl<A> Default for BenchmarkKeyedBucket<A>
where
    A: Algorithm,
{
    fn default() -> Self {
        BenchmarkKeyedBucket(KeyedRateLimiter::per_second(nonzero!(50u32)))
    }
}
impl<A> BenchmarkKeyedBucket<A>
where
    A: Algorithm,
{
    pub fn limiter(self) -> KeyedRateLimiter<u32, A> {
        self.0
    }
}

// I really wish I could just have a function that returns an impl
// Trait that was usable in all the benchmarks, but alas it should not
// be so.
macro_rules! run_with_variants {
    ($variant:expr, $var:ident : $bucket:tt, $code:block) => {
        match $variant {
            $crate::variants::Variant::GCRA => {
                let mut $var = $bucket::<::ratelimit_meter::GCRA>::default().limiter();
                $code
            }
            $crate::variants::Variant::LeakyBucket => {
                let mut $var = $bucket::<::ratelimit_meter::LeakyBucket>::default().limiter();
                $code
            }
        }
    };
}

/// A representation of a bare in-memory algorithm, without any bucket
/// attached.
pub struct AlgorithmForBenchmark<A: Algorithm>(A::BucketParams);

impl<'a, A> AlgorithmForBenchmark<A>
where
    A: Algorithm,
{
    pub fn new() -> Self {
        AlgorithmForBenchmark(
            A::params_from_constructor(
                nonzero!(50u32),
                nonzero!(1u32),
                ::std::time::Duration::from_secs(1),
            ).unwrap(),
        )
    }

    pub fn params(&'a self) -> &'a A::BucketParams {
        &self.0
    }

    pub fn state(&self) -> A::BucketState {
        A::BucketState::default()
    }

    pub fn check(
        &self,
        state: &A::BucketState,
        params: &A::BucketParams,
        t0: ::std::time::Instant,
    ) -> Result<(), A::NegativeDecision> {
        A::test_and_update(state, params, t0)
    }

    pub fn check_n(
        &self,
        state: &A::BucketState,
        params: &A::BucketParams,
        n: u32,
        t0: ::std::time::Instant,
    ) -> Result<(), NegativeMultiDecision<A::NegativeDecision>> {
        A::test_n_and_update(state, params, n, t0)
    }
}

macro_rules! run_with_algorithm_variants {
    ($variant:expr, $var:ident, $code:block) => {
        match $variant {
            $crate::variants::Variant::GCRA => {
                let mut $var =
                    $crate::variants::AlgorithmForBenchmark::<::ratelimit_meter::GCRA>::new();
                $code
            }
            $crate::variants::Variant::LeakyBucket => {
                let mut $var = $crate::variants::AlgorithmForBenchmark::<
                    ::ratelimit_meter::LeakyBucket,
                >::new();
                $code
            }
        }
    };
}
