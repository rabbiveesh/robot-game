#!/bin/bash
set -euo pipefail

# Build the game for the browser:
#   1. cargo → raw wasm (size-optimized `wasm-release` profile)
#   2. wasm-bindgen → robot-buddy-game.js + robot-buddy-game_bg.wasm
#      (its version must match the wasm-bindgen crate in Cargo.lock)
#   3. wasm-opt -Oz, if installed (binaryen) — roughly a third smaller
cargo build --target wasm32-unknown-unknown --profile wasm-release -p robot-buddy-game

WANT=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')
HAVE=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)
if [ "$WANT" != "$HAVE" ]; then
    echo "ERROR: need wasm-bindgen-cli $WANT (have '${HAVE:-none}'):"
    echo "  cargo install wasm-bindgen-cli --version $WANT"
    exit 1
fi

OUT=robot-buddy-game/www
mkdir -p "$OUT"
wasm-bindgen --target web --no-typescript --out-dir "$OUT" --out-name robot-buddy-game \
    target/wasm32-unknown-unknown/wasm-release/robot-buddy-game.wasm

if command -v wasm-opt >/dev/null; then
    wasm-opt -Oz --strip-debug --enable-bulk-memory --enable-nontrapping-float-to-int \
        -o "$OUT/robot-buddy-game_bg.wasm" "$OUT/robot-buddy-game_bg.wasm"
else
    echo "note: wasm-opt not found; skipping (the download is ~30% bigger)"
fi

cp robot-buddy-game/index.html "$OUT/"

echo "Built! Serve with: cd $OUT && npx serve ."
echo "WASM size: $(wc -c < "$OUT/robot-buddy-game_bg.wasm" | tr -d ' ') bytes ($(gzip -9 -c "$OUT/robot-buddy-game_bg.wasm" | wc -c | tr -d ' ') gzipped)"
