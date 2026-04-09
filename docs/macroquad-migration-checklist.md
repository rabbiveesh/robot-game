# Macroquad Migration — Checklist

No new design. Same game, same logic, same architecture. One language instead of two.

## What dies

```
DELETE: game.js, dialogue.js, sprites.js, world.js, characters.js
DELETE: adapter.js, wasm-bridge.js
DELETE: src/presentation/ (JS renderers)
DELETE: src/infrastructure/speech-recognition.js
DELETE: index.html (replace with minimal WASM loader)
DELETE: rollup.config.js, vitest.config.js, package.json, package-lock.json, node_modules/
DELETE: test/ (JS tests — domain tests are cargo test, boundary tests no longer needed)
DELETE: dist/ (JS bundles — WASM binary replaces everything)
DELETE: .github/workflows/static.yml test-js job (no JS to test)
```

## What stays (Rust domain, unchanged)

```
robot-buddy-domain/src/
  types.rs, learning/*, challenge/*, economy/*, bin/*
```

The domain crate becomes a library dependency of the game crate, not a separate WASM module.

## New: Game crate

```
robot-buddy-game/
  Cargo.toml                  # depends on robot-buddy-domain + macroquad
  src/
    main.rs                   # game loop, state machine, init
    input.rs                  # keyboard, mouse, touch — one file, one place
    camera.rs                 # viewport tracking
    tilemap.rs                # map data + rendering
    sprites/
      mod.rs
      player.rs               # boy/girl sprites
      robot.rs                # Sparky
      npcs.rs                 # Mommy, kids, Gizmo, dog, etc.
      tiles.rs                # grass, water, trees, houses, interior tiles
    ui/
      mod.rs
      dialogue.rs             # dialogue box, typewriter, speaker name
      challenge.rs            # QuizRenderer equivalent — choices, show-me, tell-me
      interaction_menu.rs     # NPC option picker
      hud.rs                  # area name, Dum Dum counter, skill badges
      title_screen.rs         # save slots, name input, settings
      dev_zone.rs             # justinbailey debug gallery
      parent_overlay.rs       # P key debug overlay
    visuals/
      mod.rs
      dots.rs
      ten_frame.rs
      base10_blocks.rs
      kenken.rs               # build it in Rust from the start
    audio/
      tts.rs                  # web-sys SpeechSynthesis bindings
      speech_recognition.rs   # web-sys SpeechRecognition bindings
    net/
      ai_dialogue.rs          # fetch to Claude/Gemini APIs
    save.rs                   # localStorage via web-sys or quad-storage
    encounters.rs             # random encounter logic + rendering
```

## Cargo workspace

```toml
# /Cargo.toml (workspace root)
[workspace]
members = ["robot-buddy-domain", "robot-buddy-game"]

# robot-buddy-game/Cargo.toml
[package]
name = "robot-buddy-game"
edition = "2021"

[dependencies]
robot-buddy-domain = { path = "../robot-buddy-domain" }
macroquad = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rand = { version = "0.8", features = ["small_rng"] }

# For browser APIs (localStorage, speech, fetch)
[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "0.3", features = [
    "Window", "Storage",
    "SpeechSynthesis", "SpeechSynthesisUtterance",
    "SpeechRecognition", "SpeechRecognitionResult",
    "Headers", "Request", "RequestInit", "Response",
] }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
```

## Migration order

Port one system at a time. Game is playable after each step.

### Step 1: Skeleton — game loop + tile rendering

```rust
#[macroquad::main("Robot Buddy Adventure")]
async fn main() {
    loop {
        clear_background(Color::from_rgba(26, 26, 46, 255));
        // Draw a hardcoded 5x5 grass grid
        for row in 0..5 {
            for col in 0..5 {
                draw_rectangle(col * 48.0, row * 48.0, 48.0, 48.0,
                    Color::from_rgba(76, 175, 80, 255));
            }
        }
        next_frame().await
    }
}
```

Build: `cargo build --target wasm32-unknown-unknown --release`
Serve: copy WASM + Macroquad JS glue to `_site/`, open in browser.
Validates: Macroquad WASM pipeline works.

### Step 2: Full tile map + camera

Port the overworld map data (the 30×25 tile array from world.js).
Port tile drawing functions (grass, path, water, trees, houses, etc.).
Port the camera (follow player, clamp to map bounds).
Validates: the world renders correctly.

### Step 3: Player + movement + Sparky

Port player sprite (boy/girl).
Port grid-based movement (arrow keys, smooth interpolation).
Port Sparky follow AI.
Validates: can walk around the map with Sparky following.

### Step 4: NPC sprites + interaction

Port all NPC sprites (Mommy, kids, Gizmo, dog, etc.).
Port `getInteractTarget` (facing direction + adjacent tile check).
Port interaction menu (Space → show options → pick one).
Port give mechanic (direct call to `economy::give::process_give` — no bridge!).
Validates: can talk to NPCs, give Dum Dums.

### Step 5: Dialogue box

Port typewriter text rendering.
Port speaker name tab, colored borders.
Port "SPACE >" blink indicator.
Validates: NPCs talk with the typewriter effect.

### Step 6: Challenge system

Port QuizRenderer (challenge panel, choice buttons, feedback, celebration).
Wire domain calls directly: `challenge::challenge_state::challenge_reducer(state, action)`.
Port show-me/tell-me buttons.
Port all visualization methods (dots, ten-frames, base-10 blocks).
Validates: full challenge flow — answer, feedback, teaching, celebration.

### Step 7: Portals + interior maps

Port portal transitions (overworld ↔ houses, secret areas).
Port dream world palette swap, doghouse glitch rendering, hidden grove.
Validates: can enter/exit all map areas.

### Step 8: Save/load + title screen

Port localStorage access (web-sys or quad-storage).
Port save slot UI (3 NES-style slots).
Port title screen (name input, gender picker, level picker, settings).
Port session export.
Validates: full game lifecycle — new game, play, save, reload, continue.

### Step 9: HUD + overlays

Port area name indicator, Dum Dum counter, skill badges.
Port parent debug overlay (P key).
Port voice debug (V key).
Port Dev Zone (justinbailey).
Validates: all debug/info UI works.

### Step 10: Audio

Port TTS via web-sys SpeechSynthesis bindings.
Port speech recognition via web-sys.
Port AI dialogue fetch (Claude/Gemini) via web-sys fetch or reqwest.
Validates: Sparky talks, voice input works, AI dialogue generates.

### Step 11: Delete all JS

Remove every JS file. Remove Node deps. Remove rollup. Update CI to Rust-only.
`index.html` becomes the 5-line WASM loader.
Validates: `ls *.js` returns nothing. Game still works.

## CI after migration

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: cargo test                  # domain + game tests
      - run: cargo build --target wasm32-unknown-unknown --release

  deploy:
    needs: test
    steps:
      # Build WASM binary
      # Copy to _site/
      # Deploy to Pages
```

No Node. No npm. No vitest. No rollup. One toolchain.

## What we gain

- **Zero boundary bugs.** No JSON serialization between domain and presentation. Domain types used directly.
- **One language.** No more "is this function in dialogue.js or adapter.js or wasm-bridge.js?" It's in a Rust module, the compiler tells you.
- **Compiler-checked everything.** Renderers take `&ChallengeState` directly — can't pass the wrong type, can't forget a field, can't read an undefined property.
- **Faster builds.** Macroquad WASM builds are ~3-5 seconds incremental. No npm install, no rollup.
- **Smaller output.** One WASM binary (~300-500KB) vs current HTML + 15 JS files + WASM domain + JS bundles.
- **Future features (KenKen, encounters, ten-frames) built in Rust from the start.** No more "build in JS, worry about the boundary later."
