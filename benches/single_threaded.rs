#![feature(test)]

extern crate ratelimit_meter;
extern crate test;

use ratelimit_meter::example_algorithms::Allower;
use ratelimit_meter::{Decider, LeakyBucket, MultiDecider, GCRA};
use std::num::NonZeroU32;
use std::time::{Duration, Instant};

#[bench]
fn bench_gcra(b: &mut test::Bencher) {
    let mut gcra = GCRA::for_capacity(50)
        .unwrap()
        .cell_weight(1)
        .unwrap()
        .build();
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut i = 0;
    b.iter(|| {
        i += 1;
        gcra.check_at(now + (ms * i)).is_ok();
    });
}

#[bench]
fn bench_gcra_bulk(b: &mut test::Bencher) {
    let mut gcra = GCRA::for_capacity(500)
        .unwrap()
        .cell_weight(1)
        .unwrap()
        .build();
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut i = 0;
    b.iter(|| {
        i += 1;
        gcra.check_n_at(10, now + (ms * i)).is_ok();
    });
}

#[bench]
fn bench_leaky_bucket(b: &mut test::Bencher) {
    let mut lb = LeakyBucket::per_second(NonZeroU32::new(50).unwrap());
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut i = 0;
    b.iter(|| {
        i += 1;
        lb.check_at(now + (ms * i)).is_ok();
    });
}

#[bench]
fn bench_leaky_bucket_bulk(b: &mut test::Bencher) {
    let mut lb = LeakyBucket::per_second(NonZeroU32::new(500).unwrap());
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut i = 0;
    b.iter(|| {
        i += 1;
        lb.check_n_at(10, now + (ms * i)).is_ok();
    });
}

#[bench]
fn bench_allower(b: &mut test::Bencher) {
    let mut allower = Allower::new();
    b.iter(|| allower.check().unwrap());
}

#[bench]
fn bench_threadsafe_leaky_bucket(b: &mut test::Bencher) {
    let mut lb = LeakyBucket::per_second(NonZeroU32::new(50).unwrap());
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut i = 0;
    b.iter(|| {
        i += 1;
        lb.check_at(now + (ms * i)).is_ok();
    });
}
