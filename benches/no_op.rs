use std::time::Duration;

use ratelimit_meter::example_algorithms::{Allower, Always};
use ratelimit_meter::instant::AbsoluteInstant;
use ratelimit_meter::test_utilities::algorithms::AlgorithmForTest;

use criterion::{black_box, Benchmark, Criterion, Throughput};

pub fn bench_all(c: &mut Criterion) {
    let id = "algorithm/no_op";

    let bm = Benchmark::new(id, move |b| {
        let algo = AlgorithmForTest::<Allower, Always>::default();
        let now = Always::now();
        let ms = Duration::from_millis(20);
        let state = algo.state();
        let mut i = 0;
        b.iter(|| {
            i += 1;
            black_box(algo.check(&state, now + (ms * i)).is_ok());
        });
    })
    .throughput(Throughput::Elements(1));
    c.bench(id, bm);
}
