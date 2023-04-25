#!/bin/bash

RUST_TARGET="aarch64-unknown-linux-musl"
RUSTFLAGS="-C target-cpu=cortex-a57 -C target-feature=-outline-atomics"

. ${0%/*}/_release.sh $@