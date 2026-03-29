# Rust + WASM Migration — Design Spec

## Strategy

Two-phase approach:

**Phase 1 (now):** Port the domain layer to Rust. Compile to WASM. JS presentation calls into WASM for all game logic. Eliminates the entire class of bugs we've been fighting (silent mutations, typos, missing cases, magic strings). The domain is already pure functions with zero browser deps — it's designed for this.

**Phase 2 (future, optional):** Port the renderer to Rust via Macroquad or raw web-sys Canvas2D bindings. This is optional — the JS renderer works fine. But if we want a full Rust game, the path exists.

## Phase 1: Domain in Rust

### What moves to Rust

```
src/domain/learning/     → robot_buddy_domain/src/learning/
  learner_profile.rs       (reducer, CRA progression, band blending)
  challenge_generator.rs   (band distribution, number generation, sub-skill classification)
  rolling_window.rs        (immutable sliding window)
  operation_stats.rs       (coarse + fine-grained stats)
  frustration_detector.rs  (signal analysis)
  intake_assessor.rs       (placement logic)

src/domain/challenge/    → robot_buddy_domain/src/challenge/
  challenge_state.rs       (lifecycle reducer, voice state, scaffold actions)

src/domain/economy/      → robot_buddy_domain/src/economy/
  rewards.rs
  give.rs
  interaction_options.rs
```

### Rust project structure

```
robot-buddy-domain/
  Cargo.toml
  src/
    lib.rs                 # WASM entry point, exports all public functions
    learning/
      mod.rs
      learner_profile.rs
      challenge_generator.rs
      rolling_window.rs
      operation_stats.rs
      frustration_detector.rs
      intake_assessor.rs
    challenge/
      mod.rs
      challenge_state.rs
    economy/
      mod.rs
      rewards.rs
      give.rs
      interaction_options.rs
    types.rs               # Shared enums and structs
  tests/
    learning_tests.rs
    challenge_tests.rs
    economy_tests.rs
```

### The type system payoff

Magic strings → enums:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation { Add, Sub, Multiply, Divide, NumberBond }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubSkill {
    AddSingle, AddNoCarry, AddCarry, AddCarryTens,
    SubSingle, SubNoBorrow, SubBorrow, SubBorrowTens,
    MulTrivial, MulEasy, MulHard,
    DivEasy, DivHard,
    BondSmall, BondLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase { Presented, Feedback, Teaching, Complete }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CraStage { Concrete, Representational, Abstract }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrustrationLevel { None, Mild, High }
```

Exhaustive reducers:

```rust
pub fn challenge_reducer(state: ChallengeState, action: ChallengeAction) -> ChallengeState {
    match (state.phase, action) {
        (Phase::Presented, ChallengeAction::AnswerSubmitted { answer }) => {
            let correct = answer == state.challenge.correct_answer;
            if correct {
                ChallengeState {
                    phase: Phase::Complete,
                    correct: Some(true),
                    reward: Some(Reward::DumDum(1)),
                    feedback: Some(Feedback::correct()),
                    voice: VoiceState::reset(),
                    ..state
                }
            } else if state.attempts + 1 >= state.max_attempts {
                ChallengeState {
                    phase: Phase::Teaching,
                    correct: Some(false),
                    reward: None,
                    feedback: Some(Feedback::teaching()),
                    voice: VoiceState::reset(),
                    attempts: state.attempts + 1,
                    ..state
                }
            } else {
                ChallengeState {
                    phase: Phase::Feedback,
                    feedback: Some(Feedback::try_again()),
                    attempts: state.attempts + 1,
                    ..state
                }
            }
        },
        (Phase::Presented, ChallengeAction::ShowMe) => {
            // CRA drop — compiler checked, impossible to forget
            ...
        },
        (Phase::Presented, ChallengeAction::TellMe) => { ... },
        (Phase::Feedback, ChallengeAction::AnswerSubmitted { answer }) => { ... },
        (Phase::Feedback, ChallengeAction::Retry) => { ... },
        (Phase::Teaching, ChallengeAction::TeachingComplete) => { ... },
        // Voice actions in any phase...
        (_, ChallengeAction::VoiceListenStart) => { ... },
        (_, ChallengeAction::VoiceResult { number, confidence }) => { ... },
        (_, ChallengeAction::VoiceConfirm { confirmed }) => { ... },
        (_, ChallengeAction::VoiceError { error }) => { ... },
        // Catch-all for invalid phase×action combinations
        _ => state, // no-op, but compiler tells us which cases we handled
    }
}
```

Add a new action? Compiler error until you handle it in every relevant phase. Add a new phase? Compiler error until you handle it for every action. The "missing case" class of bug is eliminated at compile time.

**The structural completeness guarantee:**

This is the real win — not performance, not even the match arms. It's that Rust makes it **impossible to add a field and forget to wire it through.**

In JS, you add `toldMe` to the challenge state and nothing stops you from forgetting to:
- Include it in the window entry
- Include it in the PUZZLE_ATTEMPTED event
- Include it in save/load serialization
- Check it in the CRA progression logic
- Pass it through the adapter

In Rust:

```rust
struct WindowEntry {
    correct: bool,
    operation: Operation,
    sub_skill: Option<SubSkill>,
    band: u8,
    hint_used: bool,
    told_me: bool,        // add this field
    cra_level_shown: Option<CraStage>,
    // ...
}

// NOW: everywhere that constructs a WindowEntry MUST provide told_me.
// The compiler finds every construction site and errors.
// You literally cannot compile until every path provides the field.
```

Same for events, challenge state, profile — add a field and the compiler shows you every place that needs to change. This is the guarantee that the Dev Zone registry pattern tries to give us for renderers, but Rust gives it for EVERYTHING, for free, at compile time.

Every bug in our "burn the bridge" spec was a structural completeness failure:
- `action.answer` — wrong variable name, would be a compile error
- `choice._bounds` — mutation of immutable data, would be a compile error
- Missing `craLevelShown` field in events — would be a compile error
- `dismissChallenge` not nulling `onComplete` — ownership system prevents this class entirely

Immutability by construction:

```rust
// No Object.freeze needed — Rust structs are immutable by default
// You literally can't mutate them unless you explicitly own them
let profile = create_profile(ProfileConfig::default());
// profile.math_band = 5;  // COMPILE ERROR: cannot assign to immutable field
let new_profile = learner_reducer(profile, event);
// profile is still the old value — it was moved or cloned, never mutated
```

### WASM bridge

Using `wasm-bindgen` + `serde`:

```rust
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

#[wasm_bindgen]
pub fn create_profile(config_json: &str) -> Result<String, JsValue> {
    let config: ProfileConfig = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let profile = Profile::new(config);
    Ok(serde_json::to_string(&profile).unwrap())
}

#[wasm_bindgen]
pub fn learner_reducer(state_json: &str, event_json: &str) -> Result<String, JsValue> {
    let state: Profile = serde_json::from_str(state_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let event: LearnerEvent = serde_json::from_str(event_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let new_state = learning::reduce(state, event);
    Ok(serde_json::to_string(&new_state).unwrap())
}

#[wasm_bindgen]
pub fn generate_challenge(profile_json: &str, seed: u64) -> Result<String, JsValue> {
    let profile: Profile = serde_json::from_str(profile_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut rng = StdRng::seed_from_u64(seed);
    let challenge = generator::generate(&profile, &mut rng);
    Ok(serde_json::to_string(&challenge).unwrap())
}
```

### JS adapter changes

The adapter swaps imports:

```js
// OLD:
const { createProfile, learnerReducer } = window.LearningDomain;

// NEW:
import init, {
  create_profile,
  learner_reducer,
  generate_challenge,
  challenge_reducer,
  detect_frustration,
} from './pkg/robot_buddy_domain.js';

await init(); // load WASM

// Usage stays the same shape:
let profileState = JSON.parse(create_profile(JSON.stringify(config)));
profileState = JSON.parse(learner_reducer(
  JSON.stringify(profileState),
  JSON.stringify(event)
));
```

Or with a thin JS wrapper that handles the JSON:

```js
// domain-bridge.js — thin wrapper, JSON in/out
const wasm = await import('./pkg/robot_buddy_domain.js');
await wasm.default();

export const Domain = {
  createProfile(config) {
    return JSON.parse(wasm.create_profile(JSON.stringify(config)));
  },
  learnerReducer(state, event) {
    return JSON.parse(wasm.learner_reducer(JSON.stringify(state), JSON.stringify(event)));
  },
  generateChallenge(profile, seed) {
    return JSON.parse(wasm.generate_challenge(JSON.stringify(profile), seed));
  },
  // ... etc
};
```

The adapter doesn't know or care that it's calling into WASM. The API shape is identical.

### RNG becomes a seed

JS: `generateChallenge(profile, rng)` where `rng = Math.random` or seeded PRNG.
Rust: `generate_challenge(profile, seed: u64)` — Rust creates its own `StdRng` from the seed.

For production: `seed = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER)`.
For tests: `seed = 42` (deterministic, same as our current seededRng tests).
For simulator: `seed = seedFromString(profileName)` (same as current).

### Testing

```bash
# Rust tests (fast, compile-time checked)
cargo test

# WASM integration tests (optional, slow)
wasm-pack test --chrome --headless
```

The 235 vitest domain tests get rewritten as Rust tests. They'll be MORE thorough because Rust's type system catches things the tests currently check manually:
- "state is frozen" tests → unnecessary (Rust is immutable by default)
- "returns frozen objects" tests → unnecessary
- Enum validation tests → unnecessary (compiler checks)

### Build toolchain

```toml
# Cargo.toml
[package]
name = "robot-buddy-domain"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rand = { version = "0.8", features = ["small_rng"] }

[profile.release]
opt-level = "s"    # optimize for size (WASM)
lto = true
```

```bash
# Build
wasm-pack build --target web --out-dir ../dist/wasm

# The output:
dist/wasm/
  robot_buddy_domain_bg.wasm   # ~50-100KB
  robot_buddy_domain.js        # JS glue
```

### CI changes

```yaml
# .github/workflows/test.yml
- name: Install Rust
  uses: dtolnay/rust-toolchain@stable
  with:
    targets: wasm32-unknown-unknown

- name: Install wasm-pack
  run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

- name: Rust tests
  run: cargo test --manifest-path robot-buddy-domain/Cargo.toml

- name: Build WASM
  run: wasm-pack build robot-buddy-domain --target web --out-dir ../dist/wasm
```

### Migration order

Start with the simplest module, prove the bridge works, then port the rest:

1. **`rolling_window.rs`** — simplest module, pure data + arithmetic. Proves the JSON bridge works.
2. **`operation_stats.rs`** — simple, depends on nothing.
3. **`frustration_detector.rs`** — reads from rolling window, produces a recommendation.
4. **`challenge_generator.rs`** — the biggest module. Enums for operations/sub-skills. Band distribution. Feature extraction. This is where the type system pays off most.
5. **`learner_profile.rs`** — the main reducer. Depends on everything above. CRA progression. Band blending.
6. **`intake_assessor.rs`** — small, depends on challenge generator.
7. **`challenge_state.rs`** — the lifecycle reducer. Independent from the learning domain.
8. **`economy/`** — small, independent.

Each step: port to Rust, add Rust tests, update the JS bridge, delete the JS domain file, verify game still works.

## Phase 2: Full Rust Rendering (optional, future)

If we want the renderer in Rust too:

**Option A: web-sys Canvas2D** — call the same canvas API from Rust. Typed bindings to `CanvasRenderingContext2d`. No performance gain over JS (same browser-native rendering), but type safety and no global mutable state in the renderer.

**Option B: Macroquad** — a Rust game engine that targets WASM natively. Has built-in sprite batching, tilemap support (`macroquad-tiled` loads .tmx files), input handling, and audio. Our entire game could be a single Rust binary compiled to WASM. The JS layer becomes just the HTML page that loads the WASM.

Macroquad example for our use case:

```rust
use macroquad::prelude::*;

#[macroquad::main("Robot Buddy Adventure")]
async fn main() {
    let tilemap = load_tiled_map("overworld.tmx").await;

    loop {
        // Input
        if is_key_pressed(KeyCode::Space) {
            handle_interact();
        }

        // Update
        update_player(get_frame_time());
        update_camera();

        // Draw
        clear_background(Color::from_hex(0x1a1a2e));
        draw_tilemap(&tilemap, camera_offset);
        draw_player(player_pos);
        draw_robot(robot_pos);

        next_frame().await
    }
}
```

This replaces: game.js, sprites.js, characters.js, world.js, dialogue.js (rendering parts), and all the canvas drawing functions. The domain logic (already in Rust from Phase 1) is called directly — no JSON bridge, no WASM boundary, just Rust function calls.

**We'd consider Phase 2 when:**
- The JS renderer becomes a maintenance burden
- We want mobile deployment (Macroquad also targets iOS/Android/desktop natively)
- Bundle size matters (one WASM binary vs 15+ JS files)
- We want to open-source and attract Rust contributors

## Open Questions

- **Bundle size:** The Rust WASM domain module will be ~50-100KB gzipped. The current JS domain is ~10KB. Acceptable for a game but notable.
- **Debugging:** WASM source maps exist but are less mature than JS devtools. console.log from Rust works via `web_sys::console::log_1`. The Dev Zone and session export still work (they read the same JSON state).
- **Does the implementer know Rust?** If not, the learning curve is real. The domain is ~1000 lines of logic — small enough to learn Rust on, but it's not a weekend project.
- **Serde overhead:** JSON serialize/deserialize on every reducer call adds ~0.1ms. Over 30 challenges in a session, that's 3ms total. Imperceptible. But if we ever hot-loop (game physics), we'd need to avoid the boundary.
