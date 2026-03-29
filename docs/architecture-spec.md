# Architecture Spec — Rust Domain + JS Presentation

## Bounded Contexts

```
┌─────────────────────────────────────────────────────┐
│                 PRESENTATION (JS)                     │
│  Canvas, Sprites, QuizRenderer, UI, Input, TTS       │
│  Depends on everything below. Nothing depends on it. │
└──────────────────────┬──────────────────────────────┘
                       │ reads state (JSON from WASM)
┌──────────────────────▼──────────────────────────────┐
│              APPLICATION (JS: adapter.js)             │
│  Orchestrates WASM domain ↔ presentation. Thin glue. │
└───┬──────────────────────────────────────────────────┘
    │ calls via wasm-bindgen (JSON in/out)
┌───▼──────────────────────────────────────────────────┐
│               DOMAIN (Rust → WASM)                    │
│  robot-buddy-domain/                                  │
│  ├── learning/   (profile, reducer, generator, etc.)  │
│  ├── challenge/  (lifecycle state machine)             │
│  └── economy/    (rewards, gifts)                     │
│  Pure Rust. No browser APIs. cargo test.              │
└──────────────────────────────────────────────────────┘
    ▲
┌───┴──────────────────────────────────────────────────┐
│              INFRASTRUCTURE (JS)                      │
│  Speech recognition, Claude/Gemini API, ElevenLabs,   │
│  localStorage (SaveManager)                           │
└──────────────────────────────────────────────────────┘
```

**The golden rule**: the domain is Rust, compiled to WASM. It has ZERO browser dependencies. All domain logic is tested with `cargo test`. The JS layer handles browser APIs, rendering, and user interaction.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Domain | Rust → WASM (wasm-bindgen + serde) |
| Domain tests | `cargo test` (58 tests) |
| Presentation | Vanilla JS, Canvas 2D |
| Infrastructure | JS (browser APIs: Speech, fetch, localStorage) |
| Presentation tests | Vitest (speech recognition parser) |
| Build | wasm-pack (domain), rollup (infrastructure JS) |
| CI | GitHub Actions: cargo test + wasm-pack build + npm test |
| Deploy | GitHub Pages (static: HTML + JS + WASM) |

## Rust Domain Structure

```
robot-buddy-domain/
  Cargo.toml
  src/
    lib.rs                    # WASM exports (wasm_bindgen functions)
    types.rs                  # Shared enums: Operation, SubSkill, CraStage, Phase, etc.
    learning/
      mod.rs
      learner_profile.rs      # Main reducer: band blending, CRA progression, dials
      challenge_generator.rs  # Band distribution, number gen, classification, features
      rolling_window.rs       # Immutable sliding window
      operation_stats.rs      # Coarse + fine-grained stats (HashMap<enum>)
      frustration_detector.rs # Signal analysis → recommendations
      intake_assessor.rs      # Placement logic
    challenge/
      mod.rs
      challenge_state.rs      # Lifecycle reducer: phases, rewards, voice, scaffold
    economy/
      mod.rs
      rewards.rs              # Correct → reward
      give.rs                 # Gift tracking with milestones
    bin/
      simulate.rs             # CLI learning simulator
      simulate_challenge.rs   # CLI challenge lifecycle simulator
```

## JS Presentation Structure

```
# Legacy files (game runs, being migrated incrementally)
index.html              # Entry point, title screen, settings
game.js                 # Game loop, state machine, input dispatch
dialogue.js             # Dialogue box, challenge rendering entry, NPC interactions
sprites.js              # Programmatic tile + character sprites
world.js                # Map data, portals, camera
characters.js           # Player movement, robot follow AI, NPC management
adapter.js              # Wires WASM domain ↔ presentation
wasm-bridge.js          # Loads WASM, exposes window.WasmDomain

# Structured presentation code
src/presentation/
  renderers/
    quiz-renderer.js            # QuizRenderer: choices, show-me/tell-me, celebration
    visual-registry.js          # Registry for visualization methods
    visuals/
      base10-blocks-visual.js   # Tens rods + fives bars + ones cubes
  dev-zone.js                   # Debug gallery (justinbailey)

src/infrastructure/
  speech-recognition.js         # Voice input, number parser

# Build output (gitignored)
dist/
  wasm/
    robot_buddy_domain_bg.wasm  # Compiled domain (~134KB)
    robot_buddy_domain.js       # WASM glue
  speech-recognition.js         # Bundled infrastructure
```

## WASM Bridge

All domain calls go through `wasm-bridge.js` which exposes `window.WasmDomain`:

```js
WasmDomain.createProfile(overrides)       → LearnerProfile (JSON)
WasmDomain.learnerReducer(state, event)   → LearnerProfile (JSON)
WasmDomain.generateChallenge(profile, _)  → Challenge (JSON)
WasmDomain.detectFrustration(window, [])  → FrustrationResult (JSON)
WasmDomain.challengeReducer(state, action)→ ChallengeState (JSON)
// ... etc
```

The adapter calls `WasmDomain.*` for all domain operations. WASM is required — no JS fallback.

## Type Safety Guarantees

The Rust domain provides compile-time guarantees that JS cannot:

| Guarantee | How Rust enforces it |
|-----------|---------------------|
| No state mutation | Structs are immutable by default. Reducers return new state. |
| No missing fields | Adding a field to a struct → compiler error at every construction site |
| No magic strings | `Operation::Add` not `"add"`. Typos are compile errors. |
| Exhaustive matching | Match on `Phase` or `Operation` must handle every variant |
| No null surprise | `Option<T>` is explicit. Must be handled with `match` or `if let`. |
| Deterministic RNG | `SmallRng::seed_from_u64(seed)` — no global `Math.random()` |

## Event Flow

```
User clicks answer
  → game.js handlePointer
  → QuizRenderer.handleClick → returns { type: ANSWER_SUBMITTED, answer: 13 }
  → adapter._onChallengeAnswer(13, time, 'choice')
  → WasmDomain.challengeReducer(state, action)     ← Rust
  → WasmDomain.learnerReducer(profile, event)       ← Rust
  → adapter updates presentation state
  → QuizRenderer.render reads new state
```

Every domain computation crosses the WASM boundary via JSON. The serialization cost is ~0.1ms per call — imperceptible for 30 challenges per session.

## Key Domain Concepts

- **LearnerProfile**: Main state. Dials (pace, scaffolding, spread), CRA stages per operation, math band, rolling window, operation stats. Immutable — reducer returns new profile.
- **Band Blending**: Math band is a distribution center, not a hard level. Problems sampled from a spread around the center. See ADR-001.
- **CRA Progression**: Concrete → Representational → Abstract, tracked per operation. Advances on 3 no-hint correct, demotes on hint-assisted correct or repeated tell-me.
- **Sub-Skills**: Operations split into cognitive sub-skills (add_carry, sub_borrow, etc.). Feature vectors on every problem enable future dynamic sub-skill discovery.
- **Challenge Lifecycle**: State machine (Presented → Feedback → Teaching → Complete). Single reducer handles answers, voice, show-me, tell-me. Rewards are domain-produced, not presentation-decided.
- **Stealth Assessment**: Every interaction is a data point. Events include features, CRA level shown, hint usage, voice metadata. The child never feels tested.

## Evolution Points

### Event Bus (deferred)
Currently the adapter dispatches directly. When the adapter grows too many cross-cutting concerns, introduce a priority-tiered event bus. See `docs/presentation-migration.md`.

### Input Dispatcher (needed)
Multiple files register key listeners independently. Should consolidate to one dispatcher with state-based routing. See `docs/presentation-migration.md`.

### Full Rust Renderer (optional future)
The presentation layer could be ported to Rust via Macroquad (2D game engine with WASM target). Same domain crate, no WASM boundary for domain calls. See `docs/rust-wasm-migration-spec.md` Phase 2.

## Commands

```bash
# Domain tests (Rust)
cd robot-buddy-domain && cargo test

# Infrastructure tests (JS)
npx vitest run

# Build WASM
wasm-pack build robot-buddy-domain --target web --out-dir ../dist/wasm

# Build JS bundles
npm run build

# Simulate a kid
cargo run --manifest-path robot-buddy-domain/Cargo.toml --bin simulate -- --profile gifted

# Dev server
npx serve .
```
