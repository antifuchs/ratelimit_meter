extern crate ratelimit_meter;

use ratelimit_meter::{Decider, MultiDecider, NegativeMultiDecision, GCRA};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn accepts_first_cell() {
    let mut gcra: GCRA = GCRA::for_capacity(5).unwrap().into();
    assert_eq!(Ok(()), gcra.check());
}
#[test]
fn rejects_too_many() {
    let mut gcra = GCRA::for_capacity(1).unwrap().build();
    let now = Instant::now();
    gcra.check_at(now).unwrap();
    gcra.check_at(now).unwrap();
    assert_ne!(Ok(()), gcra.check_at(now), "{:?}", gcra);
}
#[test]
fn allows_after_interval() {
    let mut gcra = GCRA::for_capacity(1).unwrap().build();
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    gcra.check_at(now).unwrap();
    assert_eq!(Ok(()), gcra.check_at(now + ms));
    assert!(gcra.check_at(now + ms * 2).is_err());
    // should be ok again in 1s:
    let next = now + Duration::from_secs(1);
    assert_eq!(Ok(()), gcra.check_at(next));
}

#[test]
fn allows_n_after_interval() {
    let mut gcra = GCRA::for_capacity(2).unwrap().build();
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    assert_eq!(Ok(()), gcra.check_n_at(2, now));
    assert!(!gcra.check_n_at(2, now + ms).is_ok());
    // should be ok again in 1.5s:
    let next = now + Duration::from_secs(1);
    assert_eq!(Ok(()), gcra.check_n_at(2, next), "now: {:?}", next);

    // should always accommodate 0 cells:
    assert_eq!(Ok(()), gcra.check_n_at(0, next));
}

#[test]
fn never_allows_more_than_capacity() {
    let mut gcra = GCRA::for_capacity(5).unwrap().build();
    let now = Instant::now();
    let ms = Duration::from_millis(1);

    // Should not allow the first 15 cells on a capacity 5 bucket:
    assert!(gcra.check_n_at(15, now).is_err());

    // After 3 and 20 seconds, it should not allow 15 on that bucket either:
    assert!(gcra.check_n_at(15, now + (ms * 3 * 1000)).is_err());

    let result = gcra.check_n_at(15, now + (ms * 20 * 1000));
    match result {
        Err(NegativeMultiDecision::InsufficientCapacity(n)) => assert_eq!(n, 15),
        _ => panic!("Did not expect {:?}", result),
    }
}

#[test]
fn actual_threadsafety() {
    let mut lim = GCRA::for_capacity(20).unwrap().build();
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    let mut children = vec![];

    lim.check_at(now).unwrap();
    for _i in 0..20 {
        let mut lim = lim.clone();
        children.push(thread::spawn(move || {
            lim.check_at(now).unwrap();
        }));
    }
    for child in children {
        child.join().unwrap();
    }
    assert!(!lim.check_at(now + ms * 2).is_ok());
    assert_eq!(Ok(()), lim.check_at(now + ms * 1000));
}
