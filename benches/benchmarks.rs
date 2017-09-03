#![feature(test)]

extern crate test;
extern crate ratelimit_meter;

use ratelimit_meter::{GCRA, Threadsafe, Decider};
use ratelimit_meter::example_algorithms::Allower;
use std::time::{Instant, Duration};
use std::thread;

#[bench]
fn bench_gcra(b: &mut test::Bencher) {
    let mut gcra = GCRA::for_capacity(50).cell_weight(1).build();
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut i = 0;
    b.iter(|| {
        i += 1;
        gcra.check_at(now + (ms * i)).unwrap();
    });
}

#[bench]
fn bench_allower(b: &mut test::Bencher) {
    let mut allower = Allower::new();
    b.iter(|| allower.check().unwrap());
}

#[bench]
fn bench_threadsafe_gcra(b: &mut test::Bencher) {
    let mut gcra = GCRA::for_capacity(50).cell_weight(1).build_sync();
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

// This one doesn't seem to actually do a thing & I can't quite figure out why /:
#[bench]
fn bench_multithreading_potentially_buggy(b: &mut test::Bencher) {
    let mut lim = GCRA::for_capacity(50).cell_weight(1).build_sync();
    let now = Instant::now();
    let ms = Duration::from_millis(20);
    let mut children = vec![];

    lim.check_at(now).unwrap();
    for _i in 0..20 {
        let mut lim = lim.clone();
        let mut b = b.clone();
        children.push(thread::spawn(move || {
            let mut i = 0;
            b.iter(|| {
                i += 1;
                lim.check_at(now + (ms * i)).unwrap();
            });
        }));
    }
    for child in children {
        child.join().unwrap();
    }
}
