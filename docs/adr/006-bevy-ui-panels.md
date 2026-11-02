# ADR-006: Panels on Bevy UI

**Status:** Proposed (built on `claude/bevy-ui-panels` to compare with ADR-004's layout on taffy)
**Date:** 2026-10-22
**Deciders:** Veesh, Claude

## Context

After the Bevy port (ADR-005), the panels still described a node tree that our own
`ui::layout` laid out on taffy, painted through our own painter and hit-tested with
our own `Frame`. That is a small layout engine we maintain. The owner's position:
"I care about making math content. I don't care about making a layout engine." The
layout sweep is not sacred if a framework keeps us from doing dumb things.

Bevy UI is the same taffy underneath, plus nodes as entities, text measuring and
wrapping, borders, rounded corners, gradients, scroll areas, z-order and picking.

## Decision

Every panel (dialogue, challenge, settings, quest, shop, swag, Shelly's leap strip,
the dive shaft, the shooter's footer) is a Bevy UI panel. `src/ui/layout/` is
deleted; `metrics.rs` (headless font widths) and `rect.rs` (`UiRect`, used by the
hand-drawn logic-puzzle boards) moved up to `src/ui/`.

```text
Game::step(FrameInput) ──► Game::ui_views(screen) ──► UiViews (plain data)
      ▲                                                     │ reconcile::<P>
      │ FrameInput.ui_press: Option<UiAction>                ▼
  bevy_picking ◄──────────────── panel entities (Bevy UI, built by P::bundle(view))
```

- **A panel is a view and a bundle** (`ui::kit::Panel`). The game builds each
  panel's view (`ChallengeView`, `ShopPanelView`…) from its state every frame; a
  panel is respawned only when its view changes. `bundle(view)` is pure.
- **Taps carry names, not coordinates.** Buttons carry an `Action(UiAction)`
  (`UiAction::Shop(ShopId::Done)`…). A `Pointer<Press>` observer puts the
  innermost one in `FrameInput.ui_press`; `Game::step` matches on it. `click_on_ui`
  tells the map a tap was on a panel, so it never also walks.
- **Art in a panel** (the CRA visual, the kid in Bolt's mirror, the pearl pile, the
  dive shaft) is an `Art(ArtId)` slot. Each slot gets an offscreen camera on its
  own render layer drawing into a texture the slot shows; `Game::render` paints
  the slot through `draw::art(layer, ..)` in the slot's own coordinates. Art sits
  on its panel and under anything stacked above it.
- **Text that fits** is the one thing Bevy lacks (it wraps but never shrinks):
  `kit::Shrink` steps a font down to fit its box's width, then wraps; `squeeze`
  texts also shrink while the box holding them spills out of its max height.
  Helpers: `fitted`, `fitted_squeezed`, `filled`.
- **Lists scroll** (`kit::scroll_list`) instead of paging, and keep their place
  when the panel is rebuilt.

### How we keep layouts sound without our own engine

`ui::probe::UiProbe` is a windowless Bevy app with the game's panels
(`ui::PanelsPlugin`). Hand it a `UiViews` and it runs Bevy's own layout and
returns a `Snapshot` of every button, text and art slot in screen pixels.
`Snapshot::problems` reports what a kid would see as broken:

| Check | Catches |
|---|---|
| button off screen (or off its scroll list across) | a panel taller or wider than the window |
| buttons overlapping (as far as they're visible) | stacked or squashed rows |
| text whose glyphs run past its box | a label too long even at its smallest size |
| text off screen | a panel spilling past the window |
| text running over a button it isn't on | a message wrapping onto Done |
| a visible box whose content spills out of the bottom | a column with more in it than room |
| art slot off screen | — |

It doesn't catch: text clipped by a scroll list (by design), colours, art drawn
outside its own slot (the challenge sweep checks the CRA visual's plan against
its slot), anything mid-animation.

Two places use it:

- **Every story-test tap** (`Harness::tap(UiAction)`) lays out the screen as it
  stands, asserts it has no problems and that the button is there, scrolls its
  list to it if needed, and taps its centre. Story tests can't press a button that
  isn't really on screen.
- **`tests/panels.rs`** sweeps every panel × five screens (360×640 phone up to
  1600×900) × awkward data.

## Alternatives Considered

- **Keep ADR-004's layout on taffy** (PR #38 as it stood). Works, and its checks
  run without any Bevy app. But it is ~2,000 lines of node builder, frame, painter,
  paging, text fitting and sanity checks we own, and every new panel speaks our
  dialect rather than Bevy's.
- **Bevy UI with no headless checks.** Bevy guarantees a consistent layout, not a
  sensible one: on the first sweep it showed panels taller than a phone, messages
  wrapping over Done and a word problem pushing buttons off a 640×480 window. The
  probe is needed either way; it is small because it only reads what Bevy computed.

## Consequences

### Positive
- No layout engine of ours. Panels are ordinary Bevy UI that any Bevy doc or
  example applies to; scroll areas, gradients, rounded corners and z-order are free.
- No hit rects: a tap names its button. Tests tap by name and still prove the
  button is on screen.
- Dialogue and panels draw above the hand-drawn boards by z-order, not by draw
  order.

### Negative
- Shrink-to-fit is ours (`kit::Shrink`), approximate (it steps by ratio, not by
  measuring every size) and settles over a frame or two; text stays hidden until
  it does.
- Layout checks need a Bevy app (`UiProbe`), slower than a pure function: the
  panel sweep takes seconds in a debug build, not milliseconds.
- Each art slot costs an offscreen camera and a texture.

### Behaviour changes from ADR-004's panels
- Shop and swag lists scroll (drag or wheel) instead of paging with More / Back.
- On short windows the parent section of settings scrolls.
- Answer buttons put the key hint in the corner instead of above the number.
