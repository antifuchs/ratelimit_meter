#![feature(test)]

extern crate test;
extern crate ratelimit_meter;

use ratelimit_meter::{GCRA, Threadsafe, Decider, MultiDecider};
use ratelimit_meter::example_algorithms::Allower;
use std::time::{Instant, Duration};

#[bench]
fn bench_gcra(b: &mut test::Bencher) {
    let mut gcra = GCRA::for_capacity(50).unwrap().cell_weight(1).unwrap().build();
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut i = 0;
    b.iter(|| {
        i += 1;
        gcra.check_at(now + (ms * i)).unwrap();
    });
}

#[bench]
fn bench_gcra_bulk(b: &mut test::Bencher) {
    let mut gcra = GCRA::for_capacity(500).unwrap().cell_weight(1).unwrap().build();
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut i = 0;
    b.iter(|| {
        i += 1;
        gcra.check_n_at(10, now + (ms * i)).unwrap();
    });
}

#[bench]
fn bench_allower(b: &mut test::Bencher) {
    let mut allower = Allower::new();
    b.iter(|| allower.check().unwrap());
}

#[bench]
fn bench_threadsafe_gcra(b: &mut test::Bencher) {
    let mut gcra = GCRA::for_capacity(50).unwrap().cell_weight(1).unwrap().build_sync();
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut i = 0;
    b.iter(|| {
        i += 1;
        gcra.check_at(now + (ms * i)).unwrap();
    });
}

#[bench]
fn bench_threadsafe_allower(b: &mut test::Bencher) {
    let allower_one = Allower::new();
    let mut threadsafe_allower = Threadsafe::new(allower_one);
    b.iter(|| threadsafe_allower.check());
}
