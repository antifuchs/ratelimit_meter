use {DeciderImpl, Decider, Decision, Result};
use algorithms::gcra::{GCRA, Builder};

use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
/// A wrapper that ensures operations in otherwise thread-unsafe
/// rate-limiting decision algorithms are thread-safe.

/// This is implemented by wrapping the actual Decider implementation
/// in an atomically reference-counted mutex. It takes out a mutex
/// whenever `.test_and_update()` is called.
pub struct Threadsafe<Impl>
    where Impl: Decider + Sized + Clone
{
    sub: Arc<Mutex<Impl>>,
}

impl<Impl> Threadsafe<Impl>
    where Impl: Decider + Sized + Clone
{
    // Returns a new Threadsafe wrapper for the given rate-limiting
    // implementation object `sub`.
    pub fn new(sub: Impl) -> Threadsafe<Impl> {
        Threadsafe { sub: Arc::new(Mutex::new(sub)) }
    }
}

impl<Impl> DeciderImpl for Threadsafe<Impl>
    where Impl: Decider + Sized + Clone
{
    type T = Impl::T;

    fn test_and_update(&mut self, at: Instant) -> Result<Decision<Impl::T>> {
        self.sub.lock()?.test_and_update(at)
    }

    fn test_n_and_update(&mut self, n: u32, at: Instant) -> Result<Decision<Impl::T>> {
        self.sub.lock()?.test_n_and_update(n, at)
    }
}

impl<Impl> Decider for Threadsafe<Impl>
    where Impl: Decider + Sized + Clone
{
}

/// Allows converting from a GCRA builder directly into a threadsafe
/// GCRA decider. For example:
/// # Example
/// ```
/// use ratelimit_meter::{GCRA, Decider, Threadsafe, Decision};
/// let mut gcra_sync: Threadsafe<GCRA> = GCRA::for_capacity(50).unwrap().into();
/// assert_eq!(Decision::Yes, gcra_sync.check().unwrap());
/// ```
impl<'a> From<&'a Builder> for Threadsafe<GCRA> {
    fn from(b: &'a Builder) -> Self {
        b.build_sync()
    }
}

/// Allows converting from a GCRA builder directly into a threadsafe
/// GCRA decider. Same as
/// [the borrowed implementation](#impl-From<&'a Builder>), except for
/// owned `Builder`s.
impl<'a> From<Builder> for Threadsafe<GCRA> {
    fn from(b: Builder) -> Self {
        b.build_sync()
    }
}
