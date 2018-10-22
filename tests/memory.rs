// This test uses procinfo, so can only be run on Linux.
extern crate libc;
extern crate ratelimit_meter;
#[macro_use]extern crate nonzero_ext;

use ratelimit_meter::{DirectRateLimiter, LeakyBucket, GCRA};
use std::thread;

fn resident_memsize() -> i64 {
    let mut out: libc::rusage = unsafe { std::mem::zeroed() };
    assert!(unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut out) } == 0);
    out.ru_maxrss
}

const LEAK_TOLERANCE: i64 = 1024 * 1024 * 10;

fn check_for_leaks(n_iter: usize, usage_before: i64) {
    let usage_after = resident_memsize();
    assert!(
        usage_after <= usage_before + LEAK_TOLERANCE,
        "Plausible memory leak!\nAfter {} iterations, usage before: {}, usage after: {}",
        n_iter,
        usage_before,
        usage_after
    );
}

#[test]
fn memleak_gcra() {
    const N_ITER: usize = 500_000;
    let mut bucket =
        DirectRateLimiter::<GCRA>::build_with_capacity(nonzero!(1_000_000u32))
            .build()
            .unwrap();
    let usage_before = resident_memsize();

    for _i in 0..N_ITER {
        drop(bucket.check());
    }
    check_for_leaks(N_ITER, usage_before);
}

#[test]
fn memleak_gcra_multi() {
    const N_ITER: usize = 500_000;
    let mut bucket =
        DirectRateLimiter::<GCRA>::build_with_capacity(nonzero!(1_000_000u32))
            .build()
            .unwrap();
    let usage_before = resident_memsize();

    for _i in 0..N_ITER {
        drop(bucket.check_n(2));
    }
    check_for_leaks(N_ITER, usage_before);
}

#[test]
fn memleak_gcra_threaded() {
    const N_ITER: usize = 5_000;
    let bucket =
        DirectRateLimiter::<GCRA>::build_with_capacity(nonzero!(1_000_000u32))
            .build()
            .unwrap();
    let usage_before = resident_memsize();

    for _i in 0..N_ITER {
        let mut bucket = bucket.clone();
        thread::spawn(move || drop(bucket.check())).join().unwrap();
    }
    check_for_leaks(N_ITER, usage_before);
}

#[test]
fn memleak_leakybucket() {
    const N_ITER: usize = 500_000;
    let mut bucket =
        DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(1_000_000u32));
    let usage_before = resident_memsize();

    for _i in 0..N_ITER {
        drop(bucket.check());
    }
    check_for_leaks(N_ITER, usage_before);
}

#[test]
fn memleak_leakybucket_threaded() {
    const N_ITER: usize = 5_000;
    let bucket = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(1_000_000u32));
    let usage_before = resident_memsize();

    for _i in 0..N_ITER {
        let mut bucket = bucket.clone();
        thread::spawn(move || drop(bucket.check())).join().unwrap();
    }
    check_for_leaks(N_ITER, usage_before);
}
