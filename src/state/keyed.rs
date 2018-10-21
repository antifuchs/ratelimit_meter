use parking_lot::Mutex;
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use evmap::{self, ReadHandle, WriteHandle};

use {algorithms::Algorithm, InconsistentCapacity, NegativeMultiDecision, NonConformance};

#[derive(Clone)]
pub struct KeyedRateLimiter<A: Algorithm, K: Eq + Hash + Clone> {
    algorithm: PhantomData<A>,
    params: A::BucketParams,
    map_reader: ReadHandle<K, A::BucketState>,
    map_writer: Arc<Mutex<WriteHandle<K, A::BucketState>>>,
}

impl<A, K> fmt::Debug for KeyedRateLimiter<A, K>
where
    A: Algorithm,
    K: Eq + Hash + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "KeyedRateLimiter{{{params:?}}}", params = self.params)
    }
}

impl<A, K> KeyedRateLimiter<A, K>
where
    A: Algorithm,
    K: Eq + Hash + Clone,
{
    pub fn new(capacity: NonZeroU32, per_time_unit: Duration) -> Self {
        let (r, mut w): (
            ReadHandle<K, A::BucketState>,
            WriteHandle<K, A::BucketState>,
        ) = evmap::new();
        w.refresh();

        KeyedRateLimiter {
            algorithm: PhantomData,
            params: <A as Algorithm>::params_from_constructor(
                capacity,
                NonZeroU32::new(1).unwrap(),
                per_time_unit,
            ).unwrap(),
            map_reader: r,
            map_writer: Arc::new(Mutex::new(w)),
        }
    }

    pub fn check_at(&mut self, key: K, at: Instant) -> Result<(), NonConformance> {
        self.map_reader
            .get_and(&key, |v| {
                // we have at least one element (owing to the nature of
                // the evmap, it says there could be >1
                // entries, but we'll only ever add one):
                let state = v.get(0).unwrap().clone();
                <A as Algorithm>::test_and_update(state, &self.params, at)
            }).unwrap_or_else(|| {
                // entry does not exist, let's add one.
                let mut w = self.map_writer.lock();
                let state: A::BucketState = Default::default();
                let result = <A as Algorithm>::test_and_update(&state, &self.params, at);
                w.update(key, state);
                w.flush();
                result
            })
    }

    pub fn check_n_at(&mut self, key: K, n: u32, at: Instant) -> Result<(), NegativeMultiDecision> {
        self.map_reader
            .get_and(&key, |v| {
                // we have at least one element (owing to the nature of
                // the evmap, it says there could be >1
                // entries, but we'll only ever add one):
                let state = v.get(0).unwrap().clone();
                <A as Algorithm>::test_n_and_update(state, &self.params, n, at)
            }).unwrap_or_else(|| {
                // entry does not exist, let's add one.
                let mut w = self.map_writer.lock();
                let state: A::BucketState = Default::default();
                let result = <A as Algorithm>::test_n_and_update(&state, &self.params, n, at);
                w.update(key, state);
                w.flush();
                result
            })
    }

    pub fn check(&mut self, key: K) -> Result<(), NonConformance> {
        self.check_at(key, Instant::now())
    }
}

// TODO: add a builder for this one
