#!/bin/bash

RUST_TARGET="x86_64-unknown-linux-musl"
RUSTFLAGS="-C target-cpu=x86-64-v3 -C target-feature=+aes"

. ${0%/*}/_release.sh $@