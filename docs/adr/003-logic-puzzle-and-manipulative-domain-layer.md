# ADR-003: Logic-Puzzle, CRA-Manipulative, and Quest Domain Layer

**Status:** Domain layer accepted and implemented; presentation wiring partial
**Date:** 2026-06-06
**Deciders:** Veesh, Claude

## Context

The game shipped one logic puzzle (KenKen) end-to-end. The design docs called for a
much larger surface that all shares one goal — **make math the gameplay, not a
quiz** (the Broccoli Test): the rest of the logic puzzles (`logic-puzzles-spec`),
CRA manipulatives that replace the multiple-choice quiz with hands-on interaction
(`debroccoli-spec`, `visualization-methods-spec`), a quest spine that embeds math
in story (`rpg-quest-spec`), and supporting systems (shop economy, random
encounters, tap-to-move, voice input).

Two forces shaped how we built this:

1. **The game crate has a serialization bottleneck.** `game.rs` (state machine),
   `npc.rs`, and `economy/interaction_options.rs` are touched by *every* player
   feature. Parallel feature branches collide there constantly.
2. **The harness is strong for domain logic, weak for presentation.** Per ADR-002,
   pure reducers are trivially `cargo test`-able; macroquad drag/drop, network, and
   browser APIs are not.

## Decision

**Build every new feature as a pure, headlessly-tested domain module first, and
wire presentation as a separate, serialized step.**

Each puzzle/manipulative/quest is modeled exactly like the existing
`logic::kenken` / `logic::patterns` modules:

- Plain data types (`serde`, camelCase) + a `*Session` + a pure `*_reducer`.
- A `*Phase` (InProgress/Complete) and seeded generation (`&mut impl Rng`).
- Architecture invariants honored: reducers-only mutation, seeded RNG, *fail
  gracefully* (wrong moves are recoverable, never punished, never lock the puzzle),
  no time pressure, no labels.
- Generators take **explicit operands**, decoupled from the challenge generator, so
  they're unit-testable in isolation; the game maps a `Challenge` → a module's
  inputs at wire time.

This makes the domain layer fully parallelizable (independent files; only one-line
`mod.rs` edits collide) and the conflict-prone `game.rs` wiring the only serial
step.

### What landed (domain, all `cargo test`-green)

| Module | Spec | What the kid does |
|---|---|---|
| `logic::patterns` | logic-puzzles §2 | continue a sequence (AB/ABB/ABC, skip-count, double, squares) |
| `logic::balance` | logic-puzzles §3 | pick the value that levels a scale (visual algebra) |
| `logic::sudoku` | logic-puzzles §4 | fill a 4×4 (picture) / 6×6 grid — pure constraint logic |
| `logic::manipulate_concrete` | debroccoli (concrete) | tap-to-count, drag-to-group, build-a-tower, take-away |
| `logic::number_line` | debroccoli (representational) | hop a character to the answer |
| `logic::base_ten` | debroccoli (representational) | trade ten ones for a ten (carry) / break a ten (borrow) |
| `quest` | rpg-quest | advance a story whose steps include embedded math puzzles |
| `economy::shop` | dum-dum-economy §2 | buy cosmetics — every purchase is an embedded subtraction |
| `world::encounters` | ten-frames-and-encounters §2 | the world springs flavor / free Dum Dums / surprise puzzles |
| `text::voice_parser` | voice-input-impl | "twenty-three" → 23 (the "Say it" answer mode core) |
| `pathfinding` (game crate) | tap-to-move | BFS path to a tile / adjacent to an NPC |

`patterns`, `balance`, and `sudoku` are also **fully wired** into the game
(GameState, UI, NPC menu options, dev-control triggers, events, learner-profile
hooks, story tests) — same shape as KenKen.

### Anti-freeze rule (also fixes the original KenKen bug)

Grid puzzles that pre-fill "given" cells must report `Complete` from
`Session::new` if the givens already solve the board. Otherwise a fully-given
puzzle starts solved but the reducer never flips the phase (no action is ever
dispatched) and the kid is frozen on a finished grid. KenKen generation also caps
the number of givens so low levels stay non-trivial.

## Wiring roadmap (the remaining serial step)

Each pending module wires exactly like the puzzle features already shipped. The
proven recipe (see commits for patterns/balance/sudoku):

1. `GameState::X` variant + `ActiveX { session, … }` + `Game.active_x` field +
   `active_x()` accessor + `None` init.
2. `GameEvent::XStarted/XResolved`; dispatch arm → `step_x`; render arm → `ui::x`.
3. `start_x` / `step_x` (+ `apply_x_intent` for grid puzzles with a selected cell).
4. Reachability: a `ctrl_trigger_x` dev knob (deterministic test hook) and/or a
   puzzler-menu option; clear `active_x` in the ESC/back-to-title safeguards.
5. `ui::x` — pure `layout` (hit-testable rects) + `draw_x` + `handle_click/key`.
6. Harness helpers + a story-style integration test.

Module-specific notes:

- **CRA manipulatives** are the real broccoli-killer: hook them into the *challenge
  flow*, not the puzzler menu. When a challenge is add/sub and the learner's CRA
  stage for that op is Concrete/Representational, present the manipulative
  (`manipulate_concrete` for concrete, `number_line`/`base_ten` for
  representational) instead of multiple choice; map the `Challenge` operands into
  the module's generator and report the outcome as the normal `PUZZLE_ATTEMPTED`
  event. This is the deepest integration (challenge_state / RenderHint / CRA /
  `ui::challenge`) and benefits most from human verification of feel.
- **Quest**: needs a `GameState` for step execution + a journal UI (Tab) + NPC
  `quest_giver` flag. `MathPuzzle` steps hand off to the existing challenge flow
  (or a manipulative); on `correct`, dispatch `CompletePuzzle{correct:true}`.
  `Travel` steps pair naturally with `pathfinding`. `pending_reward()` tells the
  game when to pay out before the final `AdvanceStep`.
- **Shop**: set `NpcInfo.has_shop` for the Shopkeeper; the `"shop"` interaction
  option already exists in `interaction_options`. Selecting an affordable item
  presents `PurchaseResult.{spent,new_balance}` as the subtraction moment;
  `shortfall()` drives the "how many more?" number bond. Owned cosmetics need a
  save-schema field and a Sparky sprite hook (future).
- **Encounters**: increment a step counter on each completed tile move in
  `step_playing`; when `should_trigger_encounter` fires *and the player has
  stopped*, run `pick_encounter` → reuse the dialogue flow (flavor / sighting),
  award a Dum Dum (FoundDumDum), or `start_challenge` (Challenge).
- **Voice / tap-to-move**: `voice_parser` is the pure core of the "Say it" mode;
  the mic/recognition layer is browser-only and deferred (see below). `pathfinding`
  feeds tap-to-move and quest travel; the game converts a tap → tile via the camera
  and walks the returned path as move intents.

## Deferred (need a human decision; unsafe to build unattended)

- **AI dialogue cache + conversational mode** (`genai-spec`): requires API keys,
  network, and a WASM `fetch` plugin — untestable headlessly, needs secrets.
- **Browser speech recognition** (`native-speech-spec`, `voice-input-impl`):
  browser-only Web Speech API; sends audio to Google on Chrome (privacy). Needs an
  opt-in + parent disclosure decision. The pure parser (`text::voice_parser`) is
  done and waiting.
- **Native TTS crates / Whisper**: platform deps and a ~75 MB model download.
- **Per-sub-skill bands** (`per-subskill-bands-spec`): a band-model refactor the
  spec itself gates on real playtest data (>30% accuracy gaps between sub-skills).

## Consequences

- **Good:** Every spec's *logic* now exists as tested, reviewable Rust independent
  of rendering. Wiring is mechanical and follows one recipe. The domain test count
  roughly tripled (62 → ~200). New puzzle types can't reintroduce the freeze bug.
- **Cost:** The ⏳ modules are not yet player-reachable — they're correct but inert
  until wired. A reader must consult this ADR to see what's live vs. pending.
- **Parallelism caveat:** We intended to build the domain modules on parallel
  worktree branches with an integration step. A transient API overload took the
  background workers offline, so they were built serially on one branch instead —
  which incidentally removed the integration step entirely. The layered split
  (pure domain modules, serial wiring) is the durable decision regardless of how
  many workers are available.
