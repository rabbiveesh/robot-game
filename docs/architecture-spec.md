# Architecture Spec

One language: Rust. One binary: WASM. Two crates in a workspace.

## Bounded contexts

```
┌────────────────────────────────────────────────────────────┐
│                  PRESENTATION (Macroquad)                  │
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
build-wasm.sh                       # cargo build + assemble www/

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
  Cargo.toml                        # depends on robot-buddy-domain + macroquad
  index.html                        # WASM loader (source — copied into www/ by build)
  src/
    main.rs                         # game loop, GameState, input dispatch
    tilemap.rs                      # map data + rendering
    npc.rs                          # NPC data, facing/adjacency, give handlers
    save.rs                         # localStorage via miniquad plugin
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
      tts.rs                        # SpeechSynthesis via miniquad plugin

robot-buddy-game/www/                # build output (gitignored, except index.html)
  index.html
  mq_js_bundle.js                   # macroquad runtime (from cargo registry)
  robot-buddy-game.wasm
```

## Browser interop

Macroquad uses miniquad, not wasm-bindgen. Browser APIs that the engine doesn't expose are reached through miniquad plugins — small JS shims registered in `robot-buddy-game/index.html` and called from Rust via `extern "C"`. Today there are three:

| Plugin       | Purpose                                          | Rust caller        |
|--------------|--------------------------------------------------|--------------------|
| localStorage | `ls_get`, `ls_set` for save slots                | `save.rs`          |
| tts          | `tts_speak`, `tts_cancel` for dialogue voice     | `audio/tts.rs`     |
| download     | `download_file` for session export blob          | `session.rs`       |

Adding a new browser API means: (1) add the JS shim in `index.html` under `miniquad_add_plugin`, (2) declare the `extern "C"` function in the Rust caller, (3) call it.

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
cargo test                                          # 62 domain tests
cargo build --target wasm32-unknown-unknown --release
./build-wasm.sh                                     # build + assemble www/
cd robot-buddy-game/www && npx serve .              # local dev
cargo run -p robot-buddy-domain --bin simulate -- --profile gifted
```

## ADRs

- [ADR-001: Band Blending](adr/001-band-blending.md) — bands are distribution centers, not hard levels.
