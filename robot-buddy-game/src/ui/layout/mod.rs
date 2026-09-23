//! UI layout: describe a panel as a tree, get back a [`Frame`] of placed rects
//! that both drawing and click handling read. See `docs/adr/004-ui-layout.md`.
//!
//! ```text
//!  panel code                       this module                      panel code
//!  ──────────                       ───────────                      ──────────
//!  build Node tree ──► LayoutEngine::compute ──► Frame::resolve ──► draw: loop over Frame
//!  (col/row/text/…)    (DefaultEngine = Flow)    (text fit, ids)    click: frame.hit_at(x, y)
//!                            ▲                         ▲
//!                            └──── TextMetrics ────────┘  (FontMetrics: bundled font via fontdue)
//! ```
//!
//! Layers, and who may depend on what:
//! * [`node`] — the panel-facing vocabulary. Flexbox-only, taffy-shaped.
//! * [`engine`] — the seam: `LayoutEngine::compute(tree, bounds, metrics) -> rects`
//!   (exact CSS flexbox, overflow allowed), then the shared `round_edges` + `clip`.
//! * [`flow`] — the in-house engine (`FlowEngine`).
//! * [`text`] — fit/wrap/measure for text leaves; shared by every engine.
//! * [`frame`] — the resolved output (texts with lines + baselines, boxes,
//!   regions, hit targets keyed by panel-defined ids).
//! * [`page`] — list paging policy, above the engine (reads `Frame::clipped`).
//! * [`sane`] — `assert_sane`, the generic no-overlap test.
//! * [`paint`] — macroquad helpers for drawing a frame (render-only).
//!
//! ## Vocabulary → taffy mapping
//!
//! | `node::Style`          | `taffy::Style`                                   |
//! |------------------------|--------------------------------------------------|
//! | `direction`            | `flex_direction` (Row / Column)                  |
//! | `padding: Edges`       | `padding: Rect<LengthPercentage::length>`        |
//! | `gap`                  | `gap: Size { width: g, height: g }`              |
//! | `width` / `height`     | `size: Size<Dimension>` (auto / length / percent) |
//! | `min_*` / `max_*`      | `min_size` / `max_size` (`LengthPercentageAuto`) |
//! | `flex_grow/shrink`     | `flex_grow` / `flex_shrink`                      |
//! | `align_items/self`     | `align_items` / `align_self` (START/CENTER/END/STRETCH) |
//! | `justify_content`      | `justify_content` (START/CENTER/END/SPACE_BETWEEN) |
//! | `Content::Text(spec)`  | leaf with context; measure = `text::natural_width` / `natural_height` / `min_*` |
//! | `Content::Region`      | plain leaf                                       |
//! | root fills `bounds`    | root `size` = bounds (its own size style ignored) |
//! | whole pixels           | `disable_rounding()`; the shared `engine::round_edges` rounds |
//! | overflow → clipped     | `overflow: Visible` (default); the shared `engine::clip` post-pass |
//!
//! `tests/layout_taffy.rs` holds a working `TaffyEngine` built on this table
//! and checks `FlowEngine` against it over the whole sweep.

pub mod engine;
pub mod flow;
pub mod frame;
pub mod metrics;
pub mod node;
pub mod page;
pub mod paint;
pub mod rect;
pub mod sane;
pub mod text;

use std::fmt::Debug;

pub use engine::{LayoutEngine, LayoutTree};
pub use frame::{Clipped, Element, Frame, Kind, PlacedLine, PlacedText};
pub use metrics::{FontMetrics, TextMetrics};
pub use node::{
    button, col, gap_box, region, row, spacer, text, Align, Dim, Direction, Edges, Fit, Justify, Len, Node, Style,
    TextAlign,
};
pub use page::{paged, Page};
pub use rect::UiRect;
pub use sane::{assert_sane, check_sane};

/// The engine every panel uses. Swapping engines = change this alias.
pub type DefaultEngine = flow::FlowEngine;

/// Lay `root` out inside `bounds` with the default engine and the bundled
/// font's metrics. Pure: safe to call from `Game::step` and headless tests.
pub fn layout<Id: Copy + PartialEq + Debug>(root: &Node<Id>, bounds: UiRect) -> Frame<Id> {
    layout_with(&DefaultEngine::default(), FontMetrics::bundled(), root, bounds)
}

/// [`layout`] with an explicit engine and metrics (for tests / engine swaps).
pub fn layout_with<Id: Copy + PartialEq + Debug, E: LayoutEngine + ?Sized>(
    engine: &E,
    metrics: &dyn TextMetrics,
    root: &Node<Id>,
    bounds: UiRect,
) -> Frame<Id> {
    let tree = LayoutTree::new(root);
    let mut raw = engine.compute(&tree, bounds, metrics);
    engine::round_edges(&mut raw);
    let rects = engine::clip(&tree, &raw);
    Frame::resolve(root, &rects, bounds, metrics)
}

/// The whole screen as a layout bounds.
pub fn screen_rect(screen: (f32, f32)) -> UiRect {
    UiRect::new(0.0, 0.0, screen.0, screen.1)
}

/// Wrap a panel in a full-screen root that centers it with `margin` on every
/// side. Height is the root's main axis, so a panel's `.h()` shrinks to fit a
/// short screen like any flex item. Width is the cross axis, where CSS never
/// shrinks anything: give the panel `.w_pct(1.0).max_w(PANEL_W)` ("PANEL_W,
/// or all the room there is"), not `.w(PANEL_W)`, or it overflows (and is
/// clipped) on a narrow phone.
pub fn centered_on_screen<Id>(panel: Node<Id>, margin: f32) -> Node<Id> {
    col().pad(margin).align(Align::Center).justify(Justify::Center).child(panel)
}

/// Screens every sweep test covers (portrait phone, small, default, wide).
/// 360×640 is a small phone in portrait — the smallest real device a kid will hold.
pub const SWEEP_SCREENS: [(f32, f32); 5] = [(360.0, 640.0), (480.0, 800.0), (640.0, 480.0), (960.0, 720.0), (1600.0, 900.0)];

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Debug)]
    enum T {
        Panel,
        Title,
        Row(usize),
        Label(usize),
        More,
    }

    fn panel(page: Page) -> Node<T> {
        let rows = page.rows().map(|i| {
            button(T::Row(i), T::Label(i), format!("Row number {i}"), 24, Fit::shrink(14)).h(50.0).min_h(40.0)
        });
        centered_on_screen(
            col()
                .id(T::Panel)
                .w_pct(1.0)
                .max_w(400.0)
                .h(300.0)
                .pad(16.0)
                .gap(8.0)
                .child(text("A title", 30, Fit::shrink(16)).id(T::Title).center_text().fixed())
                .child(col().gap(6.0).grow(1.0).min_h(0.0).children(rows))
                .maybe(page.paged().then(|| button(T::More, T::Title, "More", 22, Fit::shrink(14)).size(100.0, 36.0).fixed())),
            20.0,
        )
    }

    #[test]
    fn paging_keeps_every_row_reachable_without_clipping() {
        for &screen in &SWEEP_SCREENS {
            let bounds = screen_rect(screen);
            let mut seen = 0;
            let mut want = 0;
            loop {
                let (frame, page) = paged(12, want, bounds, panel);
                assert_sane(&frame, bounds);
                seen += page.len;
                if !page.has_next() {
                    break;
                }
                assert!(frame.rect(T::More).is_some(), "paged lists show a More control");
                want += 1;
            }
            assert_eq!(seen, 12, "every row lands on some page at {screen:?}");
        }
    }

    /// A clipped node without an id used to vanish without a trace; now
    /// every dropped text / region is reported, and the sweep fails on it.
    #[test]
    fn clipping_an_anonymous_node_is_reported() {
        let bounds = UiRect::new(0.0, 0.0, 200.0, 30.0);
        let root: Node<T> = col().children([
            text("I fit", 20, Fit::shrink(20)).fixed(),
            text("I don't", 20, Fit::shrink(20)).fixed(),
            region(50.0, 40.0).fixed(),
            col().child(text("nested", 20, Fit::shrink(20)).fixed()).fixed(),
        ]);
        let frame = layout(&root, bounds);
        let what: Vec<String> = frame.clipped().iter().map(|c| c.what()).collect();
        assert_eq!(what, ["text \"I don't\"", "region (node 3)", "text \"nested\""]);
        assert_eq!(frame.clipped()[2].node, 5);
        let issues = check_sane(&frame, bounds).unwrap_err();
        assert_eq!(issues.iter().filter(|i| i.contains("clipped out")).count(), 3, "{issues:?}");
    }

    #[test]
    fn hit_at_finds_the_deepest_target() {
        let bounds = screen_rect((960.0, 720.0));
        let (frame, _) = paged(3, 0, bounds, panel);
        let r = frame.rect(T::Row(1)).unwrap();
        let (x, y) = r.center();
        assert_eq!(frame.hit_at(x, y), Some(T::Row(1)));
        assert_eq!(frame.hit_at(1.0, 1.0), None);
    }
}
