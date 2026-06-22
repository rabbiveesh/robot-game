#!/bin/bash
set -euo pipefail

# Build the game for WASM
cargo build --target wasm32-unknown-unknown --release -p robot-buddy-game

# The JS glue must match the miniquad version the wasm was compiled against.
# While miniquad is pinned to a git rev (see Cargo.toml), use gl.js from that
# exact checkout — macroquad's prebuilt mq_js_bundle.js ships the older
# crates.io miniquad's gl.js, whose GL shims don't match and fail at
# init_webgl. We don't need the rest of the bundle: no audio/jsutils/net
# plugin crates are in Cargo.lock, and our plugins live in index.html.
MQ_SOURCE=$(grep -A2 'name = "miniquad"' Cargo.lock | grep '^source = ')
if [[ "$MQ_SOURCE" == *"git+"* ]]; then
    REV="${MQ_SOURCE##*#}"
    REV="${REV%\"}"
    MQ_JS=$(find "${CARGO_HOME:-$HOME/.cargo}/git/checkouts" -path "*/miniquad-*/${REV:0:7}/js/gl.js" | head -1)
    if [ -z "$MQ_JS" ]; then
        echo "ERROR: gl.js for pinned miniquad rev $REV not found in cargo git checkouts."
        exit 1
    fi
else
    MQ_JS=$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" -path "*/macroquad-*/js/mq_js_bundle.js" | head -1)
    if [ -z "$MQ_JS" ]; then
        echo "ERROR: macroquad JS bundle not found. Run 'cargo build' first to download deps."
        exit 1
    fi
fi

# Assemble www directory
mkdir -p robot-buddy-game/www
cp target/wasm32-unknown-unknown/release/robot-buddy-game.wasm robot-buddy-game/www/
cp "$MQ_JS" robot-buddy-game/www/mq_js_bundle.js
cp robot-buddy-game/index.html robot-buddy-game/www/

echo "Built! Serve with: cd robot-buddy-game/www && npx serve ."
echo "JS bundle: $MQ_JS"
echo "WASM size: $(wc -c < robot-buddy-game/www/robot-buddy-game.wasm | tr -d ' ') bytes"
