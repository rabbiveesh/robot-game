# ADR-002: Headless Story-Style Integration Tests

**Status:** Accepted and implemented
**Date:** 2026-04-26
**Deciders:** Veesh, Claude

## Context

After the macroquad migration, the game crate had no tests. `async fn main()` was ~1,400 lines holding ~30 mutable locals — input, state machine, RNG, dialogue, challenges, save persistence — all interleaved. Three concrete consequences blocked integration tests:

1. **Can't drive a frame from outside.** Input was scattered across 34 sites (`is_key_pressed` etc.) including inside UI draw functions. No way to inject input without mocking macroquad.
2. **Non-deterministic logic.** Seven `macroquad::rand::gen_range` calls in branching paths (challenge-roll probabilities, dialogue-line picks, gift reactions) bypassed the `SmallRng` already present for the domain. Same input + same seed could produce different outcomes.
3. **Save persistence won't link in tests.** WASM build uses `extern "C"` calls into `index.html`; native build uses `/tmp/*.json` files. Native test runs would either fail to link (against extern "C") or share `/tmp` state across parallel tests.

The domain crate (`robot-buddy-domain`) was clean — pure reducers, seeded RNG, 62 tests. The rot was entirely in the glue.

The goal: integration tests that read like player flows —

```rust
let mut h = Harness::new(seed);
h.start_new_game("Test");
h.walk_to_npc("sparky");
h.interact();
h.select_option("talk");
h.finish_dialogue();
h.answer_correctly();
```

— and run as plain `cargo test` with no window, no harness flag, no display server.

## Decision

Refactor the game crate around a single `Game` struct with a pure step/render split, centralize input through a `FrameInput` boundary, route all gameplay RNG through a seeded `SmallRng`, and abstract save persistence behind a `SaveBackend` trait so tests can inject in-memory storage. Build a thin `Harness` in `tests/common/` with story-level helpers, and assert on a `GameEvent` log rather than internal struct fields.

### Key design choices

**Logic vs. render split.** `Game::step(&FrameInput, dt, screen)` is pure — no `macroquad::*` calls. `Game::render(screen, &FrameInput)` holds every `clear_background`, `set_camera`, `draw_*` call. Tests never invoke render, so they need no graphics context. Production `main()` is ~10 lines: build `FrameInput`, call `step`, call `render`, `next_frame().await`.

**`FrameInput` is the single input boundary.** Populated from macroquad in production via `FrameInput::capture()`; constructed by tests directly. UI modules read from `FrameInput`, never from `is_key_pressed`. The only remaining `is_key_*` callsite in the game crate is inside `FrameInput::capture` itself.

**All gameplay RNG is seeded.** `Game::rng: SmallRng` flows through every probability check and dialogue-line pick. The only `macroquad::rand` use is the production seed source (`main.rs:17`), which never runs in tests.

**`SaveBackend` trait.** Two implementations live in `save.rs`:
- `LocalStorageBackend` — production. Wraps the WASM `extern "C"` ls_get/ls_set calls and the native `/tmp` fallback.
- `InMemoryBackend` — `RefCell<SaveSlots>`. Each test's `Harness` owns one. /tmp is never touched in tests; parallel tests can't share state.

`Game::with_backend(seed, Box<dyn SaveBackend>)` is the test entry point; `Game::new(seed)` defaults to `LocalStorageBackend` for production callers.

**`GameEvent` log as the assertion surface.** `Game::step` pushes events for every state transition, dialogue start/advance, challenge start/resolve, gift, dum_dum award, and map transition. Tests pattern: capture `h.mark()`, perform the action, inspect `h.events_since(mark)`. Events describe *what happened* — they catch regressions like "single-option NPC accidentally pops the menu first" that pure end-state checks miss. Field reads (`h.game.dum_dums`, `h.game.profile`) remain available for genuinely state-y assertions.

**Harness primitives + story helpers.** Primitives drive frames and input: `step`, `press`, `hold`, `type_chars`, `wait_until`, `walk_to`. Story helpers compose them into player-facing actions: `start_new_game`, `start_dev_game`, `walk_to_npc`, `interact`, `select_option`, `answer_correctly`, `complete_intake_correctly`, `finish_dialogue`. New helpers are added when a test demands one — not speculatively.

## Consequences

### Positive
- 7 integration tests run in <0.1s under plain `cargo test`. No window, no `harness = false`, no objc shim.
- Adding a new story-level test is minutes once the helpers exist.
- Event-log assertions print the actual sequence on failure — the diagnostic *is* the assertion surface.
- `InMemoryBackend` makes parallel `cargo test` safe; CI runs tests + WASM build in one job.
- The domain stayed untouched. All 62 domain tests still pass.

### Negative
- `Game` is now ~1,600 lines. The match-arm-as-method extraction is fine, but eventually individual states (Title, Playing, Challenge, etc.) want their own modules.
- Hand-picked seeds (e.g. seed `0` for "talk rolls a challenge") are fragile to RNG-flow changes. If we re-order any `rng.gen()` call before the talk path, the seed picks a different draw and the test hangs in `wait_until`. Acceptable until many such tests exist; revisit by injecting a "force challenge" hook if it bites.
- `GameEvent` will grow. The `// add cases as tests demand them` comment at the enum is load-bearing — resist speculative additions.

### Risks
- **Render-only state mutation.** The split assumes `render(&self)` is read-only. If a per-frame counter (animation phase, particle state) ever moves into `Game` and is mutated during draw, it diverges silently between production and tests. Currently clean.
- **Wall-clock leakage.** `get_time()` is used cosmetically in two cursor-blink renderers (`ui/challenge.rs`, `ui/dialogue.rs`). Both are inside `render`. If logic ever calls `get_time()` from `step`, tests break determinism. Flag in review.
- **Save backend semantics drift.** `LocalStorageBackend` and `InMemoryBackend` must agree on edge cases (`slot >= 3` ignored, missing slot returns `None`). No automated cross-impl test today; the current behavior is simple enough that drift is unlikely.

## Alternatives Considered

**Macroquad headless mode (`harness=false`).** Drive the actual macroquad window with a fake event source. Rejected: still needs an objc shim on macOS, ties test speed to the macroquad event loop, and won't run on a headless CI runner without display config. Pure `step()` with no window beats it on every axis.

**Test by calling domain reducers directly.** Skip the integration entirely; let the domain's 62 tests carry the load. Rejected: the bugs the harness catches (input wiring, state-machine transitions, save flush, dialogue-to-challenge handoff) live in the glue, not the domain. The domain is already well-tested.

**Trait-based rendering to fully decouple macroquad.** Make `Game` generic over a render trait. Rejected: tests assert on `Game` state and the event log, not on what got drawn. The render side stays macroquad-only and that's fine.

**Data-drive NPCs and dialogue lines.** NPCs are hardcoded in `npc.rs`; dialogue lines are inline string literals inside `match` arms. Out of scope here — the harness pathfinds by NPC id, and dialogue assertions go through events. Worth doing later for content velocity, not blocking for testing.

**Keep direct field-read assertions only (no event log).** Simpler, but loses the "what sequence happened" signal. Migration happened mid-stream once the canonical talk→challenge→answer test made the value obvious.

## Implementation

Landed across PR #16 (initial harness + 6 tests) and follow-up branch `harness-followups` (SaveBackend trait, event-based assertions, canonical talk→challenge→answer test).

- `robot-buddy-game/src/input.rs` — `FrameInput`, `FrameInput::capture()`, `FrameInput::empty()`
- `robot-buddy-game/src/game.rs` — `Game` struct, `step` / `render`, `GameEvent`, `event_mark` / `events_since`, accessors used by the harness
- `robot-buddy-game/src/save.rs` — `SaveBackend` trait, `LocalStorageBackend`, `InMemoryBackend`
- `robot-buddy-game/src/lib.rs` — re-exports the modules so `tests/` can import them
- `robot-buddy-game/tests/common/mod.rs` — `Harness` with primitives + story helpers
- `robot-buddy-game/tests/headless.rs` — 2 sanity tests (Title state, Key1 transition)
- `robot-buddy-game/tests/story.rs` — 5 player-flow tests (intake, gift, talk-rolls-challenge, single-option NPC)

CI: existing `.github/workflows/test.yml` runs `cargo test` at workspace root; the new test binaries are picked up automatically with no workflow changes.
