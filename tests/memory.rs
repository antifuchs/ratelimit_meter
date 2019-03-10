#![cfg(feature = "std")]

// This test uses procinfo, so can only be run on Linux.
extern crate libc;
extern crate ratelimit_meter;
#[macro_use]
extern crate nonzero_ext;

use ratelimit_meter::{DirectRateLimiter, LeakyBucket, GCRA};
use std::thread;

fn resident_memsize() -> i64 {
    let mut out: libc::rusage = unsafe { std::mem::zeroed() };
    assert!(unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut out) } == 0);
    out.ru_maxrss
}

const LEAK_TOLERANCE: i64 = 1024 * 1024 * 10;

struct LeakCheck {
    usage_before: i64,
    n_iter: usize,
}

impl Drop for LeakCheck {
    fn drop(&mut self) {
        let usage_after = resident_memsize();
        assert!(
            usage_after <= self.usage_before + LEAK_TOLERANCE,
            "Plausible memory leak!\nAfter {} iterations, usage before: {}, usage after: {}",
            self.n_iter,
            self.usage_before,
            usage_after
        );
    }
}

impl LeakCheck {
    fn new(n_iter: usize) -> Self {
        LeakCheck {
            n_iter,
            usage_before: resident_memsize(),
        }
    }
}

#[test]
fn memleak_gcra() {
    let mut bucket = DirectRateLimiter::<GCRA>::build_with_capacity(nonzero!(1_000_000u32))
        .build()
        .unwrap();
    let leak_check = LeakCheck::new(500_000);

    for _i in 0..leak_check.n_iter {
        drop(bucket.check());
    }
}

#[test]
fn memleak_gcra_multi() {
    let mut bucket = DirectRateLimiter::<GCRA>::build_with_capacity(nonzero!(1_000_000u32))
        .build()
        .unwrap();
    let leak_check = LeakCheck::new(500_000);

    for _i in 0..leak_check.n_iter {
        drop(bucket.check_n(2));
    }
}

#[test]
fn memleak_gcra_threaded() {
    let bucket = DirectRateLimiter::<GCRA>::build_with_capacity(nonzero!(1_000_000u32))
        .build()
        .unwrap();
    let leak_check = LeakCheck::new(5_000);

    for _i in 0..leak_check.n_iter {
        let mut bucket = bucket.clone();
        thread::spawn(move || drop(bucket.check())).join().unwrap();
    }
}

#[test]
fn memleak_leakybucket() {
    let mut bucket = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(1_000_000u32));
    let leak_check = LeakCheck::new(500_000);

    for _i in 0..leak_check.n_iter {
        drop(bucket.check());
    }
}

#[test]
fn memleak_leakybucket_threaded() {
    let bucket = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(1_000_000u32));
    let leak_check = LeakCheck::new(5_000);

    for _i in 0..leak_check.n_iter {
        let mut bucket = bucket.clone();
        thread::spawn(move || drop(bucket.check())).join().unwrap();
    }
}
