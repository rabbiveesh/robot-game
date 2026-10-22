# Architecture Spec

One language: Rust. One binary: WASM. Two crates in a workspace.

## Bounded contexts

```
┌────────────────────────────────────────────────────────────┐
│                    PRESENTATION (Bevy)                     │
│  Tilemap, sprites, UI, input, audio, save, AI dialogue.    │
│  robot-buddy-game/src/                                     │
└────────────────────────┬───────────────────────────────────┘
                         │ direct Rust calls — no JSON, no bridge
┌────────────────────────▼───────────────────────────────────┐
│                       DOMAIN (Rust)                        │
│  Pure logic. No browser APIs.                              │
│  robot-buddy-domain/src/                                   │
│  ├── learning/   profile reducer, challenge gen,           │
│  │               rolling window, frustration, intake       │
│  ├── challenge/  lifecycle state machine                   │
│  ├── economy/    rewards, gifts, interaction options       │
│  └── types.rs    Operation, SubSkill, CraStage, Phase      │
└────────────────────────────────────────────────────────────┘
```

The domain crate has zero rendering or browser deps. The game crate consumes domain types directly through a path dependency. There is no serialization boundary between the two — adding a domain field is a single compiler-checked refactor across the whole workspace.

## Workspace layout

```
Cargo.toml                          # workspace root
build-wasm.sh                       # cargo build + wasm-bindgen + wasm-opt → www/

robot-buddy-domain/
  Cargo.toml
  src/
    lib.rs                          # pub mod types/learning/challenge/economy
    types.rs
    learning/
      learner_profile.rs            # main reducer: bands, CRA, dials
      challenge_generator.rs        # band distribution, number gen, classification
      rolling_window.rs             # immutable sliding window
      operation_stats.rs            # coarse + fine-grained stats
      frustration_detector.rs       # signal analysis → recommendations
      intake_assessor.rs            # placement quiz logic
    challenge/
      challenge_state.rs            # lifecycle reducer (Presented/Feedback/Teaching/Complete)
    economy/
      rewards.rs                    # correct → reward
      give.rs                       # gift tracking with milestones
      interaction_options.rs        # menu items per NPC + player state
    bin/
      simulate.rs                   # CLI learning simulator

robot-buddy-game/
  Cargo.toml                        # depends on robot-buddy-domain + bevy
  index.html                        # WASM loader (source — copied into www/ by build)
  src/
    main.rs                         # starts the Bevy app
    app.rs                          # Bevy plugins: input → Game::step → render → paint
    draw.rs                         # immediate-mode draw calls, recorded for Bevy to paint
    game/                           # Game struct, GameState, step/render
    web.rs                          # browser APIs via web-sys
    tilemap.rs                      # map data + rendering
    npc.rs                          # NPC data, facing/adjacency, give handlers
    save.rs                         # save slots (localStorage via web.rs)
    session.rs                      # session log, JSON export
    settings.rs                     # AI provider, voice, TTS toggle
    sprites/
      player.rs                     # boy/girl
      robot.rs                      # Sparky
      npcs.rs                       # Mommy, kids, Gizmo, dog, etc.
    ui/
      challenge.rs                  # challenge panel, choices, show-me/tell-me, celebration
      dialogue.rs                   # dialogue box, typewriter
      hud.rs                        # area name, Dum Dum counter, badges, debug overlays
      interaction_menu.rs           # NPC option picker
      title_screen.rs               # save slots, name input, settings
      settings_overlay.rs           # in-game settings (ESC)
      visuals.rs                    # math visualizations (dots, ten-frames, base-10 blocks)
    audio/
      tts.rs                        # SpeechSynthesis via web.rs

robot-buddy-game/www/                # build output (gitignored, except index.html)
  index.html
  robot-buddy-game.js               # wasm-bindgen glue (generated)
  robot-buddy-game_bg.wasm
```

## Browser interop

The game is built with wasm-bindgen, so browser APIs are called from Rust directly
through `web-sys`, all in `robot-buddy-game/src/web.rs` (WASM-only; native builds
have stand-ins at each call site). There is no hand-written JS beyond the loader in
`index.html`.

| API                 | Purpose                                     | Rust caller    |
|---------------------|---------------------------------------------|----------------|
| localStorage        | save slots (+ backups of unreadable saves)  | `save.rs`      |
| document visibility | don't autosave from a hidden tab            | `save.rs`      |
| speechSynthesis     | dialogue voice                              | `audio/tts.rs` |
| Blob + `<a download>` | session export                            | `session.rs`   |
| Date                | save timestamps, export date                | `save.rs`, `session.rs` |

Adding a new browser API means: enable its `web-sys` feature in
`robot-buddy-game/Cargo.toml`, wrap it in `web.rs`, and call it under
`#[cfg(target_arch = "wasm32")]`.

## Type-safety guarantees

| Guarantee            | Mechanism                                                              |
|----------------------|------------------------------------------------------------------------|
| No state mutation    | Domain structs immutable; reducers return new state.                   |
| No missing fields    | Adding a field → compiler error at every construction site.            |
| No magic strings     | `Operation::Add`, not `"add"`. Typos are compile errors.               |
| Exhaustive matching  | `match` on `Phase`/`Operation` must cover every variant.               |
| No null surprises    | `Option<T>` is explicit.                                               |
| Deterministic RNG    | Domain takes `&mut impl Rng`. Tests seed `SmallRng::seed_from_u64(42)`.|

## Event flow

```
User clicks an answer choice
  → main.rs handle_pointer
  → ui::challenge::hit_test → ChoiceBound { answer: 13 }
  → GameState::on_answer_submitted(13, response_time, InputKind::Choice)
  → challenge::challenge_state::challenge_reducer(state, action)
  → learning::learner_profile::learner_reducer(profile, event)
  → ui::challenge re-reads new ChallengeState next frame
```

Single thread, single update loop. No async dispatch except for AI dialogue fetch (when wired) and TTS (fire-and-forget into the JS plugin).

## Key domain concepts

- **LearnerProfile** — main aggregate. Dials (pace, scaffolding, spread), per-operation CRA stages, math band, rolling window, operation stats. Immutable; reducer returns new profile.
- **Band blending** — math band is a distribution center, not a hard level. Problems sampled around it. See ADR-001.
- **CRA progression** — Concrete → Representational → Abstract, tracked per operation. Advances on 3 no-hint correct, demotes on hint-assisted correct or repeated tell-me.
- **Sub-skills** — operations split into cognitive sub-skills (add_carry, sub_borrow, …). Feature vectors on every problem.
- **Challenge lifecycle** — state machine (Presented → Feedback → Teaching → Complete). Single reducer handles answers, voice, show-me, tell-me. Rewards are domain-produced.
- **Stealth assessment** — every interaction is a data point. Events carry features, CRA stage shown, hint usage, voice metadata. Child never feels tested.

## Commands

```bash
cargo test                                          # domain + game tests
cargo build --target wasm32-unknown-unknown --release
./build-wasm.sh                                     # build + assemble www/
cd robot-buddy-game/www && npx serve .              # local dev
cargo run -p robot-buddy-domain --bin simulate -- --profile gifted
```

## ADRs

- [ADR-001: Band Blending](adr/001-band-blending.md) — bands are distribution centers, not hard levels.
