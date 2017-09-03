#![feature(test)]

extern crate test;
extern crate ratelimit_meter;

use ratelimit_meter::{GCRA, Threadsafe, Limiter, Decider};
use ratelimit_meter::example_algorithms::Allower;
use std::time::{Instant, Duration};
use std::thread;

#[bench]
fn bench_gcra(b: &mut test::Bencher) {
    let mut gcra = Limiter::new().capacity(50).weight(1).build::<GCRA>().unwrap();
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
    let mut allower = Limiter::new().capacity(50).weight(1).build::<Allower>().unwrap();
    b.iter(|| allower.check().unwrap());
}

#[bench]
fn bench_threadsafe_gcra(b: &mut test::Bencher) {
    let mut gcra = Limiter::new().capacity(50).weight(1).build::<Threadsafe<GCRA>>().unwrap();
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
    let mut allower = Limiter::new().capacity(50).weight(1).build::<Threadsafe<Allower>>().unwrap();
    b.iter(|| allower.check());
}

// This one doesn't seem to actually do a thing & I can't quite figure out why /:
#[bench]
fn bench_multithreading_potentially_buggy(b: &mut test::Bencher) {
    let mut lim = Limiter::new()
        .capacity(20)
        .weight(1)
        .build::<Threadsafe<GCRA>>()
        .unwrap();
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
