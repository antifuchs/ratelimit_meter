#[macro_use]
extern crate criterion;
extern crate ratelimit_meter;
#[macro_use]
extern crate nonzero_ext;

use criterion::Criterion;

#[macro_use]
mod variants;

mod algorithms;
mod multi_threaded;
mod no_op;
mod single_threaded;

criterion_group!(
    benches,
    algorithms::bench_all,
    multi_threaded::bench_all,
    single_threaded::bench_all,
    no_op::bench_all,
);
criterion_main!(benches);
