//! # Leaky Bucket Rate-Limiting (as a meter) in Rust
//!
//! This crate provides generic rate-limiting interfaces and
//! implements a few rate-limiting algorithms for programs that need
//! to regulate the rate of their outgoing requests.
//!
//! This crate currently provides in-memory implementations of a by-key
//! (limits enforced per key, e.g. an IP address or a customer ID) and a
//! simple (one limit per object) state tracker.
//!
//! The simple (one limit per object) state tracker can be used in
//! `no_std` environments, such as embedded systems.
//!
//! ## Interface
//!
//! This crate implements two "serious" rate-limiting/traffic-shaping
//! algorithms:
//! [GCRA](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm)
//! and a [Leaky
//! Bucket](https://en.wikipedia.org/wiki/Leaky_bucket#As_a_meter). An
//! "unserious" implementation is provided also: The
//! [`Allower`](example_algorithms/struct.Allower.html), which returns
//! "Yes" to all rate-limiting queries.
//!
//! The Generic Cell Rate Algorithm can be used by in an in-memory
//! rate limiter like so:
//!
//! ``` rust
//! use std::num::NonZeroU32;
//! use ratelimit_meter::{DirectRateLimiter, GCRA};
//!
//! # #[macro_use] extern crate nonzero_ext;
//! # extern crate ratelimit_meter;
//! # #[cfg(feature = "std")]
//! # fn main () {
//! let mut lim = DirectRateLimiter::<GCRA>::per_second(nonzero!(50u32)); // Allow 50 units per second
//! assert_eq!(Ok(()), lim.check());
//! # }
//! # #[cfg(not(feature = "std"))]
//! # fn main() {}
//! ```
//!
//! The rate-limiter interface is intentionally geared towards only
//! providing callers with the information they need to make decisions
//! about what to do with each cell. Deciders return additional
//! information about why a cell should be denied alongside the
//! decision. This allows callers to e.g. provide better error
//! messages to users.
//!
//! As a consequence, the `ratelimit_meter` crate does not provide any
//! facility to wait until a cell would be allowed - if you require
//! this, you should use the
//! [`NonConformance`](struct.NonConformance.html) returned with
//! negative decisions and have the program wait using the method best
//! suited for this, e.g. an event loop.
//!
//! ## Using this crate effectively
//!
//! Many of the parameters in use by this crate are `NonZeroU32` -
//! since they are not very ergonomic to construct from constants
//! using stdlib means, I recommend using the
//! [nonzero_ext](https://crates.io/crates/nonzero_ext) crate, which
//! comes with a macro `nonzero!()`. This macro makes it far easier to
//! construct rate limiters without cluttering your code.
//!
//! ## Rate-limiting Algorithms
//!
//! ### Design and implementation of GCRA
//!
//! The GCRA limits the rate of cells by determining when the "next"
//! cell is expected to arrive; any cells that arrive before that time
//! are classified as non-conforming; the methods for checking cells
//! also return an expected arrival time for these cells, so that
//! callers can choose to wait (adding jitter), or reject the cell.
//!
//! Since using the GCRA results in a much smoother usage pattern, it
//! appears to be very useful for "outgoing" traffic behaviors,
//! e.g. throttling API call rates, or emails sent to a person in a
//! period of time.
//!
//! Unlike token or leaky bucket algorithms, the GCRA assumes that all
//! units of work are of the same "weight", and so allows some
//! optimizations which result in much more concise and fast code (it
//! does not even use multiplication or division in the "hot" path).
//!
//! See [the documentation of the GCRA type](algorithms/gcra/struct.GCRA.html) for
//! more details on its implementation and on trade-offs that apply to
//! it.
//!
//! ### Design and implementation of the leaky bucket
//!
//! In contrast to the GCRA, the leaky bucket algorithm does not place
//! any constraints on the next cell's arrival time: Whenever there is
//! capacity left in the bucket, it can be used. This means that the
//! distribution of "yes" decisions from heavy usage on the leaky
//! bucket rate-limiter will be clustered together. On average, the
//! cell rates of both the GCRA and the leaky bucket will be the same,
//! but in terms of observable behavior, the leaky bucket will appear
//! to allow requests at a more predictable rate.
//!
//! This kind of behavior is usually what people of online APIs expect
//! these days, which makes the leaky bucket a very popular technique
//! for rate-limiting on these kinds of services.
//!
//! The leaky bucket algorithm implemented in this crate is fairly
//! standard: It only updates the bucket fill gauge when a cell is
//! checked, and supports checking "batches" of cells in a single call
//! with no problems.
//!
//! ## Thread-safe operation
//!
//! The in-memory implementations in this crate use parking_lot
//! mutexes to ensure rate-limiting operations can happen safely
//! across threads.
//!
//! Example:
//!
//! ```
//! use std::thread;
//! use std::num::NonZeroU32;
//! use std::time::Duration;
//! use ratelimit_meter::{DirectRateLimiter, GCRA};
//!
//! # #[macro_use] extern crate nonzero_ext;
//! # extern crate ratelimit_meter;
//! # #[cfg(feature = "std")]
//! # fn main () {
//! // Allow 50 units/second across all threads:
//! let mut lim = DirectRateLimiter::<GCRA>::per_second(nonzero!(50u32));
//! let mut thread_lim = lim.clone();
//! thread::spawn(move || { assert_eq!(Ok(()), thread_lim.check());});
//! assert_eq!(Ok(()), lim.check());
//! # }
//! # #[cfg(not(feature = "std"))]
//! # fn main() {}
//! ```
//!
//! ## Usage with `no_std`
//!
//! `ratelimit_meter` can be used in `no_std` crates, with a reduced
//! feature set. These features are available:
//!
//! * [`DirectRateLimiter`](state/direct/struct.DirectRateLimiter.html)
//!   for a single rate-limiting history per limit,
//! * measurements using relative timestamps (`Duration`) by default,
//! * extensibility for integrating a custom time source.
//!
//! The following things are not available in `no_std` builds by default:
//!
//! * `check` and `check_n` - unless you implement a custom time
//!   source, you have to pass a timestamp to check the rate-limit
//!   against.
//! * [`KeyedRateLimiter`](state/keyed/struct.KeyedRateLimiter.html) -
//!   the keyed state representation requires too much of `std` right
//!   now to be feasible to implement.
//!
//! To use the crate, turn off default features and enable the
//! `"no_std"` feature, like so:
//!
//! ``` toml
//! ratelimit_meter = { version = "...", no_default_features = true, features = "no_std" }
//! ```
//!
//! ### Implementing your own custom time source in `no_std`
//!
//! On platforms that do have a clock or other time source, you can
//! use that time source to implement a trait provided by
//! `ratelimit_meter`, which will enable the `check` and `check_n`
//! methods on rate limiters. Here is an example:
//!
//! ```rust,ignore
//! // MyTimeSource is what provides your timestamps. Since it probably
//! // doesn't live in your crate, we make a newtype:
//! use ratelimit_meter::instant;
//! struct MyInstant(MyTimeSource);
//!
//! impl instant::Relative for MyInstant {
//!     fn duration_since(&self, other: Self) -> Duration {
//!         self.duration_since(other)
//!     }
//! }
//!
//! impl instant::Absolute for MyInstant {
//!     fn now() -> Self {
//!         MyTimeSource::now()
//!     }
//! }
//!
//! impl Add<Duration> for MyInstant {
//!     type Output = MyInstant;
//!     fn add(self, rhs: Duration) -> Always {
//!         self.0 + rhs
//!     }
//! }
//!
//! impl Sub<Duration> for MyInstant {
//!     type Output = MyInstant;
//!     fn sub(self, rhs: Duration) -> Always {
//!         self.0 - rhs
//!     }
//! }
//! ```
//!
//! Then, using that type to create a rate limiter with that time
//! source is a little more verbose. It looks like this:
//!
//! ```rust,ignore
//! let mut lim = DirectRateLimiter::<GCRA<MyInstant>,MyInstant>::per_second(nonzero!(50u32));
//! lim.check().ok();
//! ```

// Allow using the alloc crate
#![cfg_attr(not(feature = "std"), feature(alloc))]
// Allow using ratelimit_meter without std
#![cfg_attr(not(feature = "std"), no_std)]
// Deny warnings
#![cfg_attr(feature = "cargo-clippy", deny(warnings))]

pub mod algorithms;
mod errors;
pub mod example_algorithms;
pub mod instant;
pub mod state;
pub mod test_utilities;
mod thread_safety;

#[macro_use]
extern crate nonzero_ext;

#[cfg(feature = "std")]
extern crate evmap;
#[cfg(feature = "std")]
extern crate parking_lot;

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
extern crate spin;

pub use self::algorithms::LeakyBucket;
pub use self::algorithms::NonConformance;
pub use self::algorithms::GCRA;

pub use self::state::DirectRateLimiter;

#[cfg(feature = "std")]
pub use self::state::KeyedRateLimiter;

pub use self::errors::*;

/// A facade around all the types we need from std/core crates, to
/// avoid unnecessary cfg-conditionalization everywhere.
mod lib {
    mod core {
        #[cfg(not(feature = "std"))]
        pub use core::*;

        #[cfg(feature = "std")]
        pub use std::*;
    }

    pub use self::core::clone::Clone;
    pub use self::core::cmp::{Eq, Ord, PartialEq};
    pub use self::core::default::Default;
    pub use self::core::fmt::Debug;
    pub use self::core::marker::{Copy, PhantomData, Send, Sized, Sync};
    pub use self::core::num::NonZeroU32;
    pub use self::core::ops::{Add, Sub};
    pub use self::core::time::Duration;

    pub use self::core::cmp;
    pub use self::core::fmt;

    /// Imports that are only available on std.
    #[cfg(feature = "std")]
    mod std {
        pub use std::collections::hash_map::RandomState;
        pub use std::hash::{BuildHasher, Hash};
        pub use std::sync::Arc;
        pub use std::time::Instant;
    }

    #[cfg(feature = "no_std")]
    mod no_std {
        pub use alloc::sync::Arc;
    }

    #[cfg(feature = "std")]
    pub use self::std::*;

    #[cfg(not(feature = "std"))]
    pub use self::no_std::*;
}
