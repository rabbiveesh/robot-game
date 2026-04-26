# E2E Readiness Assessment

Status: branch `e2e-readiness`. Goal: get the macroquad game to a place where we can write headless integration tests that read like a story (`game.walk_to_npc("kid_1"); game.interact(); game.select_option("give"); game.answer_correctly();`) and trust them to catch regressions.

## TL;DR

- **Rot level: moderate-to-high in the game crate, low in the domain.** The domain (`robot-buddy-domain/`) is clean — pure reducers, seeded RNG, 62 tests. The game crate (`robot-buddy-game/`) is one 1,400-line `async fn main` with ~30 mutable locals, input scattered across 34 sites, and 7 unseeded RNG calls in branching logic. There are zero tests in the game crate.
- **Estimate to e2e-ready: ~5–7 focused days.** Three refactors in dependency order, then the harness. Most of the work is mechanical extraction; nothing requires re-thinking the design.
- **No domain refactor needed.** Boundary is already pure functions returning new state. The work is entirely in the game crate.

## The North Star: what an e2e test should look like

```rust
#[test]
fn giving_dum_dums_to_kid_1_records_gift() {
    let mut game = GameHarness::new(seed: 42)
        .new_save("Test", Gender::Boy, band: 5)
        .skip_intake();          // shortcut helper, runs intake with all-correct answers

    game.walk_to_npc("kid_1");
    game.interact();
    game.select_option("give");
    game.answer_correctly();      // if a challenge was rolled

    assert_eq!(game.gifts_given_to("kid_1"), 1);
    assert!(game.last_dialogue().contains("thanks"));
}
```

Properties we want:
- Deterministic given a seed (no wall-clock, no `macroquad::rand`).
- Drives the game by **simulated frames + simulated input**, not by calling reducers directly. The whole point is to exercise the integration.
- Runs without a window. `cargo test -p robot-buddy-game` should just work.
- Fails with a useful message — ideally a printable event log of what the harness did.

## Inventory of the rot

### 1. Monolithic main loop — `robot-buddy-game/src/main.rs:329-1403`
`async fn main()` holds ~30 `let mut` bindings (lines 330–364) covering map, player, sparky, camera, dialogue, challenge, intake, profile, save slots, menus, RNG, timers, dum-dums, gifts, session log, settings. The frame body is one big `match state { ... }` (line 374) with each arm doing input + update + side effects + state transitions inline.

Consequences:
- Can't snapshot or pass a slice of state to a function.
- Can't drive a single frame from a test.
- State transitions are imperative mutations spread across match arms — reading "what causes Playing → Challenge?" requires scanning ~5 disjoint sites.

### 2. Input scattered across 34 sites in 6 files
- `main.rs`: 15 sites
- `ui/title_screen.rs`: 11
- `ui/interaction_menu.rs`: 3
- `ui/challenge.rs`: 2
- `ui/settings_overlay.rs`: 2
- `ui/hud.rs`: 1

All call `is_key_pressed`/`is_key_down`/`is_mouse_button_pressed` directly. There is no input abstraction — UI modules read keyboard inside their draw functions (e.g. `ui/interaction_menu.rs:30,60`). Tests cannot inject input without mocking macroquad.

### 3. Unseeded RNG in game logic — violates CLAUDE.md invariant #3
Confirmed sites (all use `macroquad::rand::gen_range`, none routed through the seeded `SmallRng` at `main.rs:339`):
- `main.rs:733` — 40% chance to roll a challenge after talking to NPC
- `main.rs:763` — 50% chance to roll after talking to Sparky
- `main.rs:1114, 1119` — same probabilities, interaction-menu path
- `main.rs:1210, 1256` — pick a random Sparky/NPC dialogue line
- `main.rs:1367` — pick a random gift reaction

Even if everything else is fixed, these alone prevent reproducible tests of the "did interaction X trigger a challenge?" path.

### 4. No event log on the game side
The domain is event-sourced. The game is not. When tests fail, we'll want to know what happened. There's no `Vec<GameEvent>` to print.

### 5. NPCs and dialogue lines are hardcoded
- `npc.rs:54-105`: NPCs hardcoded per map with hardcoded tile coordinates.
- `main.rs:~1199-1258, 1367`: Dialogue lines as inline string literals inside match arms.

This isn't blocking for e2e (we can address NPCs by id), but it means `walk_to_npc("kid_1")` needs a lookup helper, and "what dialogue did we see?" is harder than it should be.

## Refactor plan (dependency-ordered)

### Phase 1: Centralize input (~0.5 day)
Create `robot-buddy-game/src/input.rs` with a `FrameInput` struct populated once per frame from macroquad in production and from a queue in tests. Replace all 34 call sites with reads from this struct.

```rust
pub struct FrameInput {
    pub keys_pressed: Vec<KeyCode>,
    pub keys_down: Vec<KeyCode>,
    pub mouse_pos: (f32, f32),
    pub mouse_clicked: bool,
}
```

Why first: cheap, mechanical, unblocks every later phase.

### Phase 2: Seed all RNG (~0.5 day)
Replace the 7 `macroquad::rand::gen_range` sites with `rng.gen_range(...)` on the existing `SmallRng`. Pass `&mut rng` into the dialogue-line and gift-reaction helpers (they're already small).

Also: change the seed source at `main.rs:339`. In production, seed from time. In tests, seed from a known value. A `cfg(test)` or constructor parameter handles this.

### Phase 3: Extract `Game` struct + `step(input, dt)` (~2-3 days)
The big one. Move the ~30 locals into a `Game` struct. Extract each match arm of the state machine into a method. The frame body becomes:

```rust
impl Game {
    fn step(&mut self, input: &FrameInput, dt: f32) -> Vec<GameEvent>;
    fn render(&self);  // still calls macroquad — only used in production
}
```

`GameEvent` is a flat enum: `DialogueStarted`, `ChallengeStarted`, `OptionSelected`, `ChallengeAnswered { correct }`, `GiftGiven { npc_id }`, etc. This becomes both the test assertion surface and the diagnostic log.

Production `main()` becomes ~30 lines: build `FrameInput::from_macroquad()`, call `game.step()`, call `game.render()`, `next_frame().await`.

### Phase 4: Build `GameHarness` (~1-2 days)
Thin wrapper around `Game` with story helpers:

```rust
impl GameHarness {
    fn new(seed: u64) -> Self;
    fn step_frame(&mut self, input: FrameInput);
    fn run_until<F: Fn(&Game) -> bool>(&mut self, pred: F);

    fn walk_to_npc(&mut self, id: &str);   // computes path, presses arrows, runs frames
    fn walk_to_tile(&mut self, x: i32, y: i32);
    fn interact(&mut self);
    fn select_option(&mut self, option_type: &str);
    fn answer_correctly(&mut self);
    fn answer_incorrectly(&mut self);
    fn advance_dialogue(&mut self);

    fn last_dialogue(&self) -> Option<&str>;
    fn events_since(&self, marker: usize) -> &[GameEvent];
    fn dum_dums(&self) -> u32;
    // ...etc
}
```

The harness is the only thing tests touch. Adding new helpers as we discover what tests need is cheap.

## Suggested first slice (in this branch)

A minimal proof, before committing to the full plan:

1. Phase 1 + Phase 2 (1 day): cheap, valuable on their own (Phase 2 is just paying invariant-#3 debt).
2. A throwaway `GameHarness::run_one_frame_with_input(...)` test that just confirms we can construct a `Game`, feed input, and observe state — even before the full struct extraction. Validates the approach before the big move.

If that lands clean, do Phase 3, then Phase 4 + 2-3 representative e2e tests.

## What's explicitly out of scope

- Data-driving NPCs and dialogue (nice but not blocking).
- Trait-based rendering / decoupling macroquad from rendering. Render stays tied to macroquad — we test by stepping the game and asserting on its state, not by inspecting draw calls.
- Replacing the `match state` with a polymorphic state pattern. Match arms inside methods are fine; the issue today is they're inside `main()` not that they're match arms.
- Any change to the domain crate.

## Risks (verified)

- **Save persistence uses `extern "C"` calls into `index.html`** (`save.rs:127`). These won't link in a native test build. Need a `SaveBackend` trait with a localStorage impl (production) and an in-memory impl (tests). Small, contained.
- **`get_time()` used cosmetically in 2 spots** (`ui/challenge.rs:184`, `ui/dialogue.rs:126`) for blinking cursors only. Not in logic. Safe to leave as wall-clock; tests don't care.
- **`get_frame_time()` at `main.rs:367`** is the only frame-delta source. In `Game::step(input, dt)` the caller passes `dt`, so production passes `get_frame_time()` and tests pass a fixed step.
- **Sprite/render coupling not yet mapped.** The audit assumed render can stay where it is. If `render(&self)` actually mutates something during draw (animation frame counters etc.), Phase 3 will surface that. Likely small.
