extern crate ratelimit_meter;

#[allow(deprecated)]
use ratelimit_meter::{Threadsafe, GCRA, Decider, Decision};
use std::thread;
use std::time::{Instant, Duration};

#[allow(deprecated)]
#[test]
fn simple_operation() {
    let mut lim = Threadsafe::new(GCRA::for_capacity(5).unwrap().build());
    assert_eq!(Decision::Yes, lim.check().unwrap());
}

#[allow(deprecated)]
#[test]
fn actual_threadsafety() {
    let mut lim = Threadsafe::new(GCRA::for_capacity(20).unwrap().build());
    let now = Instant::now();
    let ms = Duration::from_millis(1);
    let mut children = vec![];

    lim.check_at(now).unwrap();
    for _i in 0..20 {
        let mut lim = lim.clone();
        children.push(thread::spawn(move || { lim.check_at(now).unwrap(); }));
    }
    for child in children {
        child.join().unwrap();
    }
    assert!(!lim.check_at(now + ms * 2).unwrap().is_compliant());
    assert_eq!(Decision::Yes, lim.check_at(now + ms * 1000).unwrap());
}
