use std::thread;
use std::time::{Duration, Instant};

use super::variants::{BenchmarkDirectBucket, BenchmarkKeyedBucket, Variant};
use criterion::{black_box, Criterion, ParameterizedBenchmark, Throughput};

pub fn bench_all(c: &mut Criterion) {
    bench_direct(c);
    bench_keyed(c);
}

fn bench_direct(c: &mut Criterion) {
    let id = "multi_threaded/direct";

    let bm = ParameterizedBenchmark::new(
        id,
        |b, ref v| {
            run_with_variants!(v, lim: BenchmarkDirectBucket, {
                let now = Instant::now();
                let ms = Duration::from_millis(20);
                let mut children = vec![];

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

fn bench_keyed(c: &mut Criterion) {
    let id = "multi_threaded/keyed";

    let bm = ParameterizedBenchmark::new(
        id,
        |b, ref v| {
            run_with_variants!(v, lim: BenchmarkKeyedBucket, {
                let now = Instant::now();
                let ms = Duration::from_millis(20);
                let mut children = vec![];

                for _i in 0..19 {
                    let mut lim = lim.clone();
                    let mut b = b.clone();
                    children.push(thread::spawn(move || {
                        let mut i = 0;
                        b.iter(|| {
                            i += 1;
                            black_box(lim.check_at(i % 100, now + (ms * i)).is_ok());
                        });
                    }));
                }
                let mut i = 0;
                b.iter(|| {
                    i += 1;
                    black_box(lim.check_at(i % 100, now + (ms * i)).is_ok());
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
