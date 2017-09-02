extern crate ratelimit_meter;

use ratelimit_meter::{GCRA, Threadsafe, Limiter, Decider, Decision};
use std::thread;
use std::time::{Instant, Duration};

#[test]
fn simple_operation() {
    let mut lim = Limiter::new()
        .capacity(5)
        .weight(1)
        .build::<Threadsafe<GCRA>>()
        .unwrap();
    assert_eq!(Decision::Yes, lim.check());
}

#[test]
fn actual_threadsafety() {
    let mut lim = Limiter::new()
        .capacity(20)
        .weight(1)
        .build::<Threadsafe<GCRA>>()
        .unwrap();
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    let mut children = vec![];

    lim.test_and_update(now);
    for _i in 0..20 {
        let mut lim = lim.clone();
        children.push(thread::spawn(move || {
            lim.test_and_update(now);
        }));
    }
    for child in children {
        child.join().unwrap();
    }
    assert!(!lim.test_and_update(now).is_compliant());
    assert_eq!(Decision::Yes, lim.test_and_update(now+ms*1000));
}
