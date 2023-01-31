#!/bin/bash

SPLIT_RUSTFLAGS=$(echo $RUSTFLAGS | sed -E "s/\s+/\",\"/g")

echo "Building Lantern release for $RUST_TARGET"

cross build \
    --config "build.rustflags=[\"$SPLIT_RUSTFLAGS\"]" \
    --config profile.release.strip=true \
    --target $RUST_TARGET \
    --bin process -p process --features binary --release || {
        echo "Building Lantern asset processor failed"; exit 1;
    }

cross build \
    --config "build.rustflags=[\"$SPLIT_RUSTFLAGS\"]" \
    --config profile.release.strip=true \
    --target $RUST_TARGET \
    --bin main -p main --release || {
        echo "Building Lantern server failed"; exit 1;
    }