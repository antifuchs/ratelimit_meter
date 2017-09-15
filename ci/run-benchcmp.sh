#!/bin/bash

set -eu -o pipefail

if [[ "$RUN_BENCHMARKS" != "true" ]]; then
    exit 0
fi

set -x

# Get comparison data from the master branch for PRs:
if [[ "${TRAVIS_PULL_REQUEST_BRANCH:-$TRAVIS_BRANCH}" != "master" ]]; then
    cargo bench > benches-variable

    REMOTE_URL="$(git config --get remote.origin.url)"
    cd "${TRAVIS_BUILD_DIR}/.."
    git clone "${REMOTE_URL}" "${TRAVIS_REPO_SLUG}-bench"
    cd  "${TRAVIS_REPO_SLUG}-bench"

    git checkout master
    cargo bench > "${TRAVIS_BUILD_DIR}"/benches-control

    cd "${TRAVIS_BUILD_DIR}"
    cargo benchcmp benches-control benches-variable
fi
