use {DeciderImpl, Decider, Decision, Result};

use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
/// A wrapper that ensures operations in otherwise thread-unsafe
/// rate-limiting decision algorithms are thread-safe.

/// This is implemented by wrapping the actual Decider implementation
/// in an atomically reference-counted mutex. It takes out a mutex
/// whenever `.test_and_update()` is called.
pub struct Threadsafe<Impl>
    where Impl: Sized,
          Impl: Clone
{
    sub: Arc<Mutex<Impl>>,
}

impl<Impl> Threadsafe<Impl>
    where Impl: Sized,
          Impl: Clone
{
    pub fn new(sub: Impl) -> Threadsafe<Impl> {
        Threadsafe { sub: Arc::new(Mutex::new(sub)) }
    }
}

impl<Impl> DeciderImpl for Threadsafe<Impl>
    where Impl: Decider,
          Impl: Sized,
          Impl: Clone
{
    type T = Impl::T;

    fn test_and_update(&mut self, at: Instant) -> Result<Decision<Impl::T>> {
        self.sub.lock()?.test_and_update(at)
    }
}

impl<Impl> Decider for Threadsafe<Impl>
    where Impl: Decider,
          Impl: Sized,
          Impl: Clone
{
}
