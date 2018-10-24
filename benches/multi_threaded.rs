use std::thread;
use std::time::{Duration, Instant};

use super::variants::{BenchmarkDirectBucket, Variant};
use criterion::{black_box, Criterion, ParameterizedBenchmark, Throughput};
use ratelimit_meter::DirectRateLimiter;

pub fn bench_all(c: &mut Criterion) {
    let id = "multi_threaded/20_threads";

    let bm = ParameterizedBenchmark::new(
        id,
        |b, ref v| {
            run_with_variants!(v, lim: BenchmarkDirectBucket, {
                let now = Instant::now();
                let ms = Duration::from_millis(20);
                let mut children = vec![];

                lim.check_at(now).unwrap();
                for _i in 0..19 {
                    let mut lim = lim.clone();
                    let mut b = b.clone();
                    children.push(thread::spawn(move || {
                        let mut i = 0;
                        b.iter(|| {
                            i += 1;
                            black_box(lim.check_at(now + (ms * i)).is_ok());
                        });
                    }));
                }
                let mut i = 0;
                b.iter(|| {
                    i += 1;
                    black_box(lim.check_at(now + (ms * i)).is_ok());
                });
                for child in children {
                    child.join().unwrap();
                }
            });
        },
        Variant::ALL,
    ).throughput(|_s| Throughput::Elements(1));
    c.bench(id, bm);
}
