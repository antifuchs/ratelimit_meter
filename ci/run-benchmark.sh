#!/bin/bash

set -eu -o pipefail

if [[ "$RUN_BENCHMARKS" != "true" ]]; then
    exit 0
fi

set -x

cargo bench | tee benches-variable
