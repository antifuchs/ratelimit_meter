extern crate ratelimit_meter;

use ratelimit_meter::{GCRA, Threadsafe, Limiter, Decider, Decision};
use std::thread;

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
    let lim = Limiter::new()
        .capacity(5)
        .weight(1)
        .build::<Threadsafe<GCRA>>()
        .unwrap();
    let mut children = vec![];
    for _i in 0..20 {
        let mut lim = lim.clone();
        children.push(thread::spawn(move || {
            lim.check();
        }));
    }
    for child in children {
        child.join().unwrap();
    }
}
