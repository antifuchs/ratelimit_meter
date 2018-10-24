extern crate ratelimit_meter;
#[macro_use]
extern crate nonzero_ext;

use ratelimit_meter::{DirectRateLimiter, LeakyBucket, NegativeMultiDecision, NonConformance};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn accepts_first_cell() {
    let mut lb = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(5u32));
    assert_eq!(Ok(()), lb.check());
}

#[test]
fn rejects_too_many() {
    let mut lb = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(2u32));
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    assert_eq!(Ok(()), lb.check_at(now));
    assert_eq!(Ok(()), lb.check_at(now));
    assert!(!lb.check_at(now + ms * 2).is_ok());
    // should be ok again in 1s:
    let next = now + Duration::from_millis(1002);
    assert_eq!(Ok(()), lb.check_at(next));
    assert_eq!(Ok(()), lb.check_at(next + ms));
    assert!(!lb.check_at(next + ms * 2).is_ok(), "{:?}", lb);
}

#[test]
fn never_allows_more_than_capacity() {
    let mut lb = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(5u32));
    let now = Instant::now();
    let ms = Duration::from_millis(1);

    // Should not allow the first 15 cells on a capacity 5 bucket:
    assert!(lb.check_n_at(15, now).is_err());

    // After 3 and 20 seconds, it should not allow 15 on that bucket either:
    assert!(lb.check_n_at(15, now + (ms * 3 * 1000)).is_err());

    let result = lb.check_n_at(15, now + (ms * 20 * 1000));
    match result {
        Err(NegativeMultiDecision::InsufficientCapacity(n)) => assert_eq!(n, 15),
        _ => panic!("Did not expect {:?}", result),
    }
}

#[test]
fn correct_wait_time() {
    // Bucket adding a new element per 200ms:
    let mut lb = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(5u32));
    let mut now = Instant::now();
    let ms = Duration::from_millis(1);
    let mut conforming = 0;
    for _i in 0..20 {
        now += ms;
        let res = lb.check_at(now);
        match res {
            Ok(()) => {
                conforming += 1;
            }
            Err(wait) => {
                now += wait.wait_time_from(now);
                assert_eq!(Ok(()), lb.check_at(now));
                conforming += 1;
            }
        }
    }
    assert_eq!(20, conforming);
}

#[test]
fn prevents_time_travel() {
    let mut lb = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(5u32));
    let now = Instant::now();
    let ms = Duration::from_millis(1);

    assert!(lb.check_at(now).is_ok());
    assert!(lb.check_at(now - ms).is_ok());
    assert!(lb.check_at(now - ms * 500).is_ok());
}

#[test]
fn actual_threadsafety() {
    let mut lim = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(20u32));
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    let mut children = vec![];

    lim.check_at(now).unwrap();
    for _i in 0..20 {
        let mut lim = lim.clone();
        children.push(thread::spawn(move || lim.check_at(now).is_ok()));
    }
    for child in children {
        child.join().unwrap();
    }
    assert!(!lim.check_at(now + ms * 2).is_ok());
    assert_eq!(Ok(()), lim.check_at(now + ms * 1000));
}
