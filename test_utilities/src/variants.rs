use ratelimit_meter::algorithms::Algorithm;
use ratelimit_meter::{DirectRateLimiter, KeyedRateLimiter};

#[derive(Debug)]
pub enum Variant {
    GCRA,
    LeakyBucket,
}

impl Variant {
    pub const ALL: &'static [Variant; 2] = &[Variant::GCRA, Variant::LeakyBucket];
}

pub struct DirectBucket<A: Algorithm>(DirectRateLimiter<A>);
impl<A> Default for DirectBucket<A>
where
    A: Algorithm,
{
    fn default() -> Self {
        DirectBucket(DirectRateLimiter::per_second(nonzero!(50u32)))
    }
}
impl<A> DirectBucket<A>
where
    A: Algorithm,
{
    pub fn limiter(self) -> DirectRateLimiter<A> {
        self.0
    }
}

pub struct KeyedBucket<A: Algorithm>(KeyedRateLimiter<u32, A>);
impl<A> Default for KeyedBucket<A>
where
    A: Algorithm,
{
    fn default() -> Self {
        KeyedBucket(KeyedRateLimiter::per_second(nonzero!(50u32)))
    }
}
impl<A> KeyedBucket<A>
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
#[macro_export]
macro_rules! bench_with_variants {
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
