use parking_lot::Mutex;
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use evmap::{self, ReadHandle, WriteHandle};

use {
    algorithms::{Algorithm, RateLimitState},
    InconsistentCapacity, NegativeMultiDecision, NonConformance,
};

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

    fn check_and_update_key<E, F>(&self, key: K, update: F) -> Result<(), E>
    where
        F: Fn(&A::BucketState) -> Result<(), E>,
    {
        self.map_reader
            .get_and(&key, |v| {
                // we have at least one element (owing to the nature of
                // the evmap, it says there could be >1
                // entries, but we'll only ever add one):
                let state = v.get(0).unwrap();
                update(state)
            }).unwrap_or_else(|| {
                // entry does not exist, let's add one.
                let mut w = self.map_writer.lock();
                let state: A::BucketState = Default::default();
                let result = update(&state);
                w.update(key, state);
                w.flush();
                result
            })
    }

    pub fn check_at(&mut self, key: K, at: Instant) -> Result<(), NonConformance> {
        self.check_and_update_key(key, |state| {
            <A as Algorithm>::test_and_update(state, &self.params, at)
        })
    }

    pub fn check_n_at(&mut self, key: K, n: u32, at: Instant) -> Result<(), NegativeMultiDecision> {
        self.check_and_update_key(key, |state| {
            <A as Algorithm>::test_n_and_update(state, &self.params, n, at)
        })
    }

    pub fn check(&mut self, key: K) -> Result<(), NonConformance> {
        self.check_at(key, Instant::now())
    }

    pub fn check_n(&mut self, key: K, n: u32) -> Result<(), NegativeMultiDecision> {
        self.check_n_at(key, n, Instant::now())
    }

    /// Removes the keys from this rate limiter that can be expired safely.
    ///
    /// To be eligible for expiration, a key's rate limiter state must
    /// be at least `min_age` past its last relevance (see
    /// [`RateLimitState.last_touched`](trait.RateLimitState.html#method.last_touched)).
    ///
    /// This method works in two parts, but both parts block new keys
    /// from getting added while they're running:
    /// * First, it collects the keys that are eligible for expiration.
    /// * Then, it expires these keys.
    ///
    /// # Race conditions
    /// Since this is happening concurrently with other operations,
    /// race conditions can & will occur. It's possible that cells are
    /// accounted between the time `cleanup_at` is called and their
    /// expiry.  These cells will lost.
    ///
    /// The time window in which this can occur is hopefully short
    /// enough that this is an acceptable risk of loss in accuracy.
    pub fn cleanup_at<D: Into<Option<Duration>>, I: Into<Option<Instant>>>(
        &mut self,
        min_age: D,
        at: I,
    ) {
        let params = &self.params;
        let min_age = min_age.into().unwrap_or_else(|| Duration::new(0, 0));
        let at = at.into().unwrap_or_else(|| Instant::now());

        let mut expireable: Vec<K> = vec![];
        self.map_reader.for_each(|k, v| {
            v.get(0).map(|state| {
                if state.last_touched(params) < at - min_age {
                    expireable.push(k.clone());
                }
            });
        });

        // Now take the map write lock and remove all the keys that we
        // collected:
        let mut w = self.map_writer.lock();
        for key in expireable {
            w.empty(key);
        }
        w.refresh();
    }
}

// TODO: add a builder for this one
