# ADR-004: Declarative UI Layout with a Swappable Engine

**Status:** Accepted and implemented (shop, swag, quest, challenge, dialogue, settings)
**Date:** 2026-09-23
**Deciders:** Veesh, Claude

## Context

UI text kept landing on top of other text. Every panel in `robot-buddy-game/src/ui/`
had a pure `layout(state, screen)` that returned **only click rects**, while its
`draw_*` hard-coded its own y offsets. Nothing connected the two, so nothing stopped
collisions:

- **Shop:** the purchase message was drawn at a fixed `p.y + p.h - 76`; Hermie's
  six-item shelf put row 6 at y 440–502, directly under it (the reported bug).
- **Swag:** rows at `108 + 64i` vs the same fixed message line — five items collide.
- **Quest:** the retry message sat inside the answer tiles; the wrapped body flowed
  unbounded into the bottom-anchored buttons.
- **Challenge:** `layout()` ignored the extra height of a wrapped word problem that
  `draw()` added, so hit rects sat ~30px above the drawn buttons; the teaching
  visual grew into "= N".
- **Dialogue:** a fixed 170px box; a 4th line hit "SPACE >"; wrapping guessed 15px
  per character instead of measuring.
- **Settings:** "Text speed" and the note were placed outside `layout()`; with the
  parent section open the panel was ~800px tall and ran off a 960×720 window.

There was also duplication: `UiRect` defined six times, `fitted_by`/`centered`
copy-pasted in leap and descent, three different text-wrap implementations.

The constraints: layout must stay computable in `Game::step` (pure, headless — ADR-002);
the kid-styled look (hand-drawn panels, big tiles, custom art) must survive; and
swapping the in-house layout engine for an externally maintained one (taffy) later
must be a small, localized change.

## Decision

Add `robot-buddy-game/src/ui/layout/`. Panels **describe structure and sizes** as a
node tree; an **engine** turns the tree into rects; a shared **resolver** fits text
and produces a `Frame` that both drawing and click handling read.

```text
 panel code                    ui::layout                              panel code
 build Node<Id> tree ─► LayoutTree ─► LayoutEngine::compute ─► Frame::resolve ─► draw: loop over Frame
 (col/row/text/button/…)  (id-free arena)  (DefaultEngine)     (text fit, ids)   click: frame.hit_at(x, y)
                                                ▲                   ▲
                                                └── TextMetrics ────┘ (FontMetrics: bundled font via fontdue)
```

| Piece | File | Role |
|---|---|---|
| Vocabulary | `node.rs` | `col`, `row`, `text`, `region`, `spacer`, `button`; `Style` = direction, padding, gap, Px/Auto size, min/max, grow/shrink, align-items/self, justify-content. **Flexbox only** — no grid, no absolute positioning. |
| Text policy | `node.rs` `Fit` | **Required** on every text leaf: `Shrink{min}`, `Wrap{min,max_lines}`, `ShrinkThenWrap{..}`, `Ellipsis`. "Runs off the panel" can't happen by omission. |
| Engine seam | `engine.rs` | `trait LayoutEngine { fn compute(&self, &LayoutTree, bounds, &dyn TextMetrics) -> Vec<Option<UiRect>> }` — object-safe, not generic over panel ids. Contract: children inside their parent's padding box, siblings never overlap, anything else is `None` (clipped). |
| In-house engine | `flow.rs` | `FlowEngine`: single-line flexbox subset with CSS grow/shrink/freeze semantics; min-content for text = what its `Fit` can shrink to; clips instead of overflowing. |
| Text | `text.rs` | One `shape(spec, w, h)` decides both "how big is this text" (measure) and "how is it drawn" (resolve), so they can't disagree. Shared by every engine. |
| Metrics | `metrics.rs` | `TextMetrics`. `FontMetrics` parses the exact bytes `crate::text` gives macroquad (`assets/unifont-subset.ttf`) with fontdue — macroquad's own rasterizer — and sums advances the way `macroquad::text::measure_text` does. Headless and identical to what's drawn. `MacroquadMetrics` asks macroquad directly; debug builds cross-check it against `FontMetrics` at paint time and warn on drift. |
| Output | `frame.rs` | `Frame<Id>`: placed texts (lines + baselines + fitted size), boxes, regions, hit targets keyed by panel-defined ids; `hit_at`, `rect(id)`, `clipped()`. |
| Paging | `page.rs` | Kids can't scroll and flexbox can only clip, so list paging lives **above** the engine: lay out, and if any row was clipped, rebuild with fewer rows plus "More >" until every page fits. Reads only `Frame::clipped`, so it survives an engine swap. |
| Painter | `paint.rs` | The only code that turns a migrated panel's frame into pixels: `text`, `fill`, `outline`, `round_rect`, `dim`, `blink`, and a `Canvas` for custom art inside a region. |
| Sanity | `sane.rs` | `assert_sane(frame, bounds)`: everything inside bounds and its parent; no two elements overlap unless nested; nothing clipped; no text overflowed its policy. |

Enforcement:

- `tests/layout_sweep.rs` runs `assert_sane` over every migrated panel × 4 screens
  (480×800, 640×480, 960×720, 1600×900) × awkward data (both shop catalogs + long
  messages, 30-pearl trade, every challenge phase for generated bands 1–10 plus a long
  word problem, 4 long quest options, the full 9-piece swag wardrobe, a 4-line
  dialogue) and checks every list row is reachable on some page.
- `tests/layout_discipline.rs` parses each migrated panel's source and fails if it calls
  macroquad's raw draw/measure functions, `screen_width/height` or `get_time` — so a
  hand-placed `y + 76` can't come back. Migrating a panel = adding it to `MIGRATED`.
- Harness helpers (`tests/common`) click through the same frames `step` hit-tests
  (`game.shop_layout(SCREEN)`, `game.swag_layout(SCREEN)`), and page through lists
  like a kid would.

One shared `UiRect` (`layout/rect.rs`) replaces the six copies; `fit_size_by` and
`paint::centered_fitted` replace the leap/descent duplicates.

### Migrated vs not

Migrated: shop (catalog, buy, trade pile, color picker), swag, quest, challenge
(incl. teaching; `visuals::extent` reserves the CRA visual's space), dialogue, settings.
Not migrated (use the shared `UiRect` only): leap, descent, title screen, interaction
menu, kenken, sudoku, patterns, balance, shooter, HUD.

## Swapping in taffy

Keep it **flexbox-only**: `taffy = { version = "0.x", default-features = false,
features = ["std", "flexbox", "taffy_tree"] }` measured at about +79KB WASM
(opt-level s + LTO), versus about +277KB with grid and block layout.

Files touched:

1. `robot-buddy-game/Cargo.toml`: add the dependency above.
2. New `robot-buddy-game/src/ui/layout/taffy.rs` (~120 lines):
   ```rust
   pub struct TaffyEngine;
   impl LayoutEngine for TaffyEngine {
       fn compute(&self, tree: &LayoutTree, bounds: UiRect, m: &dyn TextMetrics) -> Rects {
           // 1. Build bottom-up: every TreeNode becomes a taffy node carrying its
           //    arena index as context. Map Style field-for-field (table in
           //    layout/mod.rs): direction→flex_direction, padding, gap,
           //    width/height→size, min_*/max_*→min_size/max_size,
           //    flex_grow/shrink, align_items/self, justify_content.
           // 2. compute_layout_with_measure(root, bounds size, |input, _, ctx, style|
           //        compute_leaf_layout(input, style, |_, _| 0.0, |known, avail| match kind {
           //            Text(spec) => width: MinContent → text::min_width,
           //                                 otherwise text::natural_width (capped to avail);
           //                          height: text::natural_height(spec, width)
           //                                  (min-content: text::min_height),
           //            _ => Size::ZERO }))
           // 3. Walk the tree summing `layout(node).location` into absolute rects.
           // 4. Clip post-pass (keeps the engine contract): a child whose box ends
           //    past its parent's padding box → None for its whole subtree.
       }
   }
   ```
3. `robot-buddy-game/src/ui/layout/mod.rs`: `pub mod taffy;` and
   `pub type DefaultEngine = taffy::TaffyEngine;`.
4. Optionally delete `flow.rs` (408 lines including its tests).

Nothing in any panel, the painter, `Frame`, paging, text fitting, the sweep, or the
harness changes. Run `cargo test`: the sweep is engine-agnostic and says whether the
new engine keeps the contract. Differences to expect: taffy lets children overflow
where `FlowEngine` clips, so the post-pass in step 4 matters. Taffy's automatic
minimum size also differs from our explicit `min_*` in a few corners, and the sweep
will point at any panel that needs an explicit `min_h`.

## Alternatives Considered

**Keep hand-placed offsets and fix each bug.** Every fix would be another magic
number that the next catalog item or longer string breaks. The bugs came from
drawing and hit-testing being two unconnected computations, and patching offsets
leaves them unconnected.

**egui (via egui-macroquad).** egui is immediate-mode: layout happens while painting,
inside its own input and frame loop, so `Game::step` couldn't hit-test headlessly
(ADR-002) without running egui's context. It would add roughly a megabyte of WASM,
and it brings its own theming and input model that fight the kid-styled custom art
and `FrameInput`. It solves "a desktop tool UI", not "big kid-sized tiles with
hand-drawn art".

**macroquad's built-in `ui` module.** Also immediate-mode, and its layout is
window/group placement with a fixed skin, not constraint-based. It has no
measure-then-place pass we could run in `step`, and styling it to look like the
game costs more than the layout it saves.

**Adopt taffy now.** It's viable (a spike laid Hermie's shop out correctly), but it
costs ~79KB for a UI that needs a small subset of flexbox, and it still needs our
text fitting, clip post-pass, paging policy and sanity sweep around it. So we built
those taffy-shaped with a ~350-line engine behind the seam, and made the swap a
one-file change for when the UI outgrows it.

**Generic engine over panel ids.** The first cut had `compute<Id>(&Node<Id>)`, which
compiled one copy of the engine per panel. The id-free `LayoutTree` arena removed
that (−8.5KB) and matches taffy's tree-building API.

## Consequences

### Positive
- The reported shop bug and the audited overlaps are fixed, and a generic test now
  catches the whole class across screens and data.
- Hit rects equal drawn rects because both come from one `Frame`. The challenge
  drift can't happen again.
- Text is measured with the real font headlessly, so tests see the same wrapping
  players do.
- Latent bugs the sweeps found are fixed: the Hermie trade label escaped the panel
  at 480px (caught by the taffy spike's sweep); the settings parent section ran off a 960×720 window; the trade pile
  had no height budget; long quest options didn't fit.

### Negative
- About +83KB WASM (layout module, per-panel trees and frames).
- `FontMetrics` parses the 440KB font a second time (macroquad parses its own copy)
  on first layout, which costs some memory and a one-off parse.
- Two small behavior changes needed to fit: the parent section of Settings now
  replaces the kid rows while open, and very short windows page long shop/swag
  lists behind "More >".

### Risks
- `visuals::extent` has to mirror `visuals::draw_visual`'s geometry by hand. If
  someone changes one without the other, the visual can bleed out of its reserved
  region. The painter's `Canvas` warns in debug builds for panel art, but
  `visuals.rs` isn't migrated yet.
- The `Fit` minimum sizes and `min_h` floors are tuned so the sweep passes at
  640×480. New content may need new floors, and the sweep will say where.
