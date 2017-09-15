#!/bin/bash

set -eu -o pipefail
set -x

if [[ "$RUN_CLIPPY" == "true" ]]; then
    cargo clippy -- -D warnings
fi
