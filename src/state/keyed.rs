use parking_lot::Mutex;
use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use evmap::{self, ReadHandle, WriteHandle};

use {algorithms::Algorithm, InconsistentCapacity, NegativeMultiDecision, NonConformance};

pub struct KeyedRateLimiter<A: Algorithm, K: Eq + Hash + Clone> {
    algorithm: PhantomData<A>,
    params: A::BucketParams,
    map_reader: ReadHandle<K, A::BucketState>,
    map_writer: Arc<Mutex<WriteHandle<K, A::BucketState>>>,
}

impl<A, K> KeyedRateLimiter<A, K>
where
    A: Algorithm,
    K: Eq + Hash + Clone,
{
    pub fn new(capacity: NonZeroU32, per_time_unit: Duration) -> Self {
        let (r, w): (
            ReadHandle<K, A::BucketState>,
            WriteHandle<K, A::BucketState>,
        ) = evmap::new();
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
}

// TODO: add a builder for this one
