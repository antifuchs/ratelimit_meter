extern crate ratelimit_meter;

use ratelimit_meter::{GCRA, Decider, Decision};
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
