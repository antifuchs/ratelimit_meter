extern crate ratelimit_meter;
#[macro_use]
extern crate nonzero_ext;

use ratelimit_meter::{KeyedRateLimiter, GCRA};
use std::thread;
use std::time::{Duration, Instant};

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
fn expiration() {
    let ms = Duration::from_millis(1);
    let now = Instant::now();
    let then = now + ms * 2000; // two seconds later

    fn make_bucket<'a>() -> KeyedRateLimiter<&'a str> {
        let ms = Duration::from_millis(1);
        let now = Instant::now();
        let mut lim = KeyedRateLimiter::<&str>::new(nonzero!(1u32), Duration::from_secs(1));
        lim.check_at("foo", now).unwrap();
        lim.check_at("bar", now + ms * 200).unwrap();
        lim.check_at("baz", now + ms * 800).unwrap();
        lim
    }

    // clean up all keys that are indistinguishable from unoccupied keys:
    let mut lim = make_bucket();
    let mut removed = lim.cleanup_at(None, then);
    removed.sort();
    assert_eq!(vec!["bar", "baz", "foo"], removed);

    // clean up all keys that have been so for 300ms:
    let mut lim = make_bucket();
    let mut removed = lim.cleanup_at(Some(Duration::from_millis(300)), then);
    removed.sort();
    assert_eq!(vec!["bar", "foo"], removed);

    // clean up 2 seconds plus change later:
    let mut lim = make_bucket();
    let mut removed = lim.cleanup_at(Some(Duration::from_secs(1)), now + ms * 2100);
    removed.sort();
    assert_eq!(vec!["foo"], removed);
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
