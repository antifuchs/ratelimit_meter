use crate::algorithms::Algorithm;
use crate::instant;
use crate::state::DirectRateLimiter;

#[derive(Debug)]
pub enum Variant {
    GCRA,
    LeakyBucket,
}

impl Variant {
    pub const ALL: &'static [Variant; 2] = &[Variant::GCRA, Variant::LeakyBucket];
}

pub struct DirectBucket<A: Algorithm<P>, P: instant::Relative>(DirectRateLimiter<A, P>);
impl<A, P> Default for DirectBucket<A, P>
where
    P: instant::Relative,
    A: Algorithm<P>,
{
    fn default() -> Self {
        DirectBucket(DirectRateLimiter::per_second(nonzero!(50u32)))
    }
}
impl<A, P> DirectBucket<A, P>
where
    P: instant::Relative,
    A: Algorithm<P>,
{
    pub fn limiter(self) -> DirectRateLimiter<A, P> {
        self.0
    }
}

#[cfg(feature = "std")]
mod std {
    use super::*;
    use crate::{algorithms::KeyableRateLimitState, instant::Absolute, KeyedRateLimiter};

    pub struct KeyedBucket<A: Algorithm<P>, P: Absolute>(KeyedRateLimiter<u32, A, P>)
    where
        A::BucketState: KeyableRateLimitState<A, P>;

    impl<A, P> Default for KeyedBucket<A, P>
    where
        A: Algorithm<P>,
        A::BucketState: KeyableRateLimitState<A, P>,
        P: Absolute,
    {
        fn default() -> Self {
            KeyedBucket(KeyedRateLimiter::per_second(nonzero!(50u32)))
        }
    }
    impl<A, P> KeyedBucket<A, P>
    where
        A: Algorithm<P>,
        A::BucketState: KeyableRateLimitState<A, P>,
        P: Absolute,
    {
        pub fn limiter(self) -> KeyedRateLimiter<u32, A, P> {
            self.0
        }
    }
}
#[cfg(feature = "std")]
pub use self::std::*;

// I really wish I could just have a function that returns an impl
// Trait that was usable in all the benchmarks, but alas it should not
// be so.
#[doc(hidden)]
#[macro_export]
macro_rules! bench_with_variants {
    ($variant:expr, $var:ident : $bucket:tt, $code:block) => {
        match $variant {
            $crate::test_utilities::variants::Variant::GCRA => {
                let mut $var =
                    $bucket::<::ratelimit_meter::GCRA<Instant>, Instant>::default().limiter();
                $code
            }
            $crate::test_utilities::variants::Variant::LeakyBucket => {
                let mut $var =
                    $bucket::<::ratelimit_meter::LeakyBucket<Instant>, Instant>::default()
                        .limiter();
                $code
            }
        }
    };
}
