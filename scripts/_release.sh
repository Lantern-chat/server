#!/bin/bash

if [[ $1 == "--help" || $1 == "-h" ]]; then
    HELP_TEXT=$'Usage:\nbuild-script.sh (embed-worker|process|server)'
    echo "$HELP_TEXT"; exit 0;
fi;

SHARED_FLAGS=" \
    --config build.rustflags=[\"$(echo $RUSTFLAGS | sed -E "s/\s+/\",\"/g")\"] \
    --config profile.release.strip=true \
    --target $RUST_TARGET --release"

echo "Building Lantern release for $RUST_TARGET"

if [[ !($1) || $1 == "embed-worker" ]]; then
    echo "Building Lantern embed-worker";
    cross build $SHARED_FLAGS --bin embed-worker -p embed-worker || {
        echo "Building Lantern embed-worker failed"; exit 1;
    }
else echo "Skipping Lantern embed-worker"; fi;

if [[ !($1) || $1 == "process" ]]; then
    echo "Building Lantern asset processor";
    cross build $SHARED_FLAGS --bin process -p process --features binary || {
        echo "Building Lantern asset processor failed"; exit 1;
    }
else echo "Skipping Lantern asset processor"; fi;

if [[ !($1) || $1 == "server" ]]; then
    echo "Building Lantern main server";
    cross build $SHARED_FLAGS --bin main -p main || {
        echo "Building Lantern server failed"; exit 1;
    }
else echo "Skipping Lantern main server"; fi;