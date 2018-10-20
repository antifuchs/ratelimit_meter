use std::time::{Duration, Instant};

use ratelimit_meter::{Decider, MultiDecider};

use super::variants::Variant;

use criterion::{black_box, Criterion, ParameterizedBenchmark, Throughput};

pub fn bench_all(c: &mut Criterion) {
    bench_single_element(c);
    bench_multi_element(c);
}

fn bench_single_element(c: &mut Criterion) {
    let id = "single_threaded/1_element";
    let bm = ParameterizedBenchmark::new(
        id,
        move |b, ref v| {
            run_with_variants!(v, rl, {
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
    ).throughput(|_s| Throughput::Elements(1));
    c.bench(id, bm);
}

fn bench_multi_element(c: &mut Criterion) {
    let id = "single_threaded/multi_element";
    let elements: u32 = 10;
    let bm = ParameterizedBenchmark::new(
        id,
        move |b, ref v| {
            run_with_variants!(v, rl, {
                let now = Instant::now();
                let ms = Duration::from_millis(20);
                let mut i = 0;
                b.iter(|| {
                    i += 1;
                    black_box(rl.check_n_at(elements, now + (ms * i)).is_ok());
                });
            });
        },
        Variant::ALL,
    ).throughput(move |_s| Throughput::Elements(elements));
    c.bench(id, bm);
}
