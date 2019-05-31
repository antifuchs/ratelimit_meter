use crate::algorithms::Algorithm;
use crate::clock;
use crate::state::DirectRateLimiter;

#[derive(Debug)]
pub enum Variant {
    GCRA,
    LeakyBucket,
}

impl Variant {
    pub const ALL: &'static [Variant; 2] = &[Variant::GCRA, Variant::LeakyBucket];
}

pub struct DirectBucket<A: Algorithm<C::Instant>, C: clock::Clock>(DirectRateLimiter<A, C>);
impl<A, C> Default for DirectBucket<A, C>
where
    C: clock::Clock,
    A: Algorithm<C::Instant>,
{
    fn default() -> Self {
        DirectBucket(DirectRateLimiter::per_second(nonzero!(50u32)))
    }
}
impl<A, C> DirectBucket<A, C>
where
    C: clock::Clock,
    A: Algorithm<C::Instant>,
{
    pub fn limiter(self) -> DirectRateLimiter<A, C> {
        self.0
    }
}

#[cfg(feature = "std")]
mod std {
    use super::*;
    use crate::{algorithms::KeyableRateLimitState, clock, KeyedRateLimiter};

    pub struct KeyedBucket<A: Algorithm<C::Instant>, C: clock::Clock>(KeyedRateLimiter<u32, A, C>)
    where
        A::BucketState: KeyableRateLimitState<A, C::Instant>;

    impl<A, C> Default for KeyedBucket<A, C>
    where
        A: Algorithm<C::Instant>,
        A::BucketState: KeyableRateLimitState<A, C::Instant>,
        C: clock::Clock,
    {
        fn default() -> Self {
            KeyedBucket(KeyedRateLimiter::per_second(nonzero!(50u32)))
        }
    }
    impl<A, C> KeyedBucket<A, C>
    where
        A: Algorithm<C::Instant>,
        A::BucketState: KeyableRateLimitState<A, C::Instant>,
        C: clock::Clock,
    {
        pub fn limiter(self) -> KeyedRateLimiter<u32, A, C> {
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
                let mut $var = $bucket::<
                    ::ratelimit_meter::GCRA<clock::DefaultReference>,
                    clock::DefaultClock,
                >::default()
                .limiter();
                $code
            }
            $crate::test_utilities::variants::Variant::LeakyBucket => {
                let mut $var = $bucket::<
                    ::ratelimit_meter::LeakyBucket<clock::DefaultReference>,
                    clock::DefaultClock,
                >::default()
                .limiter();
                $code
            }
        }
    };
}
