#!/bin/bash
set -euo pipefail

# Build the game for WASM
cargo build --target wasm32-unknown-unknown --release -p robot-buddy-game

# Find the macroquad JS bundle from cargo registry
MQ_JS=$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" -path "*/macroquad-*/js/mq_js_bundle.js" | head -1)
if [ -z "$MQ_JS" ]; then
    echo "ERROR: macroquad JS bundle not found. Run 'cargo build' first to download deps."
    exit 1
fi

# Assemble www directory
mkdir -p robot-buddy-game/www
cp target/wasm32-unknown-unknown/release/robot-buddy-game.wasm robot-buddy-game/www/
cp "$MQ_JS" robot-buddy-game/www/
cp robot-buddy-game/index.html robot-buddy-game/www/

echo "Built! Serve with: cd robot-buddy-game/www && npx serve ."
echo "JS bundle: $MQ_JS"
echo "WASM size: $(wc -c < robot-buddy-game/www/robot-buddy-game.wasm | tr -d ' ') bytes"
