//! The in-house engine: a single-line flexbox subset with CSS semantics.
//!
//! Supports exactly the vocabulary in `node.rs` — direction, padding, gap,
//! Px/Percent/Auto sizes, Px/Percent min/max, grow/shrink, align-items/self,
//! justify-content — and follows CSS flexbox (as implemented by taffy, which
//! a differential test holds it to):
//!
//! * main-axis sizes start at the flex basis (the preferred size, else the
//!   content size), then grow into free space by `flex_grow` or shrink by
//!   `flex_shrink × inner basis`, never below the item's minimum (explicit
//!   `min_*`, else its automatic minimum: min-content — for text, what its
//!   [`Fit`](super::node::Fit) policy can shrink to — capped by its preferred
//!   size) — the CSS "freeze" loop, ported from taffy step for step;
//! * a container's min-content counts its children's *preferred* sizes, so
//!   a nested column only gives up height it doesn't need if it says
//!   `min_h(0.0)` (CSS's "min-height: 0" rule);
//! * fixed (`flex_shrink: 0`) items keep their basis, so headers and footers
//!   are reserved before a growable body gets anything;
//! * cross size is the preferred size, or stretched to the line, or
//!   fit-content (`min(max-content, max(min-content, room))`), clamped by
//!   min/max — and **not capped to the container**: a 760px panel in a 320px
//!   container is 760px wide, as in a browser. Say `.w_pct(1.0).max_w(760.0)`
//!   to mean "760, or all the room there is";
//! * percentages resolve against the parent's content box once it's known,
//!   and behave as `auto` while measuring content;
//! * nothing is dropped here: what doesn't fit overflows (Center/End
//!   justification and alignment overflow the start side too), then the shared
//!   [`clip`](super::engine::clip) post-pass turns overflow into `None`;
//! * positions are exact (unrounded); the shared pipeline rounds edges.

use super::engine::{LayoutEngine, LayoutTree, LeafKind};
use super::metrics::TextMetrics;
use super::node::{Align, Dim, Direction, Justify, Len, Style};
use super::rect::UiRect;
use super::text;

#[derive(Debug, Default, Clone, Copy)]
pub struct FlowEngine;

impl LayoutEngine for FlowEngine {
    fn compute(&self, tree: &LayoutTree, bounds: UiRect, metrics: &dyn TextMetrics) -> Vec<UiRect> {
        let mut out = vec![UiRect::default(); tree.nodes.len()];
        if !tree.nodes.is_empty() {
            Cx { m: metrics, t: tree }.place(0, bounds, &mut out);
        }
        out
    }
}

struct Cx<'a> {
    m: &'a dyn TextMetrics,
    t: &'a LayoutTree<'a>,
}

fn clamp(v: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    // CSS: max first, then min wins.
    let v = match max {
        Some(mx) => v.min(mx),
        None => v,
    };
    match min {
        Some(mn) => v.max(mn),
        None => v,
    }
}

fn gaps(style: &Style, n: usize) -> f32 {
    style.gap * n.saturating_sub(1) as f32
}

fn dim(d: Dim, basis: Option<f32>) -> Option<f32> {
    match d {
        Dim::Auto => None,
        Dim::Px(v) => Some(v),
        Dim::Percent(f) => basis.map(|b| b * f),
    }
}

fn len(l: Option<Len>, basis: Option<f32>) -> Option<f32> {
    match l? {
        Len::Px(v) => Some(v),
        Len::Percent(f) => basis.map(|b| b * f),
    }
}

/// A node's size constraints in one axis, resolved against its parent's
/// content size (`None` while measuring content: percentages act as auto).
#[derive(Debug, Clone, Copy)]
struct Sz {
    pref: Option<f32>,
    min: Option<f32>,
    max: Option<f32>,
}

impl Sz {
    fn clamp(&self, v: f32) -> f32 {
        clamp(v, self.min, self.max)
    }
    /// The CSS automatic minimum of a flex item in this axis, given its
    /// min-content size: explicit min; else min-content capped by the
    /// preferred size (a fixed item's minimum is its preferred size).
    fn item_min(&self, min_content: impl FnOnce() -> f32, shrink: f32) -> f32 {
        if let Some(mn) = self.min {
            return mn;
        }
        let v = match self.pref {
            Some(p) if shrink == 0.0 => p,
            Some(p) => min_content().min(p),
            None => min_content(),
        };
        clamp(v, None, self.max)
    }
}

/// One flex item's main-axis inputs.
struct Item {
    /// Flex base size (border box: padding included).
    basis: f32,
    /// Padding along the main axis: CSS scales shrinking by the *inner*
    /// (content-box) basis.
    pad: f32,
    min: f32,
    max: f32,
    grow: f32,
    shrink: f32,
}

/// Resolve main-axis sizes — CSS flexbox §9.7 "Resolving Flexible Lengths",
/// following taffy's implementation step for step (including its quirks) so
/// the two engines agree to the pixel. Sizes may sum past `avail` when
/// minimums don't allow otherwise: that's overflow.
fn distribute(items: &[Item], avail: f32, gap_total: f32) -> Vec<f32> {
    // Flex base sizes are floored at the item's padding.
    let basis: Vec<f32> = items.iter().map(|i| i.basis.max(i.pad)).collect();
    let hypo: Vec<f32> = items.iter().zip(&basis).map(|(i, &b)| clamp(b, Some(i.min.max(i.pad)), Some(i.max))).collect();
    let used = gap_total + hypo.iter().sum::<f32>();
    let (growing, shrinking) = (used < avail, used > avail);

    // Size inflexible items (and every item when nothing flexes).
    let mut target = hypo.clone();
    let mut frozen: Vec<bool> = items
        .iter()
        .enumerate()
        .map(|(k, i)| {
            !(growing || shrinking)
                || (i.grow == 0.0 && i.shrink == 0.0)
                || (growing && basis[k] > hypo[k])
                || (shrinking && basis[k] < hypo[k])
        })
        .collect();
    let used_space = |target: &[f32], frozen: &[bool]| {
        gap_total + (0..items.len()).map(|k| if frozen[k] { target[k] } else { basis[k] }).sum::<f32>()
    };
    let initial_free = avail - used_space(&target, &frozen);

    while frozen.iter().any(|f| !f) {
        let used = used_space(&target, &frozen);
        let unfrozen = || (0..items.len()).filter(|&k| !frozen[k]);
        let sum_grow: f32 = unfrozen().map(|k| items[k].grow).sum();
        let sum_shrink: f32 = unfrozen().map(|k| items[k].shrink).sum();
        let free = if growing && sum_grow < 1.0 {
            (initial_free * sum_grow - gap_total).min(avail - used)
        } else if shrinking && sum_shrink < 1.0 {
            (initial_free * sum_shrink - gap_total).max(avail - used)
        } else {
            avail - used
        };

        if free.is_normal() {
            if growing && sum_grow > 0.0 {
                for k in unfrozen() {
                    target[k] = basis[k] + free * (items[k].grow / sum_grow);
                }
            } else if shrinking && sum_shrink > 0.0 {
                let scaled = |k: usize| (basis[k] - items[k].pad) * items[k].shrink;
                let sum_scaled: f32 = unfrozen().map(scaled).sum();
                if sum_scaled > 0.0 {
                    for k in unfrozen() {
                        target[k] = basis[k] + free * (scaled(k) / sum_scaled);
                    }
                }
            }
        }

        // Clamp to min/max and freeze the violators.
        let mut total_violation = 0.0;
        let mut violation = vec![0.0; items.len()];
        for k in unfrozen() {
            let clamped = clamp(target[k], Some(items[k].min), Some(items[k].max)).max(0.0);
            violation[k] = clamped - target[k];
            target[k] = clamped;
            total_violation += violation[k];
        }
        for k in (0..items.len()).filter(|&k| !frozen[k]).collect::<Vec<_>>() {
            frozen[k] = if total_violation > 0.0 {
                violation[k] > 0.0
            } else if total_violation < 0.0 {
                violation[k] < 0.0
            } else {
                true
            };
        }
    }
    target
}

impl Cx<'_> {
    fn style(&self, n: usize) -> &Style {
        self.t.nodes[n].style
    }
    fn sw(&self, n: usize, basis: Option<f32>) -> Sz {
        let s = self.style(n);
        Sz { pref: dim(s.width, basis), min: len(s.min_width, basis), max: len(s.max_width, basis) }
    }
    fn sh(&self, n: usize, basis: Option<f32>) -> Sz {
        let s = self.style(n);
        Sz { pref: dim(s.height, basis), min: len(s.min_height, basis), max: len(s.max_height, basis) }
    }

    // ─── Content sizes (the box's own content + padding) ─────

    /// Max-content width of `n`'s content.
    fn max_content_w(&self, n: usize) -> f32 {
        let s = self.style(n);
        let k = &self.t.nodes[n].children;
        let inner = match self.t.nodes[n].kind {
            LeafKind::Text(t) => text::natural_width(t, self.m),
            LeafKind::Region => 0.0,
            LeafKind::Container => match s.direction {
                Direction::Row => k.iter().map(|&c| self.max_w(c)).sum::<f32>() + gaps(s, k.len()),
                Direction::Column => k.iter().map(|&c| self.max_w(c)).fold(0.0, f32::max),
            },
        };
        inner + s.padding.horizontal()
    }

    /// Min-content width of `n`'s content.
    fn min_content_w(&self, n: usize) -> f32 {
        let s = self.style(n);
        let k = &self.t.nodes[n].children;
        let inner = match self.t.nodes[n].kind {
            LeafKind::Text(t) => text::min_width(t, self.m),
            LeafKind::Region => 0.0,
            LeafKind::Container => match s.direction {
                Direction::Row => k.iter().map(|&c| self.min_w(c)).sum::<f32>() + gaps(s, k.len()),
                Direction::Column => k.iter().map(|&c| self.min_w(c)).fold(0.0, f32::max),
            },
        };
        inner + s.padding.horizontal()
    }

    /// Max-content height of `n`'s content at border-box width `w`.
    fn content_h(&self, n: usize, w: f32) -> f32 {
        let s = self.style(n);
        let inner_w = (w - s.padding.horizontal()).max(0.0);
        let k = &self.t.nodes[n].children;
        let inner = match self.t.nodes[n].kind {
            LeafKind::Text(t) => text::natural_height(t, inner_w, self.m),
            LeafKind::Region => 0.0,
            LeafKind::Container => match s.direction {
                Direction::Column => {
                    k.iter().map(|&c| self.height_for(c, self.cross_w(s, c, inner_w))).sum::<f32>()
                        + gaps(s, k.len())
                }
                Direction::Row => {
                    let ws = self.row_widths(n, inner_w);
                    k.iter().zip(ws).map(|(&c, cw)| self.height_for(c, cw)).fold(0.0, f32::max)
                }
            },
        };
        inner + s.padding.vertical()
    }

    /// Min-content height of `n`'s content at border-box width `w`.
    fn min_content_h(&self, n: usize, w: f32) -> f32 {
        let s = self.style(n);
        let inner_w = (w - s.padding.horizontal()).max(0.0);
        let k = &self.t.nodes[n].children;
        let inner = match self.t.nodes[n].kind {
            LeafKind::Text(t) => text::min_height(t, inner_w, self.m),
            LeafKind::Region => 0.0,
            LeafKind::Container => match s.direction {
                Direction::Column => {
                    k.iter().map(|&c| self.min_h(c, self.cross_w(s, c, inner_w))).sum::<f32>() + gaps(s, k.len())
                }
                Direction::Row => {
                    let ws = self.row_widths(n, inner_w);
                    k.iter().zip(ws).map(|(&c, cw)| self.min_h(c, cw)).fold(0.0, f32::max)
                }
            },
        };
        inner + s.padding.vertical()
    }

    // ─── Contributions (content sizes seen through the node's own style) ───

    /// Max-content width contribution.
    fn max_w(&self, n: usize) -> f32 {
        let z = self.sw(n, None);
        z.clamp(z.pref.unwrap_or_else(|| self.max_content_w(n)))
    }

    /// Min-content width contribution: the preferred width if there is one,
    /// else min-content, clamped by min/max. NOT the item's automatic
    /// minimum: in CSS a child's `min-width` only stops *that child* from
    /// shrinking; what its parent can shrink to counts its preferred size.
    fn min_w(&self, n: usize) -> f32 {
        let z = self.sw(n, None);
        z.clamp(z.pref.unwrap_or_else(|| self.min_content_w(n)))
    }

    /// Max-content height contribution at width `w`.
    fn height_for(&self, n: usize, w: f32) -> f32 {
        let z = self.sh(n, None);
        z.clamp(z.pref.unwrap_or_else(|| self.content_h(n, w)))
    }

    /// Min-content height contribution at width `w` (see [`Self::min_w`]):
    /// so a container can only shrink below its children's preferred heights
    /// if it says `min_h(0.0)` — the CSS "min-height: 0" rule.
    fn min_h(&self, n: usize, w: f32) -> f32 {
        let z = self.sh(n, None);
        z.clamp(z.pref.unwrap_or_else(|| self.min_content_h(n, w)))
    }

    // ─── Sizing children against a known container ─────────

    /// Width a column child gets (its cross axis) inside `inner_w`.
    fn cross_w(&self, parent: &Style, c: usize, inner_w: f32) -> f32 {
        let z = self.sw(c, Some(inner_w));
        let w = match (z.pref, self.style(c).align_self.unwrap_or(parent.align_items)) {
            (Some(w), _) => w,
            (None, Align::Stretch) => inner_w,
            // fit-content
            (None, _) => self.max_content_w(c).min(inner_w).max(self.min_content_w(c)),
        };
        z.clamp(w).max(0.0)
    }

    /// Height a row child gets (its cross axis) at width `w` in `inner_h`.
    fn cross_h(&self, parent: &Style, c: usize, w: f32, inner_h: f32) -> f32 {
        let z = self.sh(c, Some(inner_h));
        let h = match (z.pref, self.style(c).align_self.unwrap_or(parent.align_items)) {
            (Some(h), _) => h,
            (None, Align::Stretch) => inner_h,
            (None, _) => self.content_h(c, w),
        };
        z.clamp(h).max(0.0)
    }

    /// Widths of a row's children inside `inner_w`.
    fn row_widths(&self, n: usize, inner_w: f32) -> Vec<f32> {
        let k = &self.t.nodes[n].children;
        let items: Vec<Item> = k
            .iter()
            .map(|&c| {
                let z = self.sw(c, Some(inner_w));
                let s = self.style(c);
                Item {
                    basis: z.pref.unwrap_or_else(|| self.max_content_w(c)),
                    pad: s.padding.horizontal(),
                    min: z.item_min(|| self.min_content_w(c), s.flex_shrink),
                    max: z.max.unwrap_or(f32::INFINITY),
                    grow: s.flex_grow,
                    shrink: s.flex_shrink,
                }
            })
            .collect();
        distribute(&items, inner_w, gaps(self.style(n), k.len()))
    }

    // ─── Placement ──────────────────────────────────────

    fn place(&self, n: usize, rect: UiRect, out: &mut [UiRect]) {
        out[n] = rect;
        let k = &self.t.nodes[n].children;
        if k.is_empty() {
            return;
        }
        let s = self.style(n);
        let inner = rect.inset(s.padding.left, s.padding.top, s.padding.right, s.padding.bottom);
        let column = s.direction == Direction::Column;

        // Main-axis sizes, and each child's cross size.
        let (mains, crosses): (Vec<f32>, Vec<f32>) = if column {
            let cws: Vec<f32> = k.iter().map(|&c| self.cross_w(s, c, inner.w)).collect();
            let items: Vec<Item> = k
                .iter()
                .zip(&cws)
                .map(|(&c, &cw)| {
                    let z = self.sh(c, Some(inner.h));
                    let cs = self.style(c);
                    Item {
                        basis: z.pref.unwrap_or_else(|| self.content_h(c, cw)),
                        pad: cs.padding.vertical(),
                        min: z.item_min(|| self.min_content_h(c, cw), cs.flex_shrink),
                        max: z.max.unwrap_or(f32::INFINITY),
                        grow: cs.flex_grow,
                        shrink: cs.flex_shrink,
                    }
                })
                .collect();
            (distribute(&items, inner.h, gaps(s, k.len())), cws)
        } else {
            let ws = self.row_widths(n, inner.w);
            let hs = k.iter().zip(&ws).map(|(&c, &cw)| self.cross_h(s, c, cw, inner.h)).collect();
            (ws, hs)
        };

        // Leftover space; negative when the items overflow.
        let main_avail = if column { inner.h } else { inner.w };
        let free = main_avail - mains.iter().sum::<f32>() - gaps(s, k.len());
        let (mut pos, extra_gap) = match s.justify_content {
            Justify::Start => (0.0, 0.0),
            Justify::Center => (free / 2.0, 0.0),
            Justify::End => (free, 0.0),
            // CSS: space-between with no room (or one item) behaves as start.
            Justify::SpaceBetween if k.len() > 1 && free > 0.0 => (0.0, free / (k.len() - 1) as f32),
            Justify::SpaceBetween => (0.0, 0.0),
        };

        let cross_avail = if column { inner.w } else { inner.h };
        for (i, &c) in k.iter().enumerate() {
            let (main, cross) = (mains[i], crosses[i]);
            let cross_off = match self.style(c).align_self.unwrap_or(s.align_items) {
                Align::Start | Align::Stretch => 0.0,
                Align::Center => (cross_avail - cross) / 2.0,
                Align::End => cross_avail - cross,
            };
            let child = if column {
                UiRect::new(inner.x + cross_off, inner.y + pos, cross, main)
            } else {
                UiRect::new(inner.x + pos, inner.y + cross_off, main, cross)
            };
            pos += main + s.gap + extra_gap;
            self.place(c, child, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::engine::{clip, round_edges, Rects};
    use crate::ui::layout::metrics::FontMetrics;
    use crate::ui::layout::node::{col, region, row, spacer, text, Fit, Node};

    fn raw(root: &Node<()>, w: f32, h: f32) -> Vec<UiRect> {
        FlowEngine.compute(&LayoutTree::new(root), UiRect::new(0.0, 0.0, w, h), FontMetrics::bundled())
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

    // ─── CSS semantics (pinned; matched taffy to 0.01px when last diffed) ───

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
