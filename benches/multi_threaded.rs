#![feature(test)]

extern crate test;
extern crate ratelimit_meter;

use ratelimit_meter::{GCRA, LeakyBucket, Decider};
use std::time::{Instant, Duration};
use std::thread;


#[bench]
fn bench_gcra_20threads(b: &mut test::Bencher) {
    let mut lim = GCRA::for_capacity(50).unwrap().cell_weight(1).unwrap().build_sync();
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
                lim.check_at(now + (ms * i)).unwrap();
            });
        }));
    }
    let mut i = 0;
    b.iter(|| {
        i += 1;
        lim.check_at(now + (ms * i)).unwrap();
    });
    for child in children {
        child.join().unwrap();
    }
}

#[bench]
fn bench_leaky_bucket_20threads(b: &mut test::Bencher) {
    let mut lim = LeakyBucket::per_second(50).unwrap().threadsafe();
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
                lim.check_at(now + (ms * i)).unwrap();
            });
        }));
    }
    let mut i = 0;
    b.iter(|| {
        i += 1;
        lim.check_at(now + (ms * i)).unwrap();
    });
    for child in children {
        child.join().unwrap();
    }
}
