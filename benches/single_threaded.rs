use std::time::{Duration, Instant};

use criterion::{black_box, Criterion, ParameterizedBenchmark, Throughput};
use ratelimit_meter::test_utilities::variants::{DirectBucket, Variant};

#[cfg(all(feature = "std", feature = "sync"))]
use ratelimit_meter::test_utilities::variants::KeyedBucket;

pub fn bench_all(c: &mut Criterion) {
    bench_direct(c);

    #[cfg(all(feature = "std", feature = "sync"))]
    bench_keyed(c);
}

fn bench_direct(c: &mut Criterion) {
    let id = "single_threaded/direct";
    let bm = ParameterizedBenchmark::new(
        id,
        move |b, ref v| {
            bench_with_variants!(v, rl: DirectBucket, {
                let now = Instant::now();
                let ms = Duration::from_millis(20);
                let mut i = 0;
                b.iter(|| {
                    i += 1;
                    black_box(rl.check_at(now + (ms * i)).is_ok());
                });
            });
        },
        Variant::ALL,
    )
    .throughput(|_s| Throughput::Elements(1));
    c.bench(id, bm);
}

#[cfg(all(feature = "std", feature = "sync"))]
fn bench_keyed(c: &mut Criterion) {
    let id = "single_threaded/keyed";
    let bm = ParameterizedBenchmark::new(
        id,
        move |b, ref v| {
            bench_with_variants!(v, rl: KeyedBucket, {
                let now = Instant::now();
                let ms = Duration::from_millis(20);
                let mut i = 0;
                b.iter(|| {
                    i += 1;
                    black_box(rl.check_at(i % 100, now + (ms * i)).is_ok());
                });
            });
        },
        Variant::ALL,
    )
    .throughput(|_s| Throughput::Elements(1));
    c.bench(id, bm);
}
