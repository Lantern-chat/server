#!/bin/bash

SHARED_FLAGS=" \
    --config build.rustflags=[\"$(echo $RUSTFLAGS | sed -E "s/\s+/\",\"/g")\"] \
    --config profile.release.strip=true \
    --target $RUST_TARGET --release"

echo "Building Lantern release for $RUST_TARGET"

cross build $SHARED_FLAGS --bin embed-worker -p embed-worker || {
    echo "Building Lantern embed-worker failed"; exit 1;
}

cross build $SHARED_FLAGS --bin process -p process --features binary || {
    echo "Building Lantern asset processor failed"; exit 1;
}

cross build $SHARED_FLAGS --bin main -p main || {
    echo "Building Lantern server failed"; exit 1;
}