extern crate ratelimit_meter;

use ratelimit_meter::{GCRA, Decider, Decision, ErrorKind, Error};
use std::time::{Instant, Duration};

#[test]
fn accepts_first_cell() {
    let mut gcra: GCRA = GCRA::for_capacity(5).into();
    assert_eq!(Decision::Yes, gcra.check().unwrap());
}
#[test]
fn rejects_too_many() {
    let mut gcra = GCRA::for_capacity(1).build();
    let now = Instant::now();
    gcra.check_at(now).unwrap();
    gcra.check_at(now).unwrap();
    assert_ne!(Decision::Yes, gcra.check_at(now).unwrap(), "{:?}", gcra);
}
#[test]
fn allows_after_interval() {
    let mut gcra = GCRA::for_capacity(1).build();
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    gcra.check_at(now).unwrap();
    gcra.check_at(now + ms * 1).unwrap();
    gcra.check_at(now + ms * 2).unwrap();
    // should be ok again in 1s:
    let next = now + Duration::from_secs(1);
    assert_eq!(Decision::Yes, gcra.check_at(next).unwrap());
}

#[test]
fn allows_n_after_interval() {
    let mut gcra = GCRA::for_capacity(2).build();
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    assert_eq!(Decision::Yes, gcra.check_n_at(3, now).unwrap());
    assert!(!gcra.check_n_at(2, now+ms*1).unwrap().is_compliant());
    // should be ok again in 1s:
    let next = now + Duration::from_secs(1);
    assert_eq!(Decision::Yes, gcra.check_n_at(2, next).unwrap());
}

#[test]
fn never_allows_more_than_capacity() {
    let mut gcra = GCRA::for_capacity(5).build();
    let now = Instant::now();
    let ms = Duration::from_millis(1);

    // Should not allow the first 15 cells on a capacity 5 bucket:
    assert!(gcra.check_n_at(15, now).is_err());

    // After 3 and 20 seconds, it should not allow 15 on that bucket either:
    assert!(gcra.check_n_at(15, now+(ms*3*1000)).is_err());

    let result = gcra.check_n_at(15, now+(ms*20*1000));
    match result {
        Err(Error(ErrorKind::CapacityError, _)) => (),
        _ => panic!("Did not expect {:?}", result)
    }
}
