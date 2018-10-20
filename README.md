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
thread-safe. Here are some benchmarks for repeated decisions (run on
my macbook pro, this will differ on your hardware, etc etc):

```
$ cargo bench
    Finished release [optimized] target(s) in 0.16s
     Running target/release/deps/ratelimit_meter-9874176533f7e1a0

running 1 test
test test_wait_time_from ... ignored

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out

     Running target/release/deps/criterion-67011381a5f6ed00
multi_threaded/20_threads/GCRA
                        time:   [1.9664 us 2.0747 us 2.1503 us]
                        thrpt:  [465.04 Kelem/s 482.00 Kelem/s 508.55 Kelem/s]
Found 10 outliers among 100 measurements (10.00%)
  4 (4.00%) low severe
  4 (4.00%) low mild
  2 (2.00%) high mild
multi_threaded/20_threads/LeakyBucket
                        time:   [2.4536 us 2.4878 us 2.5189 us]
                        thrpt:  [396.99 Kelem/s 401.96 Kelem/s 407.56 Kelem/s]
Found 8 outliers among 100 measurements (8.00%)
  5 (5.00%) low severe
  3 (3.00%) low mild

single_threaded/1_element/GCRA
                        time:   [68.613 ns 68.779 ns 68.959 ns]
                        thrpt:  [14.501 Melem/s 14.539 Melem/s 14.575 Melem/s]
Found 13 outliers among 100 measurements (13.00%)
  9 (9.00%) high mild
  4 (4.00%) high severe
single_threaded/1_element/LeakyBucket
                        time:   [64.513 ns 64.855 ns 65.272 ns]
                        thrpt:  [15.321 Melem/s 15.419 Melem/s 15.501 Melem/s]
Found 16 outliers among 100 measurements (16.00%)
  4 (4.00%) high mild
  12 (12.00%) high severe

single_threaded/multi_element/GCRA
                        time:   [96.461 ns 96.976 ns 97.578 ns]
                        thrpt:  [102.48 Melem/s 103.12 Melem/s 103.67 Melem/s]
Found 11 outliers among 100 measurements (11.00%)
  4 (4.00%) high mild
  7 (7.00%) high severe
single_threaded/multi_element/LeakyBucket
                        time:   [69.500 ns 70.359 ns 71.349 ns]
                        thrpt:  [140.16 Melem/s 142.13 Melem/s 143.88 Melem/s]
Found 9 outliers among 100 measurements (9.00%)
  6 (6.00%) high mild
  3 (3.00%) high severe

no-op single-element decision
                        time:   [23.755 ns 23.817 ns 23.883 ns]
Found 11 outliers among 100 measurements (11.00%)
  5 (5.00%) high mild
  6 (6.00%) high severe

no-op multi-element decision
                        time:   [22.772 ns 22.940 ns 23.125 ns]
Found 5 outliers among 100 measurements (5.00%)
  5 (5.00%) high mild
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
