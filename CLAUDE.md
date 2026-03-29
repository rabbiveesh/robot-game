# Robot Buddy Adventure

A math education RPG for kids (ages 4-10). Zelda-style top-down tile game where math IS the gameplay, not a pop quiz interrupting it.

## Project State

- **`main` branch**: Working prototype. Playable at https://rabbiveesh.github.io/robot-game/
  - Vanilla JS, no build step, flat file structure (sprites.js, world.js, characters.js, dialogue.js, game.js, index.html)
  - Global mutable state everywhere — this is the prototype, not the target architecture
  - Features: tile map, player movement, robot companion, NPC dialogue, math challenges, 3 save slots, TTS, secret areas

- **`adaptive-learning-design` branch**: Design specs for the real architecture (docs/ only, no code)
  - `docs/adaptive-learning-spec.md` — learner profiles, intake quiz, frustration detection, CRA progression
  - `docs/rpg-quest-spec.md` — math as gameplay, story-embedded puzzles, quest system
  - `docs/architecture-spec.md` — DDD architecture, domain model, reducer pattern, project layout

## Architecture Invariants

These are NOT optional. Every PR must respect these:

1. **The Learning domain (`src/domain/learning/`) has ZERO browser dependencies.** No DOM, no canvas, no `window`, no `document`. Pure logic. If it can't run in `node`, it's in the wrong layer.

2. **State mutations in the Learning domain happen ONLY through the reducer.** `(state, event) → newState`. No direct mutation. `Object.freeze` on all returned state. The event log is the source of truth.

3. **All randomness is injected.** Every function that needs randomness takes an `rng: () => number` parameter. In production, pass `Math.random`. In tests, pass a seeded PRNG. No calls to `Math.random()` inside domain code.

4. **Domain events are plain objects.** No classes, no methods on events. `{ type: 'PUZZLE_ATTEMPTED', correct: true, operation: 'add', ... }`. Serializable to JSON.

5. **The game must never time-pressure a child.** No countdown timers on challenges, ever. We measure response time silently for the adaptive system, but the child never sees a clock.

6. **Pass the Broccoli Test.** For every math interaction, ask: "Would this be more fun with the math removed?" If yes, the math is chocolate-covered broccoli and the design is wrong. The math must BE the gameplay.

7. **No labels shown to kids.** The child never sees "Easy", "Band 3", skill levels, or any indication they're being assessed. The adaptive system is invisible. Parent dashboard is the only place this is visible.

8. **Fail gracefully.** Wrong answers have natural in-game consequences (Sparky's battery drains, door doesn't open, merchant says "hmm that's not right"). Never a red X, never "WRONG!", never punishment.

## Tech Stack

- **Domain**: Rust → WASM (wasm-bindgen + serde). All game logic.
- **Domain tests**: `cargo test` (58 tests)
- **Presentation**: Vanilla JS, Canvas 2D
- **Infrastructure tests**: Vitest (speech recognition parser)
- **Build**: wasm-pack (domain), rollup (infrastructure JS)
- **CI**: GitHub Actions: cargo test + wasm-pack build + vitest + deploy to Pages

## Project Layout

```
robot-buddy-domain/           # Rust crate → WASM
  src/
    lib.rs                    # WASM exports
    types.rs                  # Shared enums (Operation, SubSkill, CraStage, Phase)
    learning/                 # Profile reducer, challenge gen, frustration, intake
    challenge/                # Lifecycle state machine
    economy/                  # Rewards, gifts
    bin/
      simulate.rs             # CLI learning simulator
      simulate_challenge.rs   # CLI challenge simulator

src/presentation/             # JS renderers
  renderers/
    quiz-renderer.js
    visuals/base10-blocks-visual.js
  dev-zone.js

src/infrastructure/           # JS browser APIs
  speech-recognition.js

# Legacy JS (game shell, being migrated)
game.js, dialogue.js, sprites.js, world.js, characters.js
adapter.js, wasm-bridge.js
```

## Architecture Decision Records

ADRs document key design decisions, their context, and consequences. Read these before proposing alternatives — the "why not" is usually in the Alternatives Considered section.

- **[ADR-001: Band Blending](docs/adr/001-band-blending.md)** — Bands are distribution centers, not hard levels. Accuracy-based promotion replaces streaks. Spread width tightens on frustration, widens on confidence. Streak is display-only.

## Key Domain Concepts

- **LearnerProfile**: Aggregate root. Dials (pace, scaffolding, etc.) + per-operation CRA stages + math band. Immutable, event-sourced.
- **CRA Progression**: Concrete → Representational → Abstract. Tracked per math operation. A kid can be abstract for addition but concrete for division.
- **Frustration Detection**: Analyzes rolling window of last 20 attempts + behavioral signals. Produces recommendations (drop band, encourage, switch to chat).
- **Stealth Assessment**: Every interaction is a data point. The child never feels tested. Assessment happens through gameplay.
- **Event Sourcing with Snapshots**: Events accumulate during a session. On save, snapshot the state + keep last 5 session logs. Bounded growth (~30KB cap).

## Commands

```bash
# Domain (Rust)
cd robot-buddy-domain && cargo test           # 58 domain tests
wasm-pack build robot-buddy-domain --target web --out-dir ../dist/wasm

# Presentation (JS)
npx vitest run                                # Infrastructure tests
npm run build                                 # Rollup JS bundles

# Simulate
cargo run --manifest-path robot-buddy-domain/Cargo.toml --bin simulate -- --profile gifted

# Dev
npx serve .                                   # Local server (WASM needs HTTP)
```

## Presentation Layer Debt

The legacy flat files (dialogue.js, game.js, world.js, sprites.js, characters.js) are the original prototype. They work but accumulate debt with every feature. The domain is Rust (clean, tested, type-safe). The presentation is legacy JS (not clean).

**DO NOT migrate the presentation layer as a standalone project.** Each feature triggers migration of the specific part it needs. See `docs/presentation-migration.md` for:
- Which feature triggers which migration
- Recommended migration order
- What each legacy file splits into
- When to delete each legacy file

The adapter (`adapter.js`) is the bridge and is intentionally ugly. It dies when the presentation migration is complete.

## For Implementers

Read these specs before writing code:
1. `docs/architecture-spec.md` — start here. Rust domain + JS presentation.
2. `docs/adaptive-learning-spec.md` — how the learning system works
3. `docs/challenge-lifecycle-spec.md` — challenge state machine + CRA feedback loop
4. `docs/rpg-quest-spec.md` — how quests and story-embedded math work (future)

Before building any presentation feature, check `docs/presentation-migration.md`.

Domain changes go in `robot-buddy-domain/` (Rust). Run `cargo test` before committing. Presentation changes go in JS legacy files or `src/presentation/`.
