extern crate ratelimit_meter;
#[macro_use]
extern crate nonzero_ext;

use ratelimit_meter::jitter::Jitter;
use ratelimit_meter::{
    algorithms::Algorithm, prelude::*, test_utilities::current_moment, DirectRateLimiter,
    LeakyBucket, NegativeMultiDecision, NonConformance,
};
use std::thread;
use std::time::Duration;

#[test]
fn accepts_first_cell() {
    let mut lb = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(5u32));
    assert_eq!(Ok(()), lb.check_at(current_moment()));
}

#[test]
fn rejects_too_many() {
    let mut lb = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(2u32));
    let now = current_moment();
    let ms = Duration::from_millis(1);
    assert_eq!(Ok(()), lb.check_at(now));
    assert_eq!(Ok(()), lb.check_at(now));

    assert_ne!(Ok(()), lb.check_at(now + ms * 2));

    // should be ok again in 1s:
    let next = now + Duration::from_millis(1002);
    assert_eq!(Ok(()), lb.check_at(next));
    assert_eq!(Ok(()), lb.check_at(next + ms));

    assert_ne!(Ok(()), lb.check_at(next + ms * 2), "{:?}", lb);
}

#[test]
fn never_allows_more_than_capacity() {
    let mut lb = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(5u32));
    let now = current_moment();
    let ms = Duration::from_millis(1);

    // Should not allow the first 15 cells on a capacity 5 bucket:
    assert_ne!(Ok(()), lb.check_n_at(15, now));

    // After 3 and 20 seconds, it should not allow 15 on that bucket either:
    assert_ne!(Ok(()), lb.check_n_at(15, now + (ms * 3 * 1000)));
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
    let mut now = current_moment();
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
    let now = current_moment() + Duration::from_secs(1);
    let ms = Duration::from_millis(1);

    assert!(lb.check_at(now).is_ok());
    assert!(lb.check_at(now - ms).is_ok());
    assert!(lb.check_at(now - ms * 500).is_ok());
}

#[test]
fn actual_threadsafety() {
    let mut lim = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(20u32));
    let now = current_moment();
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

#[test]
fn tooearly_wait_time_from() {
    let lim =
        LeakyBucket::construct(nonzero!(1u32), nonzero!(1u32), Duration::from_secs(1)).unwrap();
    let state = <LeakyBucket as Algorithm>::BucketState::default();
    let now = current_moment();
    let ms = Duration::from_millis(1);
    lim.test_and_update(&state, now).unwrap();
    if let Err(failure) = lim.test_and_update(&state, now) {
        assert_eq!(ms * 1000, failure.wait_time_from(now));
        assert_eq!(Duration::new(0, 0), failure.wait_time_from(now + ms * 1000));
        assert_eq!(Duration::new(0, 0), failure.wait_time_from(now + ms * 2001));
    } else {
        assert!(false, "Second attempt should fail");
    }
}

#[test]
fn applies_jitter() {
    let mut lim = DirectRateLimiter::<LeakyBucket>::per_second(nonzero!(20u32));
    let now = current_moment();

    let j = Jitter::up_to(Duration::from_secs(1));
    lim.check_at(now).jitter(&j).unwrap();
    lim.check_n_at(1, now).jitter(&j).unwrap();
}
