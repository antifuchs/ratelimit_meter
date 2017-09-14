extern crate ratelimit_meter;

use ratelimit_meter::{LeakyBucket, MultiDecider, Decider, Decision, ErrorKind, Error};
use std::time::{Instant, Duration};

#[test]
fn accepts_first_cell() {
    let mut lb: LeakyBucket = LeakyBucket::per_second(5).unwrap();
    assert_eq!(Decision::Yes, lb.check().unwrap());
}

#[test]
fn rejects_too_many() {
    let mut lb: LeakyBucket = LeakyBucket::per_second(2).unwrap();
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    assert_eq!(Decision::Yes, lb.check_at(now).unwrap());
    assert_eq!(Decision::Yes, lb.check_at(now).unwrap());
    assert!(!lb.check_at(now + ms * 2).unwrap().is_compliant());
    // should be ok again in 1s:
    let next = now + Duration::from_millis(1002);
    assert_eq!(Decision::Yes, lb.check_at(next).unwrap());
    assert_eq!(Decision::Yes, lb.check_at(next + ms).unwrap());
    assert!(
        !lb.check_at(next + ms * 2).unwrap().is_compliant(),
        "{:?}",
        lb
    );
}

#[test]
fn never_allows_more_than_capacity() {
    let mut lb = LeakyBucket::per_second(5).unwrap();
    let now = Instant::now();
    let ms = Duration::from_millis(1);

    // Should not allow the first 15 cells on a capacity 5 bucket:
    assert!(lb.check_n_at(15, now).is_err());

    // After 3 and 20 seconds, it should not allow 15 on that bucket either:
    assert!(lb.check_n_at(15, now + (ms * 3 * 1000)).is_err());

    let result = lb.check_n_at(15, now + (ms * 20 * 1000));
    match result {
        Err(Error(ErrorKind::InsufficientCapacity(n), _)) => assert_eq!(n, 15),
        _ => panic!("Did not expect {:?}", result),
    }
}

#[test]
fn correct_wait_time() {
    // Bucket adding a new element per 200ms:
    let mut lb = LeakyBucket::per_second(5).unwrap();
    let mut now = Instant::now();
    let ms = Duration::from_millis(1);
    let mut compliant = 0;
    for _i in 0..20 {
        now += ms;
        let res = lb.check_at(now);
        match res {
            Ok(Decision::Yes) => {
                compliant += 1;
            }
            Ok(Decision::No(wait)) => {
                now += wait;
                assert!(lb.check_at(now).unwrap().is_compliant());
                compliant += 1;
            }
            _ => panic!("Unexpected result {:?}", res),
        }
    }
    assert_eq!(20, compliant);
}

#[test]
fn prevents_time_travel() {
    let mut lb = LeakyBucket::per_second(5).unwrap();
    let now = Instant::now();
    let ms = Duration::from_millis(1);

    assert!(lb.check_at(now).unwrap().is_compliant());
    assert!(lb.check_at(now - ms).unwrap().is_compliant());
    assert!(lb.check_at(now - ms * 500).unwrap().is_compliant());
}
