#![cfg(feature = "std")]
//! An in-memory rate limiter that can keep track of rates for
//! multiple keys, e.g. per-customer or per-IC rates.

use crate::lib::*;

use evmap::{self, ReadHandle, WriteHandle};
use parking_lot::Mutex;

use crate::{
    algorithms::{Algorithm, DefaultAlgorithm, KeyableRateLimitState, RateLimitState},
    clock,
    clock::Reference,
    InconsistentCapacity, NegativeMultiDecision,
};

type MapWriteHandle<K, C, A, H> =
    Arc<Mutex<WriteHandle<K, <A as Algorithm<<C as clock::Clock>::Instant>>::BucketState, (), H>>>;

/// An in-memory rate limiter that regulates a single rate limit for
/// multiple keys.
///
/// Keyed rate limiters can be used to e.g. enforce a per-IC address
/// or a per-customer request limit on the server side.
///
/// This implementation of the keyed rate limiter uses
/// [`evmap`](../../../evmap/index.html), a read lock-free, concurrent
/// hash map. Addition of new keys (e.g. a new customer making their
/// first request) is synchronized and happens one at a time (it
/// synchronizes writes to minimize the effects from `evmap`'s
/// eventually consistent behavior on key addition), while reads of
/// existing keys all happen simultaneously, then get synchronized by
/// the rate limiting algorithm itself.
///
/// ```
/// # use std::num::NonZeroU32;
/// # use std::time::Duration;
/// use ratelimit_meter::{KeyedRateLimiter};
/// # #[macro_use] extern crate nonzero_ext;
/// # extern crate ratelimit_meter;
/// # fn main () {
/// let mut limiter = KeyedRateLimiter::<&str>::new(nonzero!(1u32), Duration::from_secs(5));
/// assert_eq!(Ok(()), limiter.check("customer1")); // allowed!
/// assert_ne!(Ok(()), limiter.check("customer1")); // ...but now customer1 must wait 5 seconds.
///
/// assert_eq!(Ok(()), limiter.check("customer2")); // it's customer2's first request!
/// # }
/// ```
///
/// # Expiring old keys
/// If a key has not been checked in a long time, that key can be
/// expired safely (the next rate limit check for that key would
/// behave as if the key was not present in the map, after all). To
/// remove the unused keys and free up space, use the
/// [`cleanup`](method.cleanup) method:
///
/// ```
/// # use std::num::NonZeroU32;
/// # use std::time::Duration;
/// use ratelimit_meter::{KeyedRateLimiter};
/// # #[macro_use] extern crate nonzero_ext;
/// # extern crate ratelimit_meter;
/// # fn main () {
/// let mut limiter = KeyedRateLimiter::<&str>::new(nonzero!(100u32), Duration::from_secs(5));
/// limiter.check("hi there");
/// // time passes...
///
/// // remove all keys that have been expireable for 10 minutes:
/// limiter.cleanup(Duration::from_secs(600));
/// # }
/// ```
#[derive(Clone)]
pub struct KeyedRateLimiter<
    K: Eq + Hash + Clone,
    A: Algorithm<C::Instant> = DefaultAlgorithm,
    C: clock::Clock = clock::DefaultClock,
    H: BuildHasher + Clone = RandomState,
> where
    A::BucketState: KeyableRateLimitState<A, C::Instant>,
{
    algorithm: A,
    map_reader: ReadHandle<K, A::BucketState, (), H>,
    map_writer: MapWriteHandle<K, C, A, H>,
    clock: C,
}

impl<A, K, C: clock::Clock> fmt::Debug for KeyedRateLimiter<K, A, C>
where
    A: Algorithm<C::Instant>,
    A::BucketState: KeyableRateLimitState<A, C::Instant>,
    K: Eq + Hash + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "KeyedRateLimiter{{{params:?}}}", params = self.algorithm)
    }
}

impl<C, A, K> KeyedRateLimiter<K, A, C>
where
    C: clock::Clock,
    A: Algorithm<C::Instant>,
    A::BucketState: KeyableRateLimitState<A, C::Instant>,
    K: Eq + Hash + Clone,
{
    /// Construct a new rate limiter that allows `capacity` cells per
    /// time unit through.
    /// # Examples
    /// ```
    /// # use std::num::NonZeroU32;
    /// # use std::time::Duration;
    /// use ratelimit_meter::{KeyedRateLimiter};
    /// # #[macro_use] extern crate nonzero_ext;
    /// # extern crate ratelimit_meter;
    /// # fn main () {
    /// let _limiter = KeyedRateLimiter::<&str>::new(nonzero!(100u32), Duration::from_secs(5));
    /// # }
    /// ```
    pub fn new(capacity: NonZeroU32, per_time_unit: Duration) -> Self {
        let (r, mut w): (
            ReadHandle<K, A::BucketState>,
            WriteHandle<K, A::BucketState>,
        ) = evmap::new();
        w.refresh();
        KeyedRateLimiter {
            algorithm: <A as Algorithm<C::Instant>>::construct(
                capacity,
                nonzero!(1u32),
                per_time_unit,
            )
            .unwrap(),
            map_reader: r,
            map_writer: Arc::new(Mutex::new(w)),
            clock: Default::default(),
        }
    }

    /// Construct a new keyed rate limiter that allows `capacity`
    /// cells per second.
    ///
    /// # Examples
    /// Constructing a rate limiter keyed by `&str` that lets through
    /// 100 cells per second:
    ///
    /// ```
    /// # use std::time::Duration;
    /// use ratelimit_meter::{KeyedRateLimiter, GCRA};
    /// # #[macro_use] extern crate nonzero_ext;
    /// # extern crate ratelimit_meter;
    /// # fn main () {
    /// let _limiter = KeyedRateLimiter::<&str, GCRA>::per_second(nonzero!(100u32));
    /// # }
    /// ```
    pub fn per_second(capacity: NonZeroU32) -> Self {
        Self::new(capacity, Duration::from_secs(1))
    }

    /// Return a constructor that can be used to construct a keyed
    /// rate limiter with the builder pattern.
    pub fn build_with_capacity(capacity: NonZeroU32) -> Builder<K, C, A, RandomState> {
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
            })
            .unwrap_or_else(|| {
                // entry does not exist, let's add one.
                let mut w = self.map_writer.lock();
                let state: A::BucketState = Default::default();
                let result = update(&state);
                w.update(key, state);
                w.flush();
                result
            })
    }

    /// Tests if a single cell for the given key can be accommodated
    /// at `Instant::now()`. If it can be, `check` updates the rate
    /// limiter state on that key to account for the conforming cell
    /// and returns `Ok(())`.
    ///
    /// If the cell is non-conforming (i.e., it can't be accomodated
    /// at this time stamp), `check_at` returns `Err` with information
    /// about the earliest time at which a cell could be considered
    /// conforming under that key.
    pub fn check(&mut self, key: K) -> Result<(), <A as Algorithm<C::Instant>>::NegativeDecision> {
        self.check_at(key, self.clock.now())
    }

    /// Tests if `n` cells for the given key can be accommodated at
    /// the current time stamp. If (and only if) all cells in the
    /// batch can be accomodated, the `MultiDecider` updates the rate
    /// limiter state on that key to account for all cells and returns
    /// `Ok(())`.
    ///
    /// If the entire batch of cells would not be conforming but the
    /// rate limiter has the capacity to accomodate the cells at any
    /// point in time, `check_n_at` returns error
    /// [`NegativeMultiDecision::BatchNonConforming`](../../enum.NegativeMultiDecision.html#variant.BatchNonConforming),
    /// holding the number of cells and the rate limiter's negative
    /// outcome result.
    ///
    /// If `n` exceeds the bucket capacity, `check_n_at` returns
    /// [`NegativeMultiDecision::InsufficientCapacity`](../../enum.NegativeMultiDecision.html#variant.InsufficientCapacity),
    /// indicating that a batch of this many cells can never succeed.
    pub fn check_n(
        &mut self,
        key: K,
        n: u32,
    ) -> Result<(), NegativeMultiDecision<<A as Algorithm<C::Instant>>::NegativeDecision>> {
        self.check_n_at(key, n, self.clock.now())
    }

    /// Tests whether a single cell for the given key can be
    /// accommodated at the given time stamp. See
    /// [`check`](#method.check).
    pub fn check_at(
        &mut self,
        key: K,
        at: C::Instant,
    ) -> Result<(), <A as Algorithm<C::Instant>>::NegativeDecision> {
        self.check_and_update_key(key, |state| self.algorithm.test_and_update(state, at))
    }

    /// Tests if `n` cells for the given key can be accommodated at
    /// the given time (`Instant::now()`), using
    /// [`check_n`](#method.check_n)
    pub fn check_n_at(
        &mut self,
        key: K,
        n: u32,
        at: C::Instant,
    ) -> Result<(), NegativeMultiDecision<<A as Algorithm<C::Instant>>::NegativeDecision>> {
        self.check_and_update_key(key, |state| self.algorithm.test_n_and_update(state, n, at))
    }

    /// Removes the keys from this rate limiter that can be expired
    /// safely and returns the keys that were removed.
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
    /// Note that this only affects new keys that need to be
    /// added. Rate-limiting operations on existing keys continue
    /// concurrently.
    ///
    /// # Race conditions
    /// Since this is happening concurrently with other operations,
    /// race conditions can & will occur. It's possible that cells are
    /// accounted between the time `cleanup_at` is called and their
    /// expiry.  These cells will be lost.
    ///
    /// The time window in which this can occur is hopefully short
    /// enough that this is an acceptable risk of loss in accuracy.
    pub fn cleanup<D: Into<Option<Duration>>>(&mut self, min_age: D) -> Vec<K> {
        self.cleanup_at(min_age, self.clock.now())
    }

    /// Removes the keys from this rate limiter that can be expired
    /// safely at the given time stamp. See
    /// [`cleanup`](#method.cleanup). It returns the list of expired
    /// keys.
    pub fn cleanup_at<D: Into<Option<Duration>>, I: Into<Option<C::Instant>>>(
        &mut self,
        min_age: D,
        at: I,
    ) -> Vec<K> {
        let params = &self.algorithm;
        let min_age = min_age.into().unwrap_or_else(|| Duration::new(0, 0));
        let at = at.into().unwrap_or_else(|| self.clock.now());

        let mut expireable: Vec<K> = vec![];
        self.map_reader.for_each(|k, v| {
            if let Some(state) = v.get(0) {
                if state
                    .last_touched(params)
                    .unwrap_or_else(|| self.clock.now())
                    < at.saturating_sub(min_age)
                {
                    expireable.push(k.clone());
                }
            }
        });

        // Now take the map write lock and remove all the keys that we
        // collected:
        let mut w = self.map_writer.lock();
        for key in expireable.iter().cloned() {
            w.empty(key);
        }
        w.refresh();
        expireable
    }
}

/// A constructor for keyed rate limiters.
pub struct Builder<K: Eq + Hash + Clone, C: clock::Clock, A: Algorithm<C::Instant>, H: BuildHasher>
{
    end_result: PhantomData<(K, A)>,
    clock: C,
    capacity: NonZeroU32,
    cell_weight: NonZeroU32,
    per_time_unit: Duration,
    hasher: H,
    map_capacity: Option<usize>,
}

impl<K, A, C> Default for Builder<K, C, A, RandomState>
where
    K: Eq + Hash + Clone,
    C: clock::Clock,
    A: Algorithm<C::Instant>,
    A::BucketState: KeyableRateLimitState<A, C::Instant>,
{
    fn default() -> Builder<K, C, A, RandomState> {
        Builder {
            end_result: PhantomData,
            clock: Default::default(),
            map_capacity: None,
            capacity: nonzero!(1u32),
            cell_weight: nonzero!(1u32),
            per_time_unit: Duration::from_secs(1),
            hasher: RandomState::new(),
        }
    }
}

impl<K, C, A, H> Builder<K, C, A, H>
where
    K: Eq + Hash + Clone,
    C: clock::Clock,
    A: Algorithm<C::Instant>,
    A::BucketState: KeyableRateLimitState<A, C::Instant>,
    H: BuildHasher,
{
    /// Sets the hashing method used for the map.
    pub fn with_hasher<H2: BuildHasher>(self, hash_builder: H2) -> Builder<K, C, A, H2> {
        Builder {
            hasher: hash_builder,
            clock: Default::default(),
            end_result: self.end_result,
            capacity: self.capacity,
            cell_weight: self.cell_weight,
            per_time_unit: self.per_time_unit,
            map_capacity: self.map_capacity,
        }
    }

    /// Sets the "weight" of each cell that is checked against the
    /// bucket.
    pub fn with_cell_weight(self, cell_weight: NonZeroU32) -> Result<Self, InconsistentCapacity> {
        if self.cell_weight > self.capacity {
            return Err(InconsistentCapacity::new(self.capacity, cell_weight));
        }
        Ok(Builder {
            cell_weight,
            ..self
        })
    }

    /// Sets the initial number of keys that the map can hold before
    /// rehashing.
    pub fn with_map_capacity(self, map_capacity: usize) -> Self {
        Builder {
            map_capacity: Some(map_capacity),
            ..self
        }
    }

    /// Sets the clock used by the bucket.
    pub fn using_clock(self, clock: C) -> Self {
        Builder { clock, ..self }
    }

    /// Constructs a keyed rate limiter with the given options.
    pub fn build(self) -> Result<KeyedRateLimiter<K, A, C, H>, InconsistentCapacity>
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
            algorithm: <A as Algorithm<C::Instant>>::construct(
                self.capacity,
                self.cell_weight,
                self.per_time_unit,
            )?,
            clock: self.clock,
            map_reader: r,
            map_writer: Arc::new(Mutex::new(w)),
        })
    }
}
