//! The layout engine: taffy, the flexbox implementation inside Bevy UI.
//!
//! Panels build a [`Node`](super::node::Node) tree; this maps each node's
//! style 1:1 onto a `taffy::Style` (the table in `layout/mod.rs`), measures
//! text leaves with [`super::text`] (so a text's [`Fit`](super::node::Fit)
//! policy decides its natural and minimum sizes), and returns absolute,
//! unrounded rects. The shared pipeline in [`super::engine`] rounds and clips.

use taffy::prelude::*;

use super::engine::{LayoutEngine, LayoutTree, LeafKind};
use super::metrics::TextMetrics;
use super::node::{self, Align, Dim, Direction, Justify, Len};
use super::rect::UiRect;
use super::text;

#[derive(Debug, Default, Clone, Copy)]
pub struct TaffyEngine;

fn dim(d: Dim) -> Dimension {
    match d {
        Dim::Auto => Dimension::auto(),
        Dim::Px(v) => Dimension::length(v),
        Dim::Percent(f) => Dimension::percent(f),
    }
}

fn len(l: Option<Len>) -> Dimension {
    match l {
        None => Dimension::auto(),
        Some(Len::Px(v)) => Dimension::length(v),
        Some(Len::Percent(f)) => Dimension::percent(f),
    }
}

fn align(a: Align) -> AlignItems {
    match a {
        Align::Start => AlignItems::Start,
        Align::Center => AlignItems::Center,
        Align::End => AlignItems::End,
        Align::Stretch => AlignItems::Stretch,
    }
}

/// `node::Style` → `taffy::Style`, field for field.
fn style(s: &node::Style) -> Style {
    let lp = |v: f32| LengthPercentage::length(v);
    Style {
        display: Display::Flex,
        flex_direction: match s.direction {
            Direction::Row => FlexDirection::Row,
            Direction::Column => FlexDirection::Column,
        },
        padding: Rect { left: lp(s.padding.left), right: lp(s.padding.right), top: lp(s.padding.top), bottom: lp(s.padding.bottom) },
        gap: Size { width: lp(s.gap), height: lp(s.gap) },
        size: Size { width: dim(s.width), height: dim(s.height) },
        min_size: Size { width: len(s.min_width), height: len(s.min_height) },
        max_size: Size { width: len(s.max_width), height: len(s.max_height) },
        flex_grow: s.flex_grow,
        flex_shrink: s.flex_shrink,
        align_items: Some(align(s.align_items)),
        align_self: s.align_self.map(align),
        justify_content: Some(match s.justify_content {
            Justify::Start => JustifyContent::Start,
            Justify::Center => JustifyContent::Center,
            Justify::End => JustifyContent::End,
            Justify::SpaceBetween => JustifyContent::SpaceBetween,
        }),
        ..Default::default()
    }
}

impl LayoutEngine for TaffyEngine {
    fn compute(&self, tree: &LayoutTree, bounds: UiRect, m: &dyn TextMetrics) -> Vec<UiRect> {
        let n = tree.nodes.len();
        if n == 0 {
            return Vec::new();
        }
        // 1. Build bottom-up; each leaf carries its arena index as context.
        let mut t: TaffyTree<usize> = TaffyTree::new();
        // The pipeline rounds absolute edges (engine::round_edges); taffy's
        // own rounding works on relative offsets and can push a snug child a
        // pixel out of its parent.
        t.disable_rounding();
        let mut ids = vec![NodeId::from(0u64); n];
        for i in (0..n).rev() {
            let node = &tree.nodes[i];
            let mut st = style(node.style);
            if i == 0 {
                // The root fills the bounds (the engine contract).
                st.size = Size { width: Dimension::length(bounds.w), height: Dimension::length(bounds.h) };
                st.min_size = Size::auto();
                st.max_size = Size::auto();
            }
            let kids: Vec<NodeId> = node.children.iter().map(|&c| ids[c]).collect();
            ids[i] = if kids.is_empty() {
                t.new_leaf_with_context(st, i).expect("taffy leaf")
            } else {
                t.new_with_children(st, &kids).expect("taffy node")
            };
        }
        // 2. Lay out, measuring text leaves with the shared text module.
        let avail = Size { width: AvailableSpace::Definite(bounds.w), height: AvailableSpace::Definite(bounds.h) };
        t.compute_layout_with_measure(ids[0], avail, |known, space, _id, ctx, _style| {
            let Some(&mut i) = ctx else { return Size::ZERO };
            let LeafKind::Text(spec) = tree.nodes[i].kind else { return Size::ZERO };
            let width = known.width.unwrap_or_else(|| match space.width {
                AvailableSpace::MinContent => text::min_width(spec, m),
                AvailableSpace::MaxContent => text::natural_width(spec, m),
                AvailableSpace::Definite(w) => text::natural_width(spec, m).min(w).max(text::min_width(spec, m)),
            });
            let height = known.height.unwrap_or_else(|| match space.height {
                AvailableSpace::MinContent => text::min_height(spec, width, m),
                _ => text::natural_height(spec, width, m),
            });
            Size { width, height }
        })
        .expect("taffy layout");
        // 3. Absolute rects: sum the relative locations.
        let mut out = vec![UiRect::default(); n];
        fn walk(t: &TaffyTree<usize>, tree: &LayoutTree, ids: &[NodeId], i: usize, x: f32, y: f32, out: &mut [UiRect]) {
            let l = t.layout(ids[i]).expect("laid out");
            let (ax, ay) = (x + l.location.x, y + l.location.y);
            out[i] = UiRect::new(ax, ay, l.size.width, l.size.height);
            for &c in &tree.nodes[i].children {
                walk(t, tree, ids, c, ax, ay, out);
            }
        }
        walk(&t, tree, &ids, 0, bounds.x, bounds.y, &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::engine::{clip, round_edges, Rects};
    use crate::ui::layout::metrics::FontMetrics;
    use crate::ui::layout::node::{col, region, row, spacer, text, Fit, Node};
    use crate::ui::layout::rect::UiRect;

    fn raw(root: &Node<()>, w: f32, h: f32) -> Vec<UiRect> {
        TaffyEngine.compute(&LayoutTree::new(root), UiRect::new(0.0, 0.0, w, h), FontMetrics::bundled())
    }

    /// The whole shared pipeline: engine, rounding, clipping.
    fn run(root: &Node<()>, w: f32, h: f32) -> Rects {
        let mut r = raw(root, w, h);
        round_edges(&mut r);
        clip(&LayoutTree::new(root), &r)
    }

    #[test]
    fn column_stacks_with_gap_and_padding() {
        let root = col().pad(10.0).gap(5.0).children([region(50.0, 20.0), region(50.0, 30.0)]);
        let r = run(&root, 200.0, 200.0);
        assert_eq!(r[1], Some(UiRect::new(10.0, 10.0, 50.0, 20.0)));
        assert_eq!(r[2], Some(UiRect::new(10.0, 35.0, 50.0, 30.0)));
    }

    #[test]
    fn fixed_footer_is_reserved_before_a_growable_body() {
        let root = col().children([
            region(10.0, 300.0).min_h(0.0).grow(1.0), // body: wants 300, may shrink
            region(10.0, 40.0).fixed(),                // footer
        ]);
        let r = run(&root, 100.0, 200.0);
        assert_eq!(r[1].unwrap().h, 160.0);
        assert_eq!(r[2].unwrap().y, 160.0);
    }

    #[test]
    fn spacer_pushes_to_the_end() {
        let root = row().children([region(20.0, 10.0), spacer(), region(30.0, 10.0)]);
        let r = run(&root, 200.0, 10.0);
        assert_eq!(r[3].unwrap().x, 170.0);
    }

    #[test]
    fn overflowing_children_are_clipped_not_overlapped() {
        let root = col().children((0..5).map(|_| region(10.0, 50.0).fixed()));
        let r = run(&root, 100.0, 120.0);
        assert!(r[1].is_some() && r[2].is_some());
        assert!(r[3].is_none() && r[4].is_none() && r[5].is_none());
    }

    #[test]
    fn text_shrinks_before_anything_clips() {
        let root = row().children([
            text("A pretty long item name", 24, Fit::shrink(12)).grow(1.0),
            text("99 P", 24, Fit::shrink(12)),
        ]);
        let r = run(&root, 250.0, 40.0);
        let (name, price) = (r[1].unwrap(), r[2].unwrap());
        assert!(!name.overlaps(&price));
        assert!(price.right() <= 250.5);
    }

    // ─── CSS semantics ───

    /// CSS doesn't cap a child's cross size to its container: a 300px-wide
    /// box in a 200px column is 300px wide and overflows (then clips).
    #[test]
    fn a_fixed_cross_size_overflows_instead_of_being_capped() {
        let root = col().align(Align::Center).child(region(300.0, 20.0));
        let r = raw(&root, 200.0, 100.0);
        assert_eq!(r[1], UiRect::new(-50.0, 0.0, 300.0, 20.0), "centered overflow spills both sides");
        assert_eq!(run(&root, 200.0, 100.0)[1], None);
        // Same in a row's cross axis.
        let root = row().child(region(20.0, 150.0));
        assert_eq!(raw(&root, 100.0, 100.0)[1], UiRect::new(0.0, -25.0, 20.0, 150.0));
    }

    /// `.w_pct(1.0).max_w(..)` is how a panel fits a narrow screen.
    #[test]
    fn percent_width_with_a_max_fits_the_container() {
        let root = col().pad(20.0).align(Align::Center).child(region(0.0, 20.0).w_pct(1.0).max_w(760.0));
        assert_eq!(raw(&root, 360.0, 100.0)[1], UiRect::new(20.0, 20.0, 320.0, 20.0));
        assert_eq!(raw(&root, 1600.0, 100.0)[1], UiRect::new(420.0, 20.0, 760.0, 20.0));
        // Half of a row's width.
        let root = row().child(region(0.0, 10.0).w_pct(0.5));
        assert_eq!(raw(&root, 300.0, 10.0)[1].w, 150.0);
        // A percent min-height against a known container.
        let root = col().child(region(10.0, 0.0).auto_h().min_h_pct(0.25));
        assert_eq!(raw(&root, 100.0, 400.0)[1].h, 100.0);
    }

    /// CSS justify-content on overflow: `end` pushes the FIRST items out of
    /// the start side, `center` spills both ways, `space-between` acts as
    /// `start`.
    #[test]
    fn justify_on_overflow_spills_like_css() {
        let kids = || (0..3).map(|_| region(10.0, 50.0).fixed());
        let end = raw(&col().justify(Justify::End).children(kids()), 10.0, 120.0);
        assert_eq!(end[1].y, -30.0);
        assert_eq!(end[3].bottom(), 120.0);
        let center = raw(&col().justify(Justify::Center).children(kids()), 10.0, 120.0);
        assert_eq!(center[1].y, -15.0);
        let between = raw(&col().justify(Justify::SpaceBetween).children(kids()), 10.0, 120.0);
        assert_eq!(between[1].y, 0.0);
        assert_eq!(between[3].y, 100.0);
        // After clipping, End keeps the LAST items and drops the first.
        let clipped = run(&col().justify(Justify::End).children(kids()), 10.0, 120.0);
        assert_eq!(clipped[1], None);
        assert!(clipped[2].is_some() && clipped[3].is_some());
    }

    /// The engine is exact; the pipeline rounds each edge to the nearest
    /// pixel, so abutting boxes stay abutting and nothing that fit overflows.
    #[test]
    fn edges_round_to_whole_pixels() {
        let root = row().children((0..3).map(|_| region(0.0, 10.0).auto_w().grow(1.0)));
        assert!((raw(&root, 100.0, 10.0)[2].x - 100.0 / 3.0).abs() < 1e-3, "the engine doesn't round");
        let r: Vec<UiRect> = run(&root, 100.0, 10.0).into_iter().map(Option::unwrap).collect();
        assert_eq!((r[1].x, r[1].w), (0.0, 33.0));
        assert_eq!((r[2].x, r[2].w), (33.0, 34.0)); // 33.33..66.67 → 33..67
        assert_eq!((r[3].x, r[3].w), (67.0, 33.0));
    }

    /// An auto-width, non-stretched column child is fit-content: as wide as
    /// its content, but no wider than the room unless its min-content is.
    #[test]
    fn auto_cross_size_is_fit_content() {
        let label = || text("aaaa bbbb cccc dddd", 16, Fit::wrap(16)); // 8px/char: 152 max, 32 min
        let root = col().align(Align::Start).child(col().child(label()));
        assert_eq!(raw(&root, 300.0, 100.0)[1].w, 152.0);
        assert_eq!(raw(&root, 100.0, 100.0)[1].w, 100.0);
        assert_eq!(raw(&root, 20.0, 100.0)[1].w, 32.0, "min-content overflows");
    }
}
