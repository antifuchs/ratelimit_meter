use {MultiDeciderImpl, DeciderImpl, TypedDecider, Decider, Decision, Result};

use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
/// A wrapper that ensures operations in otherwise thread-unsafe
/// rate-limiting decision algorithms are thread-safe.

/// This is implemented by wrapping the actual Decider implementation
/// in an atomically reference-counted mutex. It takes out a mutex
/// whenever `.test_and_update()` is called.
pub struct Threadsafe<Impl>
where
    Impl: Decider + Sized + Clone,
{
    sub: Arc<Mutex<Impl>>,
}

impl<Impl> Threadsafe<Impl>
where
    Impl: Decider + Sized + Clone,
{
    // Returns a new Threadsafe wrapper for the given rate-limiting
    // implementation object `sub`.
    pub fn new(sub: Impl) -> Threadsafe<Impl> {
        Threadsafe { sub: Arc::new(Mutex::new(sub)) }
    }
}

impl<Impl> TypedDecider for Threadsafe<Impl>
where
    Impl: TypedDecider + Decider + Sized + Clone,
{
    type T = Impl::T;
}

impl<Impl> DeciderImpl for Threadsafe<Impl>
where
    Impl: Decider + Sized + Clone,
{
    fn test_and_update(&mut self, at: Instant) -> Result<Decision<Impl::T>> {
        self.sub.lock()?.test_and_update(at)
    }
}

impl<Impl> MultiDeciderImpl for Threadsafe<Impl>
where
    Impl: MultiDeciderImpl + Decider + Sized + Clone,
{
    fn test_n_and_update(&mut self, n: u32, at: Instant) -> Result<Decision<Impl::T>> {
        self.sub.lock()?.test_n_and_update(n, at)
    }
}

impl<Impl> Decider for Threadsafe<Impl>
where
    Impl: Decider + Sized + Clone,
{
}
