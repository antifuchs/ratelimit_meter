[![Build Status](https://travis-ci.org/antifuchs/ratelimit_meter.svg?branch=master)](https://travis-ci.org/antifuchs/ratelimit_meter) [![Docs](https://docs.rs/ratelimit_meter/badge.svg)](https://docs.rs/ratelimit_meter/) [![crates.io](https://img.shields.io/crates/v/ratelimit_meter.svg)](https://crates.io/crates/ratelimit_meter)

# Rate-Limiting with leaky buckets in Rust

This crate implements two rate-limiting algorithms in Rust:
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
not even use multiplication or division in the "hot" path for a
single-cell decision).

All rate-limiting algorithm implementations in this crate are
thread-safe and lock-free. Here are some benchmarks for repeated
decisions (run on my macbook pro, this will differ on your hardware,
etc etc):

```
$ cargo bench
   Compiling ratelimit_meter v0.4.1 (file:///Users/asf/Hacks/ratelimit_meter)
    Finished release [optimized] target(s) in 1.71 secs
     Running target/release/deps/ratelimit_meter-680be7c7547f40f9

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running target/release/deps/multi_threaded-b206ea78b9fc87cc

running 2 tests
test bench_gcra_20threads         ... bench:         185 ns/iter (+/- 71)
test bench_leaky_bucket_20threads ... bench:         667 ns/iter (+/- 16,193)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out

     Running target/release/deps/single_threaded-18617cd4f9e09b0d

running 8 tests
test bench_allower                 ... bench:          26 ns/iter (+/- 4)
test bench_gcra                    ... bench:         131 ns/iter (+/- 33)
test bench_gcra_bulk               ... bench:         143 ns/iter (+/- 24)
test bench_leaky_bucket            ... bench:         156 ns/iter (+/- 27)
test bench_leaky_bucket_bulk       ... bench:         152 ns/iter (+/- 24)
test bench_threadsafe_allower      ... bench:          50 ns/iter (+/- 8)
test bench_threadsafe_gcra         ... bench:         133 ns/iter (+/- 21)
test bench_threadsafe_leaky_bucket ... bench:         154 ns/iter (+/- 47)

test result: ok. 0 passed; 0 failed; 0 ignored; 8 measured; 0 filtered out
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
