# ADR-004: Declarative UI Layout with a Swappable Engine

**Status:** Accepted and implemented (shop, swag, quest, challenge + CRA visuals, dialogue, settings)
**Date:** 2026-09-23 (revised the same day: flexbox semantics, hardened checks, a taffy differential test run and retired)
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
must be a small, localized change. The owner prefers external libraries where they
fit, so the in-house engine must not grow behaviour taffy doesn't have.

## Decision

Add `robot-buddy-game/src/ui/layout/`. Panels **describe structure and sizes** as a
node tree; an **engine** computes the exact CSS flexbox layout; shared post-passes
**round** it to whole pixels and **clip** whatever overflowed; a shared **resolver**
fits text and produces a `Frame` that both drawing and click handling read.

```text
 panel code                                                                      panel code
 build Node<Id> ─► LayoutTree ─► LayoutEngine::compute ─► round_edges ─► clip ─► Frame::resolve ─► draw: loop over Frame
 (col/row/text/…)  (id-free)     (exact CSS flexbox,       (shared)      (shared:  (text fit, ids)   click: frame.hit_at(x, y)
                                  overflow allowed)                       overflow
                                        ▲                                 → None)        ▲
                                        └────────── TextMetrics (FontMetrics: bundled font via fontdue) ──┘
```

| Piece | File | Role |
|---|---|---|
| Vocabulary | `node.rs` | `col`, `row`, `text`, `region`, `spacer`, `button`; `Style` = direction, padding, gap, `Dim` (Auto / Px / Percent) size, `Len` (Px / Percent) min/max, grow/shrink, align-items/self, justify-content. **Flexbox only** — no grid, no absolute positioning, no wrap. Every field maps 1:1 onto `taffy::Style` (table in `layout/mod.rs`). |
| Text policy | `node.rs` `Fit` | **Required** on every text leaf: `Shrink{min}`, `Wrap{min,max_lines}`, `ShrinkThenWrap{..}`, `Ellipsis`. "Runs off the panel" can't happen by omission. |
| Reserved lines | `node.rs` `reserve_lines(n)` | A text leaf holds room for `n` lines even while empty. Used for slots that fill in later — the challenge's "Hmm, not quite!" feedback — so the answer buttons below don't jump out from under the kid's finger when it appears. Empty text takes no ink, so the sweep's overlap check ignores an empty reserved box. |
| Engine seam | `engine.rs` | `trait LayoutEngine { fn compute(&self, &LayoutTree, bounds, &dyn TextMetrics) -> Vec<UiRect> }` — object-safe, not generic over panel ids. Contract: the root fills `bounds`; **exact (unrounded) CSS flexbox**, including overflow. |
| Shared post-passes | `engine.rs` | `round_edges`: each absolute edge to the nearest pixel (monotonic, so nothing that fit starts overflowing). `clip`: a node that leaves its parent's content box is `None` with its subtree. Both run for every engine. |
| In-house engine | `flow.rs` | `FlowEngine`: single-line flexbox with CSS semantics — no cross-axis cap, fit-content auto cross sizes, Center/End overflow the start side, automatic minimums, min-content contributions, and a step-for-step port of taffy's `resolve_flexible_lengths`. Held to taffy by a differential test (below). |
| Text | `text.rs` | One `shape(spec, w, h)` decides both "how big is this text" (measure) and "how is it drawn" (resolve), so they can't disagree. Shared by every engine: it is the measure function a taffy engine calls. |
| Metrics | `metrics.rs` | `TextMetrics`. `FontMetrics` parses the exact bytes `crate::text` gives macroquad (`assets/unifont-subset.ttf`) with fontdue — macroquad's own rasterizer — and sums advances the way `macroquad::text::measure_text` does. `renderer_drift()` compares it with `MacroquadMetrics` (macroquad itself) on a probe at startup. |
| Output | `frame.rs` | `Frame<Id>`: placed texts (lines + baselines + fitted size), boxes, regions, hit targets keyed by panel-defined ids; `hit_at`, `rect(id)`, and `clipped()` — **every** clipped text / region / id'd node, with or without an id. |
| Paging | `page.rs` | Kids can't scroll and flexbox can only overflow, so list paging lives **above** the engine: lay out, and if any row was clipped, rebuild with fewer rows plus "More >" until every page fits. Reads only `Frame::clipped`, so it survives an engine swap. |
| Painter | `paint.rs` | The only code that turns a migrated panel's frame into pixels: `text`, `fill`, `outline`, `round_rect`, `dim`, `blink`, and a `Canvas` (circle, ring, line, rect, rect lines, text) for custom art inside a region that warns in debug builds when art strays outside it. |
| CRA visuals | `ui/visuals.rs` | `visuals::plan(challenge, max_w)` builds the visual as a display list in local coordinates, squeezed (lengths and label sizes together) until it fits `max_w`. `extent` is that list's bounding box; `draw` paints that list through a `Canvas` bound to the challenge's `Visual` region. Measuring and drawing are one computation. |
| Sanity | `sane.rs` | `assert_sane(frame, bounds)`: everything inside bounds and its parent; every text line inside its box; no two elements overlap unless nested; nothing clipped; no text overflowed its policy. |

One shared `UiRect` (`layout/rect.rs`) replaces the six copies; `fit_size_by` and
`paint::centered_fitted` replace the leap/descent duplicates.

### Flexbox semantics, not "flexbox-ish"

The first `FlowEngine` capped every child's cross size to its container and clamped
Center/End justification at zero on overflow. Browsers and taffy do neither, and the
panels had quietly come to depend on the cap to fit a narrow phone — so a taffy swap
would have broken them. We decided the in-house engine implements **CSS flexbox
exactly** for its vocabulary, and anything that doesn't fit **overflows**, then the
shared `clip` pass turns overflow into "didn't fit" that paging and the sweep see.

What that means when writing a panel:

- **Width is the cross axis of `centered_on_screen`, and CSS never shrinks on the
  cross axis.** A panel is `.w_pct(1.0).max_w(PANEL_W)` — "PANEL_W, or all the room
  there is" — never `.w(PANEL_W)`, which is 760px wide on a 360px phone and clips.
- **Rows of buttons that give up height** on a short screen are
  `row().align(Align::Stretch).h(H).min_h(FLOOR)` with auto-height children: the row
  shrinks on its parent's main axis, the buttons stretch to it. A `.h(H)` on the
  buttons themselves would overflow the shrunken row.
- **The CSS "min-height: 0" rule.** A container's minimum is its children's
  *preferred* sizes, not their `min_h`s. To let a nested column (or the panel itself)
  squeeze its children down to their floors, give it `.min_h(0.0)`. The challenge
  panel, the shop's swatch grid and settings' nested sections do.
- Percentages (`w_pct`, `h_pct`, `min_h_pct`, `max_h_pct`) resolve against the
  parent's content box; while measuring content they act as `auto`, as in CSS.
- `Justify::End` on overflow pushes the *first* children off the start side;
  `Center` spills both ways; `SpaceBetween` with no room acts as `Start`.

Pinned by `flow.rs` unit tests (cross overflow, percent + max, justify on overflow,
rounding, fit-content). A differential test against taffy also checked these fixtures,
plus the nested min-height rule and inner-basis shrinking, before it was retired (below).

## What the checks catch — and what they don't

**`tests/layout_sweep.rs`** (bodies in `tests/sweep/`) runs every migrated panel ×
5 screens (360×640 phone, 480×800, 640×480, 960×720, 1600×900) × awkward data: both
shop catalogs with long messages, buying/trading/color-picking, a 30-pearl trade,
every challenge phase for 60 generated challenges (bands 1–10) plus a long word
problem, the full swag wardrobe, 4-line dialogue with a long speaker name, both
settings pages, every quest beat with long options. On each frame it asserts
`assert_sane`, and additionally:
- every list row is reachable on some page;
- the CRA visual `visuals::draw` would paint (the same `plan`) fits its `Visual`
  region — so the number-bond phone overflow is caught (injecting "no squeeze" fails
  it at 360×640 on a 12+5 bond);
- answer buttons don't move when feedback appears; tapping a drawn button answers it.

It catches: overlap, anything escaping its parent or the screen, any clipped node
(anonymous texts and regions included — a clipped 20px spacer inside the answer
buttons at 640×480 was the first find), text that overflowed its `Fit`, a visual
that outgrew its region. It does **not** catch: panels that aren't migrated; data or
states the sweep doesn't enumerate (a new view needs a new sweep case); screens
between or beyond the five (e.g. 320px wide, landscape phones); custom art other
than the visuals straying outside its region (only the debug-build `Canvas` warning);
drift between headless metrics and the renderer (see metrics below); whether text
at a `Fit` floor is still big enough for a four-year-old.

**A differential test against taffy (retired).** `tests/layout_taffy.rs` ran the same
sweep bodies with every `layout()` call also computed by taffy 0.14 and required
FlowEngine and taffy to agree on every node's **unrounded** rect to 0.01px (about
8,600 layouts, plus CSS fixtures). It found three real divergences, all fixed:
container min-content counted `min_h` instead of preferred sizes; shrink was weighted
by the outer instead of the inner basis; and taffy's own rounding of relative offsets
can push a snug child a pixel out of its parent, which is why rounding is a shared
pass. It was then **removed**: it doubled `cargo test` time (~20s) to re-prove an
agreement that only changes when `flow.rs` does. To bring it back, restore
`tests/layout_taffy.rs`, the `taffy` dev-dependency and the debug-only
`layout::with_engine` hook from commit b58cb74 (`git show b58cb74`), and re-run it
whenever `flow.rs` or the node vocabulary changes.

**`tests/layout_discipline.rs`** parses each migrated file (`MIGRATED`, now including
`visuals.rs`) with syn and fails on:
- raw macroquad drawing/measuring/camera/clock calls — any `draw` / `draw_*` /
  `gl_*` by prefix (so `draw_rectangle_lines_ex`, `draw_hexagon`,
  `draw_multiline_text_ex`, `draw_texture*`, future ones) plus `measure_text`,
  `get_text_center`, `screen_width/height`, `get_time`, camera and context fns;
- the same resolved through `use … as alias` imports, passed as a function value, or
  inside a macro's tokens (`format!`, `vec![]`, custom macros);
- `draw*` helpers from **unmigrated** modules (`leap::draw`, `sprites::…::draw_player`)
  — allowed only if the callee is the file's own fn, the painter, or another
  migrated module;
- hand-made rects: `UiRect::new`, `UiRect { .. }`, `.inset(..)`, `.expand(..)` outside
  a three-entry commented `COORD_ALLOW` (stale entries fail too).

It does **not** catch: arithmetic on a frame's rect fields fed to paint
(`r.x + 6.0`); raw-drawing helpers whose names don't start with `draw`; macros
defined elsewhere that draw internally; method calls named `draw` on objects.

**Metrics.** Layout measures Unifont headlessly. `text::init` now checks the
renderer against `FontMetrics` on a probe (ASCII, digits, `− × ÷`, `★`, eight sizes)
and returns `Err` if the font didn't load or widths differ by more than half a pixel
— e.g. under `high_dpi` at a fractional scale, where macroquad rasterizes at
`ceil(size × dpi)`. The game calls `init_or_die` (debug: panic; release: error log);
the screenshot example (`high_dpi: true`) prints a loud warning. It does not catch a
glyph outside the probe drifting; the per-paint debug check (`paint.rs`) still warns
on the first one it sees.

### Migrated vs not

Migrated: shop (catalog, buy, trade pile, color picker), swag, quest, challenge
(incl. teaching and the CRA visuals), dialogue, settings.
Not migrated (use the shared `UiRect` only): leap, descent, title screen, interaction
menu, kenken, sudoku, patterns, balance, shooter, HUD.

## Swapping in taffy

Verified end to end (before the differential test was retired): with the steps below
the whole suite (unit, sweep, differential, 64 story tests) passed, and the WASM grows by **~37KB** (1,677,336 → 1,714,783 bytes,
opt-level s + LTO, FlowEngine dropped by the linker).

1. `robot-buddy-game/Cargo.toml`: move the dev-dependency
   `taffy = { version = "0.14", default-features = false, features = ["std", "flexbox", "taffy_tree"] }`
   to `[dependencies]`.
2. Recover `TaffyEngine` from `tests/layout_taffy.rs` at commit b58cb74 into a new
   `src/ui/layout/taffy.rs` (~110 lines; derive `Default`; import from `super::`).
   What it does:
   - builds the `TaffyTree<usize>` bottom-up from the `LayoutTree` arena, each node
     carrying its arena index as context, mapping `Style` field for field;
   - forces the root's `size` to `bounds` (the engine contract: the root fills it);
   - calls `disable_rounding()` — rounding is the shared `round_edges` pass;
   - lays out with `compute_layout_with_measure` + `compute_leaf_layout`, measuring
     text leaves with `text.rs`: width = known, else `MinContent → min_width`,
     `MaxContent → natural_width`, `Definite(w) → natural_width.min(w).max(min_width)`;
     height = known, else `MinContent → min_height`, otherwise `natural_height`;
   - sums relative `location`s into absolute rects. No clip pass of its own: the
     shared `clip` runs after it.
3. `src/ui/layout/mod.rs`: `pub mod taffy;` and `pub type DefaultEngine = taffy::TaffyEngine;`.
4. Delete `flow.rs`, or keep it and revive the differential test to diff the two.

Nothing in any panel, the painter, `Frame`, paging, text fitting, the sweep, or the
harness changes. Differences to expect: none on the swept panels as of b58cb74 (the
differential test said so to 0.01px); if `flow.rs` changed since, revive the test
first. Taffy alignment also has `safe` variants (`AlignItems::SAFE_CENTER` …) our
vocabulary doesn't expose; adding one to `node.rs` means adding it to `FlowEngine` too.

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

**Adopt taffy now.** Viable and now proven (see the swap above): ~37KB. We still
need our text fitting, rounding/clip passes, paging policy and sanity sweep around
it, and FlowEngine is ~440 lines (plus tests) that agree with it exactly on everything we lay
out. Kept as the default for size; the swap is a one-file change whenever the UI
wants something taffy has and FlowEngine doesn't.

**Keep FlowEngine's lenient semantics (cross-axis cap, zero-clamped justify).**
Friendlier for narrow screens, but it's behaviour no browser or taffy has: panels
written against it break on a swap, and "what would CSS do" stops being a reliable
way to reason about a panel. Rejected in the revision.

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
  players do; a renderer that disagrees fails at startup instead of silently.
- The CRA visuals can't outgrow their slot: measuring and drawing are one display
  list, and the number bond now squeezes on a phone instead of overflowing it.
- The engine is CSS flexbox, checked against taffy, so a panel can be reasoned about
  (or prototyped in a browser) as CSS, and the taffy swap is proven, not promised.
- Latent bugs the sweeps found are fixed: the Hermie trade label escaped the panel
  at 480px; the settings parent section ran off a 960×720 window; the trade pile
  had no height budget; long quest options didn't fit; the number bond overflowed a
  360px phone; a clipped spacer in the answer buttons at 640×480.

### Negative
- About +85KB WASM for the layout module, per-panel trees and frames, and the
  visuals display list.
- `FontMetrics` parses the 440KB font a second time (macroquad parses its own copy)
  on first layout, which costs some memory and a one-off parse.
- CSS's min-height rule is a real gotcha: a column that should squeeze needs
  `.min_h(0.0)`, and the sweep only finds a missing one on a screen short enough.
- The challenge panel lays out twice when a visual shows (once to learn the slot's
  width, once to reserve the visual's height at that width).

### Risks
- The sweep only knows the states it enumerates. A new panel view, or a new piece of
  data with a longer string, needs a sweep case, or its overflow goes unnoticed until
  a kid sees it.
- The `Fit` minimum sizes and `min_h` floors are tuned so the sweep passes at
  640×480 and 360×640. New content may need new floors, and the sweep will say where.
- Custom art outside `visuals.rs` (the pearl pile, the star burst, sprites drawn at
  a region's corner by `game.rs`) is only checked at runtime by the debug `Canvas`.
