#!/bin/bash

set -eu -o pipefail

if [[ "$RUN_CLIPPY" == "true" || "$RUN_BENCHMARKS" == "true" ]]; then
    exit 0
fi

set -x

cargo test
if [[ "$TRAVIS_RUST_VERSION" == "stable" ]]; then
    cargo fmt -v -- --write-mode diff
fi
