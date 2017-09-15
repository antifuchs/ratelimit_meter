#!/bin/bash

set -eu -o pipefail
set -x

if [[ "$TRAVIS_RUST_VERSION" == "stable" ]]; then
    cargo install --list > /tmp/installed_crates
    grep "^rustfmt $RUSTFMT_VERSION:" /tmp/installed_crates || cargo install rustfmt --vers $RUSTFMT_VERSION --force
fi

if [[ "$RUN_CLIPPY" == "true" ]]; then
    rm -f ~/.cargo/bin/clippy
    cargo install clippy --force
fi

if [[ "$RUN_BENCHMARKS" == "true" ]]; then
    rm -f ~/.cargo/bin/clippy
    cargo install cargo-benchcmp --force
fi
