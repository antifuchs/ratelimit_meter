use algorithms::Algorithm;
use std::time::Instant;
use {instant::Point, DirectRateLimiter, KeyedRateLimiter};

#[derive(Debug)]
pub enum Variant {
    GCRA,
    LeakyBucket,
}

impl Variant {
    pub const ALL: &'static [Variant; 2] = &[Variant::GCRA, Variant::LeakyBucket];
}

pub struct DirectBucket<P: Point, A: Algorithm<P>>(DirectRateLimiter<A, P>);
impl<P, A> Default for DirectBucket<P, A>
where
    P: Point,
    A: Algorithm<P>,
{
    fn default() -> Self {
        DirectBucket(DirectRateLimiter::per_second(nonzero!(50u32)))
    }
}
impl<P, A> DirectBucket<P, A>
where
    P: Point,
    A: Algorithm<P>,
{
    pub fn limiter(self) -> DirectRateLimiter<A, P> {
        self.0
    }
}

pub struct KeyedBucket<A: Algorithm<Instant>>(KeyedRateLimiter<u32, A>);
impl<A> Default for KeyedBucket<A>
where
    A: Algorithm<Instant>,
{
    fn default() -> Self {
        KeyedBucket(KeyedRateLimiter::per_second(nonzero!(50u32)))
    }
}
impl<A> KeyedBucket<A>
where
    A: Algorithm<Instant>,
{
    pub fn limiter(self) -> KeyedRateLimiter<u32, A> {
        self.0
    }
}

// I really wish I could just have a function that returns an impl
// Trait that was usable in all the benchmarks, but alas it should not
// be so.
#[doc(hidden)]
#[macro_export]
macro_rules! bench_with_variants {
    ($variant:expr, $var:ident : $bucket:tt, $code:block) => {
        match $variant {
            $crate::test_utilities::variants::Variant::GCRA => {
                let mut $var = $bucket::<::ratelimit_meter::GCRA>::default().limiter();
                $code
            }
            $crate::test_utilities::variants::Variant::LeakyBucket => {
                let mut $var = $bucket::<::ratelimit_meter::LeakyBucket>::default().limiter();
                $code
            }
        }
    };
}
