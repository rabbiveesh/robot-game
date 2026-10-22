# ADR-005: Port the Game Crate from macroquad to Bevy

**Status:** Accepted and implemented
**Date:** 2026-10-22
**Deciders:** Veesh, Claude

## Context

The game crate had grown a set of things engines and common crates already ship:
a flexbox layout engine, per-panel hit-testing, a hand-wired game loop, a text
stack, and browser glue written as JavaScript plugins in `index.html` with
`extern "C"` calls (macroquad doesn't use wasm-bindgen). The census in
`/mnt/project-files/reviews/bevy-kitten-census.md` counted about 8–9k of the crate's
20k lines as reinvented. The owner's direction: stop reinventing wheels.

A spike (`spike-bevy/`, since deleted) proved the risky parts on Bevy 0.19.1:

- the domain crate plugs in unchanged;
- headless story tests still work;
- the procedural sprites port through a small shim over `bevy_vector_shapes`;
- the bundled Unifont works as Bevy's default font;
- it renders natively under `xvfb` (lavapipe) and in headless Chromium (WebGL2).

## Decision

Port `robot-buddy-game` in place to Bevy 0.19.1, keeping `Game` as a pure core.

```text
 PreUpdate                       Update                              PostUpdate
 capture: Bevy input ──► FrameInput ──► Game::step(input, dt, screen)
                                       Game::render ──► draw::Cmd list ──► paint: ShapePainter + pooled Text2d
```

| Piece | File | What it does |
|---|---|---|
| App | `src/app.rs` | `DrawPlugin` (camera, default font, painting the `DrawList`) and `GamePlugin` (input capture, `step`, `render`). `Game` is a non-send resource because the save backend uses `Rc`. |
| Drawing | `src/draw.rs` | Immediate-mode calls with macroquad's names and coordinates (`draw_rectangle`, `draw_text`, `Color::from_rgba`, `set_camera`…). Each call appends a `Cmd`; `paint` draws them in order, each a hair in front of the last. Recording needs no window, so `render` can run in the screenshots example. |
| Input | `src/input.rs` | `FrameInput` as before, with the game's own `KeyCode` (macroquad names) mapped from Bevy's. Touch landing counts as a click. Every mapped key is captured; macroquad's capture list had silently dropped 5–9 and 0. |
| Browser | `src/web.rs` | `web-sys`: localStorage, tab visibility, speechSynthesis, file download, `Date`. No hand-written JS beyond the loader in `index.html`. |
| UI layout | `src/ui/layout/` | ADR-004's pipeline, with the engine swapped for taffy (the same taffy Bevy UI uses). |
| Web build | `build-wasm.sh` | cargo (`wasm-release` profile) → `wasm-bindgen --target web` → `wasm-opt -Oz`. |
| Screenshots | `examples/screenshots.rs` | Drives the game with the test harness, records each shot's draw list, then paints and saves them from a Bevy window. Needs `--features screenshots` (PNG encoding stays out of the WASM). |

`Game::step` is untouched in substance: every test (unit, story, save safety,
layout sweep, layout discipline) runs exactly as before, headless.

### What was not moved onto Bevy, and why

**Panels stay on ADR-004's layout, not Bevy UI entities.** Panels already describe a
flexbox tree, and the engine under it is now taffy, the same engine as Bevy UI. Moving them to
Bevy UI entities would delete the `Frame`/painter layer and `hit_at`, but it would:

- lose the headless layout checks (`assert_sane` across 5 screens × awkward data),
  because layout would only exist inside a running Bevy app;
- need a reconciler from panel trees to entities, a custom measure for `Fit` text,
  and one camera pair per UI layer so panel art draws above its own panel but under
  the next modal;
- swap about five lines of hit-testing for `bevy_picking`.

The trade wasn't worth it now. A panel can still move to Bevy UI on its own later.

**`Game` stays one struct with a `GameState` enum**, not ECS states and per-minigame
plugins. That split is a refactor of game logic, not a port. It's easier to do one
minigame at a time now that Bevy is underneath.

**Maps stay Rust arrays** (no LDtk/Tiled yet), and **sprites stay procedural** via
`draw.rs`.

## Consequences

### Positive
- macroquad, the miniquad pin, the `gl.js` matching hack, `--allow-undefined` and
  every JS plugin are gone. Browser APIs are typed Rust calls.
- Our flexbox engine is gone. Layout is taffy, already in the binary for Bevy.
- Saves on the web now carry real timestamps; session exports carry a real date.
- Text renders crisply at the display's scale factor (Bevy rasterizes at physical
  size), and Unifont is the default font everywhere without naming it.
- Bevy is underneath for later work: drag and touch via picking, states, audio,
  asset loading, tweening.

### Negative
- **Download size.** The WASM is several times macroquad's (see the PR for the
  measured number; the spike measured 5.2 MB gzipped against 0.56 MB). It's a
  first-visit cost; browsers cache it.
- **Cold builds** take 10+ minutes and several GB of `target/`. Dependency
  debuginfo is off in the dev profile to keep the size down. CI caches the build.
- `draw.rs` is a small compatibility layer of our own. It is ~250 lines and has no
  logic beyond coordinate conversion.
- There's no startup probe that Bevy's text widths match the headless `FontMetrics`
  (macroquad had `renderer_drift`). Both read the same font's advances, and
  screenshots show they agree, but a drift would only show visually.

### Risks
- Bevy breaks APIs roughly every four months, and plugins (`bevy_vector_shapes`)
  lag. Upgrades wait on them.
- `rand`'s `SmallRng` is a different algorithm on wasm32, so a seed plays
  differently on web than natively (true under macroquad too).

## Alternatives Considered

**A side-by-side `robot-buddy-bevy` crate (strangler).** The spike plan proposed it,
with macroquad deploying until parity. Rejected: because `Game::step` is pure and
rendering is a thin recording layer, the in-place port reached parity in one step.
Two crates would have doubled CI time and every feature PR for no safety gained.

**Rewrite rendering as Bevy sprites/meshes.** Would retire `draw.rs`, but the art is
~5k lines of procedural shape calls with no assets behind it. Real sprite art is the
better reason to touch it.

**Stay on macroquad.** It works and is small. The cost is everything in the census,
plus browser glue in hand-written JS.
