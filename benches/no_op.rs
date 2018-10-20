use std::time::{Duration, Instant};

use ratelimit_meter::example_algorithms::Allower;
use ratelimit_meter::{Decider, MultiDecider};

use criterion::{black_box, Criterion};

pub fn bench_all(c: &mut Criterion) {
    c.bench_function("no-op single-element decision", |b| {
        let mut rl = Allower::new();
        let now = Instant::now();
        let ms = Duration::from_millis(20);
        let mut i = 0;
        b.iter(|| {
            i += 1;
            black_box(rl.check_at(now + (ms * i)).is_ok());
        });
    });
    c.bench_function("no-op multi-element decision", |b| {
        let mut rl = Allower::new();
        let now = Instant::now();
        let ms = Duration::from_millis(20);
        let mut i = 0;
        b.iter(|| {
            i += 1;
            black_box(rl.check_n_at(10, now + (ms * i)).is_ok());
        });
    });
}
