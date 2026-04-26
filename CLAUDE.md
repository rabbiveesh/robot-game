# Robot Buddy Adventure

A math education RPG for kids (ages 4-10). Zelda-style top-down tile game where math IS the gameplay, not a pop quiz interrupting it.

## Project State

- **`main` branch**: Vanilla-JS prototype (still deployed at https://rabbiveesh.github.io/robot-game/ until macroquad migration lands).

- **`macroquad-migration` branch (current)**: Pure Rust. Domain crate + macroquad game crate, one WASM binary, no JS.

- **`adaptive-learning-design` branch**: Design specs for the architecture (docs/ only, no code).
  - `docs/adaptive-learning-spec.md` — learner profiles, intake quiz, frustration detection, CRA progression
  - `docs/rpg-quest-spec.md` — math as gameplay, story-embedded puzzles, quest system
  - `docs/architecture-spec.md` — DDD architecture, domain model, reducer pattern, project layout

## Architecture Invariants

These are NOT optional. Every PR must respect these:

1. **The domain is Rust.** All game logic lives in `robot-buddy-domain/`. No browser APIs in domain code. `cargo test` must pass. Adding a field to a domain struct? The compiler finds every place that needs updating.

2. **State mutations happen ONLY through reducers.** `learner_reducer(state, event) → new_state`. Rust ownership enforces this — you can't mutate the old state. The event log is the source of truth.

3. **All randomness is seeded.** Domain functions take `&mut impl Rng`. In tests, `SmallRng::seed_from_u64(42)`. No global RNG state.

4. **The game must never time-pressure a child.** No countdown timers on challenges, ever. We measure response time silently for the adaptive system, but the child never sees a clock.

5. **Pass the Broccoli Test.** For every math interaction, ask: "Would this be more fun with the math removed?" If yes, the math is chocolate-covered broccoli and the design is wrong. The math must BE the gameplay.

6. **No labels shown to kids.** The child never sees "Easy", "Band 3", skill levels, or any indication they're being assessed. The adaptive system is invisible. Parent dashboard is the only place this is visible.

7. **Fail gracefully.** Wrong answers have natural in-game consequences (Sparky's battery drains, door doesn't open, merchant says "hmm that's not right"). Never a red X, never "WRONG!", never punishment.

8. **Tests express intent, not implementation.** Default to resilient tests that use helpers (`game.walk_to_npc("kid_1")`, `game.interact()`, `game.select_option("give")`, `game.answer_correctly()`). These break only when gameplay behavior changes. Use fragile tests (hardcoded coordinates, frame counts, pixel positions) ONLY when specifically protecting a layout or timing contract.

## Tech Stack

- **One language: Rust.** Domain crate + macroquad game crate, single WASM binary.
- **Tests**: `cargo test` runs 62 domain unit tests + 7 game integration tests (headless story-style — see ADR-002).
- **Build**: `cargo build --target wasm32-unknown-unknown --release` → `target/wasm32-unknown-unknown/release/robot-buddy-game.wasm`.
- **CI**: GitHub Actions: `cargo test` + WASM build + deploy to Pages. No Node, no npm.

## Project Layout

```
Cargo.toml                       # workspace root

robot-buddy-domain/              # Pure Rust domain (no browser deps)
  src/
    lib.rs                       # pub mod types/learning/challenge/economy
    types.rs                     # Shared enums (Operation, SubSkill, CraStage, Phase)
    learning/                    # Profile reducer, challenge gen, frustration, intake
    challenge/                   # Lifecycle state machine
    economy/                     # Rewards, gifts, interaction options
    bin/
      simulate.rs                # CLI learning simulator

robot-buddy-game/                # Macroquad game (depends on domain)
  Cargo.toml
  index.html                     # WASM loader (source — copied into www/ by build)
  src/
    lib.rs                       # re-exports modules so tests/ can use them
    main.rs                      # thin macroquad shim: capture FrameInput → step → render
    game.rs                      # Game struct + step (pure logic) + render (macroquad-only) + GameEvent
    input.rs                     # FrameInput — single input boundary
    save.rs                      # SaveBackend trait + LocalStorageBackend (prod) + InMemoryBackend (tests)
    tilemap.rs, npc.rs, session.rs, settings.rs
    sprites/                     # player, robot, npcs
    ui/                          # challenge, dialogue, hud, interaction_menu, title_screen, settings_overlay, visuals
    visuals/                     # math visualization renderers
    audio/                       # TTS via miniquad plugin
    net/                         # AI dialogue fetch
  tests/                         # headless integration tests — plain `cargo test`, no window
    common/mod.rs                # Harness + story helpers (walk_to_npc, interact, answer_correctly)
    headless.rs, story.rs        # 7 player-flow tests; assertions read GameEvent log
  www/                           # build output (gitignored except index.html)
```

## Architecture Decision Records

ADRs document key design decisions, their context, and consequences. Read these before proposing alternatives — the "why not" is usually in the Alternatives Considered section.

- **[ADR-001: Band Blending](docs/adr/001-band-blending.md)** — Bands are distribution centers, not hard levels. Accuracy-based promotion replaces streaks. Spread width tightens on frustration, widens on confidence. Streak is display-only.
- **[ADR-002: Headless Test Harness](docs/adr/002-headless-test-harness.md)** — `Game::step` (pure) / `Game::render` (macroquad) split, `FrameInput` boundary, `SaveBackend` trait with `InMemoryBackend` for tests, `GameEvent` log as the assertion surface. Story-style integration tests run as plain `cargo test` with no window.

## Key Domain Concepts

- **LearnerProfile**: Aggregate root. Dials (pace, scaffolding, etc.) + per-operation CRA stages + math band. Immutable, event-sourced.
- **CRA Progression**: Concrete → Representational → Abstract. Tracked per math operation. A kid can be abstract for addition but concrete for division.
- **Frustration Detection**: Analyzes rolling window of last 20 attempts + behavioral signals. Produces recommendations (drop band, encourage, switch to chat).
- **Stealth Assessment**: Every interaction is a data point. The child never feels tested. Assessment happens through gameplay.
- **Event Sourcing with Snapshots**: Events accumulate during a session. On save, snapshot the state + keep last 5 session logs. Bounded growth (~30KB cap).

## Commands

```bash
# Test
cargo test                                                    # 62 domain + 7 game integration tests

# Build WASM
cargo build --target wasm32-unknown-unknown --release

# Assemble www/ (WASM + macroquad JS bundle + index.html)
./build-wasm.sh

# Serve locally
cd robot-buddy-game/www && npx serve .

# Simulate adaptive learning
cargo run -p robot-buddy-domain --bin simulate -- --profile gifted
```

## For Implementers

Read these specs before writing code:
1. `docs/architecture-spec.md` — start here.
2. `docs/adaptive-learning-spec.md` — how the learning system works
3. `docs/challenge-lifecycle-spec.md` — challenge state machine + CRA feedback loop
4. `docs/rpg-quest-spec.md` — how quests and story-embedded math work (future)

Domain changes go in `robot-buddy-domain/` (logic, no rendering). Game changes go in `robot-buddy-game/` (rendering, input, state machine). Run `cargo test` before committing.
