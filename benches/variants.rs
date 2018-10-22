#[derive(Debug)]
pub enum Variant {
    GCRA,
    LeakyBucket,
}

impl Variant {
    pub const ALL: &'static [Variant; 2] = &[Variant::GCRA, Variant::LeakyBucket];
}

// I really wish I could just have a function that returns an impl
// Trait that was usable in all the benchmarks, but alas it should not
// be so.
macro_rules! run_with_variants {
    ($variant:expr, $var:ident, $code:block) => {
        match $variant {
            $crate::variants::Variant::GCRA => {
                let mut $var =
                    ::ratelimit_meter::DirectRateLimiter::<::ratelimit_meter::GCRA>::per_second(
                        nonzero!(50u32)
                    );
                $code
            }
            $crate::variants::Variant::LeakyBucket => {
                let mut $var = ::ratelimit_meter::DirectRateLimiter::<
                    ::ratelimit_meter::LeakyBucket,
                >::per_second(
                    nonzero!(50u32)
                );
                $code
            }
        }
    };
}
