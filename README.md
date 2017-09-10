[![Build Status](https://travis-ci.org/antifuchs/ratelimit_meter.svg?branch=master)](https://travis-ci.org/antifuchs/ratelimit_meter) [![Docs](https://docs.rs/ratelimit_meter/badge.svg)](https://docs.rs/ratelimit_meter/) [![crates.io](https://img.shields.io/crates/v/ratelimit_meter.svg)](https://crates.io/crates/ratelimit_meter)

# Rate-Limiting with leaky buckets in Rust

This crate implements two rate-limiting algorithms in rust:
* a [leaky bucket](https://en.wikipedia.org/wiki/Leaky_bucket#As_a_meter) and
* a variation on the leaky bucket, the
  [generic cell rate algorithm](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm) (GCRA)
  for rate-limiting and scheduling.

## Installation

Add the crate `ratelimit_meter` to your `Cargo.toml`
file; [the crates.io page](https://crates.io/crates/ratelimit_meter)
can give you the exact thing to paste.

## API Docs

Find them [on docs.rs](https://docs.rs/ratelimit_meter/) for the latest version!

## Design and implementation

Unlike some other token bucket algorithms, the GCRA one assumes that
all units of work are of the same "weight", and so allows some
optimizations which result in much more concise and fast code (it does
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

## Contributions welcome!

I am actively hoping that this project gives people joy in using
rate-limiting techniques. You can use these techniques for so many
things (from throttling API requests to ensuring you don't spam people
with emails about the same thing)!

So if you have any thoughts about the API design, the internals, or
you want to implement other rate-limiting algotrithms, I would be
thrilled to have your input. See [CONTRIBUTING.md](CONTRIBUTING.md)
for details!
