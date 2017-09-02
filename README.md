[![Build Status](https://travis-ci.org/antifuchs/ratelimit_meter.svg?branch=master)](https://travis-ci.org/antifuchs/ratelimit_meter) [![Docs](https://docs.rs/ratelimit_meter/badge.svg)](https://docs.rs/ratelimit_meter/)

# Leaky Bucket Rate-Limiting (as a meter) in Rust

This crate implements
the
[generic cell rate algorithm](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm) (GCRA)
for rate-limiting and scheduling in Rust.

## Interface

You construct a rate limiter using the `Limiter` builder:

``` rust
use std::time::Duration;
use ratelimit_meter::{Limiter, Decider, GCRA, Decision};

let mut lim = Limiter::new()
    .time_unit(Duration::from_secs(1)) // We calculate per-second (this is the default).
    .capacity(50) // Allow 50 units of work per second
    .weight(1) // Each cell is one unit of work "heavy".
    .build::<GCRA>(); // Construct a non-threadsafe GCRA decider.
lim.check() // => Decision::Yes
```

The rate-limiter interface is intentionally geared towards only
providing callers with the information they need to make decisions
about what to do with each cell. Whenever possible, additional
information about why a cell should be denied - the `GCRA`
implementation will return a `time::Instant` alongside the decision to
allow callers to e.g. provide better error messages to users.

Due to this, the `ratelimit_meter` crate does not provide any facility
to wait until a cell would be allowed - if you require this, you
should use the `Instant` returned with negative decisions and wait
in your own, e.g. event loop.

## Design and implementation

Unlike some other token bucket algorithms, the GCRA one assumes that
all units of work are of the same "weight", and so allows some
optimizations which result in much more consise and fast code (it does
not even use multiplication or division in the "hot" path).

The downside of this is that there is currently no support for
assigning different weights to cells.

On the other hand, look at those benchmarks:

```
$ cargo bench
   Compiling ratelimit_meter v0.1.0 (file:///Users/asf/Hacks/ratelimit_meter)
    Finished release [optimized] target(s) in 1.54 secs
     Running /Users/asf/Hacks/ratelimit_meter/target/release/deps/ratelimit_meter-a024ab042ec7d80c

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running /Users/asf/Hacks/ratelimit_meter/target/release/deps/benchmarks-c150c61d51206d3c

running 4 tests
test bench_allower            ... bench:          22 ns/iter (+/- 4)
test bench_gcra               ... bench:          65 ns/iter (+/- 10)
test bench_threadsafe_allower ... bench:          49 ns/iter (+/- 10)
test bench_threadsafe_gcra    ... bench:          84 ns/iter (+/- 36)

test result: ok. 0 passed; 0 failed; 0 ignored; 4 measured; 0 filtered out
```

## Thread-safe operation

The default GCRA implementation can not be used across
threads. However, there is a wrapper struct `Threadsafe`, that wraps
the hot path in an atomically reference-counted mutex. It still
manages to be pretty fast (see the benchmarks above), but the lock
comes with an overhead even in single-threaded operation.

Example:

``` rust
use std::time::Duration;
use ratelimit_meter::{Limiter, Decider, GCRA, Decision};

let mut lim = Limiter::new()
    .time_unit(Duration::from_secs(1)) // We calculate per-second (this is the default).
    .capacity(50) // Allow 50 units of work per second
    .weight(1) // Each cell is one unit of work "heavy".
    .build::<Threadsafe<GCRA>>(); // Construct a threadsafe GCRA decider.
lim.check() // => Decision::Yes
```
