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
                let mut $var = ::ratelimit_meter::per_second::<::ratelimit_meter::GCRA>(
                    ::std::num::NonZeroU32::new(50).unwrap(),
                );
                $code
            }
            $crate::variants::Variant::LeakyBucket => {
                let mut $var = ::ratelimit_meter::per_second::<::ratelimit_meter::LeakyBucket>(
                    ::std::num::NonZeroU32::new(50).unwrap(),
                );
                $code
            }
        }
    };
}
