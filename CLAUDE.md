# Robot Buddy Adventure

A math education RPG for kids (ages 4-10). Zelda-style top-down tile game where math IS the gameplay, not a pop quiz interrupting it.

## Project State

- **`main` branch**: Working prototype. Playable at https://rabbiveesh.github.io/robot-game/
  - Vanilla JS, no build step, flat file structure (sprites.js, world.js, characters.js, dialogue.js, game.js, index.html)
  - Global mutable state everywhere — this is the prototype, not the target architecture
  - Features: tile map, player movement, robot companion, NPC dialogue, math/phonics challenges, 3 save slots, TTS, secret areas

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

- **Runtime**: Browser (vanilla JS, ES modules)
- **Bundler**: Rollup
- **Test runner**: Vitest
- **State management**: Immutable state + reducer (domain layer). Mutable state (presentation layer — animation, camera, etc.)
- **CI**: GitHub Actions → GitHub Pages (main branch only, game files only — no docs/tests/node_modules deployed)

## Project Layout (target — migration in progress)

```
src/
  domain/
    learning/       # CORE — reducer, challenge gen, intake, frustration detection
    quest/          # Quest state, templates, micro-quest generation
    world/          # Map data, portals, zones
    character/      # Player, companion, NPC, inventory
  application/      # GameSession, InteractionService, QuestService
  infrastructure/   # SaveManager, Claude API, SpeechService
  presentation/     # Canvas rendering, sprites, UI, input
test/
  domain/learning/  # THE important tests
  domain/quest/
docs/               # Design specs (not deployed)
```

The legacy flat files (sprites.js, world.js, etc.) coexist during migration. The game works at every intermediate state.

## Key Domain Concepts

- **LearnerProfile**: Aggregate root. Dials (pace, scaffolding, etc.) + per-operation CRA stages + math band. Immutable, event-sourced.
- **CRA Progression**: Concrete → Representational → Abstract. Tracked per math operation. A kid can be abstract for addition but concrete for division.
- **Frustration Detection**: Analyzes rolling window of last 20 attempts + behavioral signals. Produces recommendations (drop band, encourage, switch to chat).
- **Stealth Assessment**: Every interaction is a data point. The child never feels tested. Assessment happens through gameplay.
- **Event Sourcing with Snapshots**: Events accumulate during a session. On save, snapshot the state + keep last 5 session logs. Bounded growth (~30KB cap).

## Commands

```bash
npm test              # Run vitest
npm run build         # Rollup bundle for production
npm run dev           # Dev server with watch mode
```

## For Implementers

Read these specs before writing code (on `adaptive-learning-design` branch):
1. `docs/architecture-spec.md` — start here for domain model and project layout
2. `docs/adaptive-learning-spec.md` — how the learning system works
3. `docs/rpg-quest-spec.md` — how quests and story-embedded math work

The current MVP task is in `docs/mvp-adaptive-engine.md`.
