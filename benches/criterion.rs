#[macro_use]
extern crate criterion;
#[macro_use]
extern crate ratelimit_meter;

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
