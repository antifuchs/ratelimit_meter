#[cfg(not(feature = "sync"))]
mod implementation {
    use criterion::Criterion;
    pub fn bench_all(_c: &mut Criterion) {}
}

#[cfg(feature = "sync")]
mod implementation {
    use criterion::{black_box, Criterion, ParameterizedBenchmark, Throughput};
    use std::thread;
    use std::time::{Duration, Instant};

    use ratelimit_meter::test_utilities::variants::{DirectBucket, KeyedBucket, Variant};

    pub fn bench_all(c: &mut Criterion) {
        bench_direct(c);

        #[cfg(feature = "std")]
        bench_keyed(c);
    }

    fn bench_direct(c: &mut Criterion) {
        let id = "multi_threaded/direct";

        let bm = ParameterizedBenchmark::new(
            id,
            |b, ref v| {
                bench_with_variants!(v, lim: DirectBucket, {
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
        )
        .throughput(|_s| Throughput::Elements(1));
        c.bench(id, bm);
    }

    #[cfg(feature = "std")]
    fn bench_keyed(c: &mut Criterion) {
        let id = "multi_threaded/keyed";

        let bm = ParameterizedBenchmark::new(
            id,
            |b, ref v| {
                bench_with_variants!(v, lim: KeyedBucket, {
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
        )
        .throughput(|_s| Throughput::Elements(1));
        c.bench(id, bm);
    }
}

pub use self::implementation::*;
