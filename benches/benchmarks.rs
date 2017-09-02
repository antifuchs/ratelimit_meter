#![feature(test)]

extern crate test;
extern crate ratelimit_meter;

use ratelimit_meter::{Allower, GCRA, Limiter, Decider};
use std::time::{Instant, Duration};

#[bench]
fn bench_gcra(b: &mut test::Bencher) {
    let mut gcra = Limiter::new().capacity(50).weight(1).build::<GCRA>();
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut i = 0;
    b.iter(|| {
        i += 1;
        gcra.test_and_update(now + (ms * i))
    });
}

#[bench]
fn bench_allower(b: &mut test::Bencher) {
    let mut allower = Limiter::new().capacity(50).weight(1).build::<Allower>();
    b.iter(|| allower.check());
}
