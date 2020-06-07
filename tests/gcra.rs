extern crate ratelimit_meter;
#[macro_use]
extern crate nonzero_ext;

use ratelimit_meter::{
    algorithms::Algorithm, test_utilities::current_moment, NegativeMultiDecision, NonConformance,
    GCRA,
};
use std::thread;
use std::time::Duration;

#[test]
fn accepts_first_cell() {
    let gcra = GCRA::construct(nonzero!(5u32), nonzero!(1u32), Duration::from_secs(1)).unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();
    let now = current_moment();
    assert_eq!(Ok(()), gcra.test_and_update(&state, now));
}

#[test]
fn rejects_too_many() {
    let gcra = GCRA::construct(nonzero!(1u32), nonzero!(1u32), Duration::from_secs(1)).unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();
    let now = current_moment();
    gcra.test_and_update(&state, now).unwrap();
    gcra.test_and_update(&state, now).unwrap();
    assert_ne!(
        Ok(()),
        gcra.test_and_update(&state, now),
        "{:?} {:?}",
        &state,
        &gcra
    );
}

#[test]
fn allows_after_interval() {
    let gcra = GCRA::construct(nonzero!(1u32), nonzero!(1u32), Duration::from_secs(1)).unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();
    let now = current_moment();
    let ms = Duration::from_millis(1);
    gcra.test_and_update(&state, now).unwrap();
    assert_eq!(Ok(()), gcra.test_and_update(&state, now + ms));
    assert_ne!(Ok(()), gcra.test_and_update(&state, now + ms * 2));
    // should be ok again in 1s:
    let next = now + Duration::from_secs(1);
    assert_eq!(Ok(()), gcra.test_and_update(&state, next));
}

#[test]
fn allows_n_after_interval() {
    let gcra = GCRA::construct(nonzero!(2u32), nonzero!(1u32), Duration::from_secs(1)).unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();
    let now = current_moment() + Duration::from_secs(1);
    let ms = Duration::from_millis(1);
    assert_eq!(Ok(()), gcra.test_n_and_update(&state, 2, now));
    assert!(!gcra.test_n_and_update(&state, 2, now + ms).is_ok());
    // should be ok again in 1.5s:
    let next = now + Duration::from_secs(1);
    assert_eq!(
        Ok(()),
        gcra.test_n_and_update(&state, 2, next),
        "now: {:?}",
        next
    );

    // should always accommodate 0 cells:
    assert_eq!(Ok(()), gcra.test_n_and_update(&state, 0, next));
}

#[test]
fn correctly_handles_per() {
    let ms = Duration::from_millis(1);
    let gcra = GCRA::construct(nonzero!(1u32), nonzero!(1u32), ms * 20).unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();
    let now = current_moment();

    assert_eq!(Ok(()), gcra.test_and_update(&state, now));
    assert_eq!(Ok(()), gcra.test_and_update(&state, now + ms));
    assert!(!gcra.test_and_update(&state, now + ms * 10).is_ok());
    assert_eq!(Ok(()), gcra.test_and_update(&state, now + ms * 20));
}

#[test]
fn never_allows_more_than_capacity() {
    let ms = Duration::from_millis(1);
    let gcra = GCRA::construct(nonzero!(5u32), nonzero!(1u32), Duration::from_secs(1)).unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();
    let now = current_moment();

    // Should not allow the first 15 cells on a capacity 5 bucket:
    assert!(gcra.test_n_and_update(&state, 15, now).is_err());

    // After 3 and 20 seconds, it should not allow 15 on that bucket either:
    assert!(gcra
        .test_n_and_update(&state, 15, now + (ms * 3 * 1000))
        .is_err());

    let result = gcra.test_n_and_update(&state, 15, now + (ms * 20 * 1000));
    match result {
        Err(NegativeMultiDecision::InsufficientCapacity(n)) => assert_eq!(n, 15),
        _ => panic!("Did not expect {:?}", result),
    }
}

#[test]
fn correct_wait_time() {
    // Bucket adding a new element per 200ms:
    let gcra = GCRA::construct(nonzero!(5u32), nonzero!(1u32), Duration::from_secs(1)).unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();
    let mut now = current_moment();
    let ms = Duration::from_millis(1);
    let mut conforming = 0;
    for _i in 0..20 {
        now += ms;
        let res = gcra.test_and_update(&state, now);
        match res {
            Ok(()) => {
                conforming += 1;
            }
            Err(wait) => {
                now += wait.wait_time_from(now);
                assert_eq!(Ok(()), gcra.test_and_update(&state, now));
                conforming += 1;
            }
        }
    }
    assert_eq!(20, conforming);
}

#[test]
fn actual_threadsafety() {
    let gcra = GCRA::construct(nonzero!(20u32), nonzero!(1u32), Duration::from_secs(1))
        .expect("can't build GCRA");
    let state = <GCRA as Algorithm>::BucketState::default();

    let now = current_moment() + Duration::from_secs(1);
    let ms = Duration::from_millis(1);
    let mut children = vec![];

    gcra.test_and_update(&state, now)
        .expect("first update should work");
    for _i in 0..20 {
        let state = state.clone();
        let gcra = gcra.clone();
        children.push(thread::spawn(move || gcra.test_and_update(&state, now)));
    }
    let results: Vec<Result<(), <ratelimit_meter::GCRA as Algorithm>::NegativeDecision>> = children
        .into_iter()
        .enumerate()
        .map(|(n, c)| c.join().expect(&format!("thread {} panicked", n)))
        .collect();
    let expected: Vec<Result<(), <ratelimit_meter::GCRA as Algorithm>::NegativeDecision>> =
        results.iter().map(|_| Ok(())).collect();
    assert_eq!(expected, results);

    assert_ne!(Ok(()), gcra.test_and_update(&state, now + ms * 2));
    assert_eq!(Ok(()), gcra.test_and_update(&state, now + ms * 1000));
}

#[test]
fn nonconformance_wait_time_from() {
    let gcra = GCRA::construct(nonzero!(1u32), nonzero!(1u32), Duration::from_secs(1)).unwrap();
    let state = <GCRA as Algorithm>::BucketState::default();
    let now = current_moment();
    let ms = Duration::from_millis(1);
    gcra.test_and_update(&state, now).unwrap();
    gcra.test_and_update(&state, now).unwrap();
    if let Err(failure) = gcra.test_and_update(&state, now) {
        assert_eq!(ms * 2000, failure.wait_time_from(now));
        assert_eq!(Duration::new(0, 0), failure.wait_time_from(now + ms * 2000));
        assert_eq!(Duration::new(0, 0), failure.wait_time_from(now + ms * 2001));
    } else {
        assert!(false, "Second attempt should fail");
    }
}
