extern crate ratelimit_meter;
#[macro_use]
extern crate nonzero_ext;

use ratelimit_meter::{KeyedRateLimiter, NegativeMultiDecision, GCRA};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn accepts_first_cell() {
    let mut gcra = KeyedRateLimiter::<&str>::new(nonzero!(5u32), Duration::from_secs(1));
    assert_eq!(Ok(()), gcra.check("foo"));
}

#[test]
fn rejects_too_many() {
    let mut lim = KeyedRateLimiter::<&str>::new(nonzero!(1u32), Duration::from_secs(1));
    let ms = Duration::from_millis(1);
    let now = Instant::now();
    lim.check_at("foo", now + ms).unwrap();
    assert_ne!(Ok(()), lim.check_at("foo", now + ms * 3), "{:?}", lim);
}

#[test]
fn different_states_per_key() {
    let mut lim = KeyedRateLimiter::<&str>::new(nonzero!(1u32), Duration::from_secs(1));
    let ms = Duration::from_millis(1);
    let now = Instant::now();
    assert_eq!(Ok(()), lim.check_at("foo", now + ms));
    assert_eq!(Ok(()), lim.check_at("bar", now + ms));
    assert_eq!(Ok(()), lim.check_at("baz", now + ms));

    assert_ne!(Ok(()), lim.check_at("foo", now + ms * 3), "{:?}", lim);
    assert_ne!(Ok(()), lim.check_at("bar", now + ms * 3), "{:?}", lim);
    assert_ne!(Ok(()), lim.check_at("baz", now + ms * 3), "{:?}", lim);
}

#[test]
fn allows_after_interval() {
    let mut gcra = KeyedRateLimiter::<&str, GCRA>::new(nonzero!(1u32), Duration::from_secs(1));
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    gcra.check_at("foo", now).unwrap();
    assert_eq!(Ok(()), gcra.check_at("foo", now + ms), "{:?}", gcra);
    assert_ne!(Ok(()), gcra.check_at("foo", now + ms * 2), "{:?}", gcra);
    // should be ok again in 1s:
    let next = now + Duration::from_secs(1) + ms;
    assert_eq!(Ok(()), gcra.check_at("foo", next));
}

#[test]
fn allows_n_after_interval() {
    let mut gcra = KeyedRateLimiter::<&str>::new(nonzero!(2u32), Duration::from_secs(1));
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    assert_eq!(Ok(()), gcra.check_n_at("foo", 2, now));
    assert!(!gcra.check_n_at("foo", 2, now + ms).is_ok());
    // should be ok again in 1.5s:
    let next = now + Duration::from_secs(1);
    assert_eq!(Ok(()), gcra.check_n_at("foo", 2, next), "now: {:?}", next);

    // should always accommodate 0 cells:
    assert_eq!(Ok(()), gcra.check_n_at("foo", 0, next));
}

#[test]
fn never_allows_more_than_capacity() {
    let mut gcra = KeyedRateLimiter::<&str>::new(nonzero!(5u32), Duration::from_secs(1));
    let now = Instant::now();
    let ms = Duration::from_millis(1);

    // Should not allow the first 15 cells on a capacity 5 bucket:
    assert!(gcra.check_n_at("foo", 15, now).is_err());

    // After 3 and 20 seconds, it should not allow 15 on that bucket either:
    assert!(gcra.check_n_at("foo", 15, now + (ms * 3 * 1000)).is_err());

    let result = gcra.check_n_at("foo", 15, now + (ms * 20 * 1000));
    match result {
        Err(NegativeMultiDecision::InsufficientCapacity(n)) => assert_eq!(n, 15),
        _ => panic!("Did not expect {:?}", result),
    }
}

#[test]
fn actual_threadsafety() {
    let mut lim = KeyedRateLimiter::<&str, GCRA>::new(nonzero!(20u32), Duration::from_secs(1));
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    let mut children = vec![];

    lim.check_at("foo", now).unwrap();
    for _i in 0..20 {
        let mut lim = lim.clone();
        children.push(thread::spawn(move || {
            lim.check_at("foo", now).unwrap();
        }));
    }
    for child in children {
        child.join().unwrap();
    }
    assert!(!lim.check_at("foo", now + ms * 2).is_ok());
    assert_eq!(Ok(()), lim.check_at("foo", now + ms * 1000));
}
