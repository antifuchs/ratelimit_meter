extern crate ratelimit_meter;

use ratelimit_meter::{GCRA, Limiter, Decider, Decision};
use std::time::{Instant, Duration};

#[test]
fn accepts_first_cell() {
    let mut gcra = Limiter::new().capacity(5).weight(1).build::<GCRA>().unwrap();
    assert_eq!(Decision::Yes, gcra.check().unwrap());
}
#[test]
fn rejects_too_many() {
    let mut gcra = Limiter::new().capacity(1).weight(1).build::<GCRA>().unwrap();
    let now = Instant::now();
    gcra.test_and_update(now).unwrap();
    gcra.test_and_update(now).unwrap();
    assert_ne!(Decision::Yes, gcra.test_and_update(now).unwrap());
}
#[test]
fn allows_after_interval() {
    let mut gcra = Limiter::new().capacity(1).weight(1).build::<GCRA>().unwrap();
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    gcra.test_and_update(now).unwrap();
    gcra.test_and_update(now + ms * 1).unwrap();
    gcra.test_and_update(now + ms * 2).unwrap();
    // should be ok again in 1s:
    let next = now + Duration::from_secs(1);
    assert_eq!(Decision::Yes, gcra.test_and_update(next).unwrap());
}
