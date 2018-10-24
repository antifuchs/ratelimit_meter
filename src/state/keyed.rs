use parking_lot::Mutex;
use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::BuildHasher;
use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use evmap::{self, ReadHandle, WriteHandle};

use {
    algorithms::{Algorithm, DefaultAlgorithm, RateLimitState},
    InconsistentCapacity, NegativeMultiDecision,
};

type MapWriteHandle<K, A, H> = Arc<Mutex<WriteHandle<K, <A as Algorithm>::BucketState, (), H>>>;

#[derive(Clone)]
pub struct KeyedRateLimiter<
    K: Eq + Hash + Clone,
    A: Algorithm = DefaultAlgorithm,
    H: BuildHasher + Clone = RandomState,
> {
    algorithm: PhantomData<A>,
    params: A::BucketParams,
    map_reader: ReadHandle<K, A::BucketState, (), H>,
    map_writer: MapWriteHandle<K, A, H>,
}

impl<A, K> fmt::Debug for KeyedRateLimiter<K, A>
where
    A: Algorithm,
    K: Eq + Hash + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "KeyedRateLimiter{{{params:?}}}", params = self.params)
    }
}

impl<A, K> KeyedRateLimiter<K, A>
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
                nonzero!(1u32),
                per_time_unit,
            ).unwrap(),
            map_reader: r,
            map_writer: Arc::new(Mutex::new(w)),
        }
    }

    pub fn per_second(capacity: NonZeroU32) -> Self {
        Self::new(capacity, Duration::from_secs(1))
    }

    pub fn build_with_capacity(capacity: NonZeroU32) -> Builder<K, A, RandomState> {
        Builder {
            capacity,
            ..Default::default()
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
                let state = &v[0];
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

    pub fn check_at(
        &mut self,
        key: K,
        at: Instant,
    ) -> Result<(), <A as Algorithm>::NegativeDecision> {
        self.check_and_update_key(key, |state| {
            <A as Algorithm>::test_and_update(state, &self.params, at)
        })
    }

    pub fn check_n_at(
        &mut self,
        key: K,
        n: u32,
        at: Instant,
    ) -> Result<(), NegativeMultiDecision<<A as Algorithm>::NegativeDecision>> {
        self.check_and_update_key(key, |state| {
            <A as Algorithm>::test_n_and_update(state, &self.params, n, at)
        })
    }

    pub fn check(&mut self, key: K) -> Result<(), <A as Algorithm>::NegativeDecision> {
        self.check_at(key, Instant::now())
    }

    pub fn check_n(
        &mut self,
        key: K,
        n: u32,
    ) -> Result<(), NegativeMultiDecision<<A as Algorithm>::NegativeDecision>> {
        self.check_n_at(key, n, Instant::now())
    }

    /// Removes the keys from this rate limiter that can be expired safely.
    ///
    /// To be eligible for expiration, a key's rate limiter state must
    /// be at least `min_age` past its last relevance (see
    /// [`RateLimitState.last_touched`](../../algorithms/trait.RateLimitState.html#method.last_touched)).
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
        let at = at.into().unwrap_or_else(Instant::now);

        let mut expireable: Vec<K> = vec![];
        self.map_reader.for_each(|k, v| {
            if let Some(state) = v.get(0) {
                if state.last_touched(params) < at - min_age {
                    expireable.push(k.clone());
                }
            }
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

pub struct Builder<K: Eq + Hash + Clone, A: Algorithm, H: BuildHasher> {
    end_result: PhantomData<(K, A)>,
    capacity: NonZeroU32,
    cell_weight: NonZeroU32,
    per_time_unit: Duration,
    hasher: H,
    map_capacity: Option<usize>,
}

impl<K, A> Default for Builder<K, A, RandomState>
where
    K: Eq + Hash + Clone,
    A: Algorithm,
{
    fn default() -> Builder<K, A, RandomState> {
        Builder {
            end_result: PhantomData,
            map_capacity: None,
            capacity: nonzero!(1u32),
            cell_weight: nonzero!(1u32),
            per_time_unit: Duration::from_secs(1),
            hasher: RandomState::new(),
        }
    }
}

impl<K, A, H> Builder<K, A, H>
where
    K: Eq + Hash + Clone,
    A: Algorithm,
    H: BuildHasher,
{
    pub fn with_hasher<H2: BuildHasher>(self, hash_builder: H2) -> Builder<K, A, H2> {
        Builder {
            hasher: hash_builder,
            end_result: self.end_result,
            capacity: self.capacity,
            cell_weight: self.cell_weight,
            per_time_unit: self.per_time_unit,
            map_capacity: self.map_capacity,
        }
    }

    pub fn with_cell_weight(self, cell_weight: NonZeroU32) -> Self {
        Builder {
            cell_weight,
            ..self
        }
    }

    pub fn with_map_capacity(self, map_capacity: usize) -> Self {
        Builder {
            map_capacity: Some(map_capacity),
            ..self
        }
    }

    pub fn build(self) -> Result<KeyedRateLimiter<K, A, H>, InconsistentCapacity>
    where
        H: Clone,
    {
        let map_opts = evmap::Options::default().with_hasher(self.hasher);
        let (r, mut w) = if self.map_capacity.is_some() {
            map_opts
                .with_capacity(self.map_capacity.unwrap())
                .construct()
        } else {
            map_opts.construct()
        };

        w.refresh();
        Ok(KeyedRateLimiter {
            algorithm: PhantomData,
            params: <A as Algorithm>::params_from_constructor(
                self.capacity,
                self.cell_weight,
                self.per_time_unit,
            )?,
            map_reader: r,
            map_writer: Arc::new(Mutex::new(w)),
        })
    }
}
