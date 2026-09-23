//! The in-house engine: a single-line flexbox subset.
//!
//! Supports exactly the vocabulary in `node.rs` — direction, padding, gap,
//! Px/Auto sizes, min/max, grow/shrink, align-items/self, justify-content —
//! with CSS semantics wherever they're cheap to match:
//!
//! * main-axis sizes start at the basis (Px, else content size), then grow
//!   into free space by `flex_grow` or shrink by `flex_shrink × basis`, never
//!   below the item's minimum (explicit `min_*`, else its min-content — for
//!   text, what its [`Fit`](super::node::Fit) policy can shrink to);
//! * fixed (`flex_shrink: 0`) items keep their basis, so headers and footers
//!   are reserved before a growable body gets anything;
//! * cross size is Px, or stretched, or content — always capped to the
//!   container, so nothing pokes out sideways.
//!
//! Where CSS would let children overflow, this engine clips instead: a child
//! whose main-axis extent passes its container's end is dropped (its subtree
//! reports `None`). Overlap is impossible by construction; what didn't fit is
//! visible in `Frame::clipped` for the caller's paging policy and the sweep.

use super::engine::{LayoutEngine, Rects};
use super::metrics::TextMetrics;
use super::node::{Align, Content, Dim, Direction, Justify, Node, Style};
use super::rect::{UiRect, EPS};
use super::text;

#[derive(Debug, Default, Clone, Copy)]
pub struct FlowEngine;

impl LayoutEngine for FlowEngine {
    fn compute<Id>(&self, root: &Node<Id>, bounds: UiRect, metrics: &dyn TextMetrics) -> Rects {
        let mut out = vec![None; root.subtree_len()];
        let cx = Cx { m: metrics };
        let mut idx = 0;
        cx.place(root, bounds, &mut idx, &mut out);
        out
    }
}

struct Cx<'a> {
    m: &'a dyn TextMetrics,
}

fn clamp(v: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    let v = match max {
        Some(mx) => v.min(mx),
        None => v,
    };
    match min {
        Some(mn) => v.max(mn),
        None => v,
    }
}

fn kids<Id>(n: &Node<Id>) -> &[Node<Id>] {
    match &n.content {
        Content::Container(k) => k,
        _ => &[],
    }
}

fn gaps(style: &Style, n: usize) -> f32 {
    style.gap * n.saturating_sub(1) as f32
}

/// One flex item's main-axis inputs.
struct Item {
    basis: f32,
    min: f32,
    max: f32,
    grow: f32,
    shrink: f32,
}

/// Resolve main-axis sizes: grow into free space or shrink out of a deficit,
/// respecting each item's min/max (iteratively re-sharing what clamped items
/// couldn't take — the CSS "freeze" loop).
fn distribute(items: &[Item], avail: f32, gap_total: f32) -> Vec<f32> {
    let mut sizes: Vec<f32> = items.iter().map(|i| i.basis.clamp(i.min, i.max.max(i.min))).collect();
    let mut frozen = vec![false; items.len()];
    for _ in 0..=items.len() {
        let free = avail - gap_total - sizes.iter().sum::<f32>();
        if free > EPS {
            let total: f32 = items.iter().zip(&frozen).filter(|(i, f)| !**f && i.grow > 0.0).map(|(i, _)| i.grow).sum();
            if total <= 0.0 {
                break;
            }
            let mut any_clamped = false;
            for (k, it) in items.iter().enumerate() {
                if frozen[k] || it.grow <= 0.0 {
                    continue;
                }
                let want = sizes[k] + free * it.grow / total;
                if want >= it.max {
                    sizes[k] = it.max;
                    frozen[k] = true;
                    any_clamped = true;
                } else {
                    sizes[k] = want;
                }
            }
            if !any_clamped {
                break;
            }
        } else if free < -EPS {
            let total: f32 = items
                .iter()
                .enumerate()
                .filter(|(k, i)| !frozen[*k] && i.shrink > 0.0 && sizes[*k] > i.min + EPS)
                .map(|(_, i)| i.shrink * i.basis.max(1.0))
                .sum();
            if total <= 0.0 {
                break;
            }
            let mut any_clamped = false;
            for (k, it) in items.iter().enumerate() {
                if frozen[k] || it.shrink <= 0.0 || sizes[k] <= it.min + EPS {
                    continue;
                }
                let want = sizes[k] + free * it.shrink * it.basis.max(1.0) / total;
                if want <= it.min {
                    sizes[k] = it.min;
                    frozen[k] = true;
                    any_clamped = true;
                } else {
                    sizes[k] = want;
                }
            }
            if !any_clamped {
                break;
            }
        } else {
            break;
        }
    }
    sizes
}

impl Cx<'_> {
    // ─── Intrinsic sizes ────────────────────────────────

    /// Max-content width.
    fn max_w<Id>(&self, n: &Node<Id>) -> f32 {
        let s = &n.style;
        if let Dim::Px(w) = s.width {
            return clamp(w, s.min_width, s.max_width);
        }
        let inner = match &n.content {
            Content::Text(t) => text::natural_width(t, self.m),
            Content::Region => 0.0,
            Content::Container(k) => match s.direction {
                Direction::Row => k.iter().map(|c| self.max_w(c)).sum::<f32>() + gaps(s, k.len()),
                Direction::Column => k.iter().map(|c| self.max_w(c)).fold(0.0, f32::max),
            },
        };
        clamp(inner + s.padding.horizontal(), s.min_width, s.max_width)
    }

    /// Min-content width.
    fn min_w<Id>(&self, n: &Node<Id>) -> f32 {
        let s = &n.style;
        if let Some(mn) = s.min_width {
            return mn;
        }
        if let (Dim::Px(w), true) = (s.width, s.flex_shrink == 0.0) {
            return clamp(w, None, s.max_width);
        }
        let inner = match &n.content {
            Content::Text(t) => text::min_width(t, self.m),
            Content::Region => 0.0,
            Content::Container(k) => match s.direction {
                Direction::Row => k.iter().map(|c| self.min_w(c)).sum::<f32>() + gaps(s, k.len()),
                Direction::Column => k.iter().map(|c| self.min_w(c)).fold(0.0, f32::max),
            },
        };
        let content = inner + s.padding.horizontal();
        let v = match s.width {
            Dim::Px(w) => content.min(w),
            Dim::Auto => content,
        };
        clamp(v, None, s.max_width)
    }

    /// Width a column child gets (cross axis) inside `inner_w`.
    fn cross_w<Id>(&self, parent: &Style, c: &Node<Id>, inner_w: f32) -> f32 {
        let s = &c.style;
        let w = match (s.width, s.align_self.unwrap_or(parent.align_items)) {
            (Dim::Px(w), _) => clamp(w, s.min_width, s.max_width),
            (Dim::Auto, Align::Stretch) => clamp(inner_w, s.min_width, s.max_width),
            (Dim::Auto, _) => self.max_w(c),
        };
        w.min(inner_w).max(0.0)
    }

    /// Widths of a row's children inside `inner_w`.
    fn row_widths<Id>(&self, n: &Node<Id>, inner_w: f32) -> Vec<f32> {
        let k = kids(n);
        let items: Vec<Item> = k
            .iter()
            .map(|c| {
                let s = &c.style;
                Item {
                    basis: self.max_w(c),
                    min: self.min_w(c),
                    max: s.max_width.unwrap_or(f32::INFINITY),
                    grow: s.flex_grow,
                    shrink: s.flex_shrink,
                }
            })
            .collect();
        distribute(&items, inner_w, gaps(&n.style, k.len()))
    }

    /// Height at a given width (max-content height).
    fn height_for<Id>(&self, n: &Node<Id>, w: f32) -> f32 {
        let s = &n.style;
        if let Dim::Px(h) = s.height {
            return clamp(h, s.min_height, s.max_height);
        }
        let inner_w = (w - s.padding.horizontal()).max(0.0);
        let inner = match &n.content {
            Content::Text(t) => text::natural_height(t, inner_w, self.m),
            Content::Region => 0.0,
            Content::Container(k) => match s.direction {
                Direction::Column => {
                    k.iter().map(|c| self.height_for(c, self.cross_w(s, c, inner_w))).sum::<f32>()
                        + gaps(s, k.len())
                }
                Direction::Row => {
                    let ws = self.row_widths(n, inner_w);
                    k.iter().zip(ws).map(|(c, cw)| self.height_for(c, cw)).fold(0.0, f32::max)
                }
            },
        };
        clamp(inner + s.padding.vertical(), s.min_height, s.max_height)
    }

    /// Least height at a given width.
    fn min_h<Id>(&self, n: &Node<Id>, w: f32) -> f32 {
        let s = &n.style;
        if let Some(mn) = s.min_height {
            return mn;
        }
        if let (Dim::Px(h), true) = (s.height, s.flex_shrink == 0.0) {
            return clamp(h, None, s.max_height);
        }
        let inner_w = (w - s.padding.horizontal()).max(0.0);
        let inner = match &n.content {
            Content::Text(t) => text::min_height(t, inner_w, self.m),
            Content::Region => 0.0,
            Content::Container(k) => match s.direction {
                Direction::Column => {
                    k.iter().map(|c| self.min_h(c, self.cross_w(s, c, inner_w))).sum::<f32>() + gaps(s, k.len())
                }
                Direction::Row => {
                    let ws = self.row_widths(n, inner_w);
                    k.iter().zip(ws).map(|(c, cw)| self.min_h(c, cw)).fold(0.0, f32::max)
                }
            },
        };
        let content = inner + s.padding.vertical();
        let v = match s.height {
            Dim::Px(h) => content.min(h),
            Dim::Auto => content,
        };
        clamp(v, None, s.max_height)
    }

    // ─── Placement ──────────────────────────────────────

    fn place<Id>(&self, n: &Node<Id>, rect: UiRect, idx: &mut usize, out: &mut Rects) {
        out[*idx] = Some(rect);
        *idx += 1;
        let k = kids(n);
        if k.is_empty() {
            return;
        }
        let s = &n.style;
        let inner = rect.inset(s.padding.left, s.padding.top, s.padding.right, s.padding.bottom);
        let column = s.direction == Direction::Column;

        // Main-axis sizes, and each child's cross size.
        let (mains, crosses): (Vec<f32>, Vec<f32>) = if column {
            let cws: Vec<f32> = k.iter().map(|c| self.cross_w(s, c, inner.w)).collect();
            let items: Vec<Item> = k
                .iter()
                .zip(&cws)
                .map(|(c, &cw)| Item {
                    basis: self.height_for(c, cw),
                    min: self.min_h(c, cw),
                    max: c.style.max_height.unwrap_or(f32::INFINITY),
                    grow: c.style.flex_grow,
                    shrink: c.style.flex_shrink,
                })
                .collect();
            (distribute(&items, inner.h, gaps(s, k.len())), cws)
        } else {
            let ws = self.row_widths(n, inner.w);
            let hs = k
                .iter()
                .zip(&ws)
                .map(|(c, &cw)| {
                    let cs = &c.style;
                    let h = match (cs.height, cs.align_self.unwrap_or(s.align_items)) {
                        (Dim::Px(h), _) => clamp(h, cs.min_height, cs.max_height),
                        (Dim::Auto, Align::Stretch) => clamp(inner.h, cs.min_height, cs.max_height),
                        (Dim::Auto, _) => self.height_for(c, cw),
                    };
                    h.min(inner.h).max(0.0)
                })
                .collect();
            (ws, hs)
        };

        let main_avail = if column { inner.h } else { inner.w };
        let used = mains.iter().sum::<f32>() + gaps(s, k.len());
        let free = (main_avail - used).max(0.0);
        let (mut pos, extra_gap) = match s.justify_content {
            Justify::Start => (0.0, 0.0),
            Justify::Center => (free / 2.0, 0.0),
            Justify::End => (free, 0.0),
            Justify::SpaceBetween if k.len() > 1 => (0.0, free / (k.len() - 1) as f32),
            Justify::SpaceBetween => (0.0, 0.0),
        };

        for (i, c) in k.iter().enumerate() {
            let main = mains[i];
            let cross = crosses[i];
            let cross_avail = if column { inner.w } else { inner.h };
            let cross_off = match c.style.align_self.unwrap_or(s.align_items) {
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

            let end = if column { child.bottom() } else { child.right() };
            let limit = if column { inner.bottom() } else { inner.right() };
            if end > limit + EPS {
                // Doesn't fit: clip the whole subtree (already `None`).
                *idx += c.subtree_len();
                continue;
            }
            self.place(c, child, idx, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::metrics::FontMetrics;
    use crate::ui::layout::node::{col, region, row, spacer, text, Fit};

    fn run(root: &Node<()>, w: f32, h: f32) -> Rects {
        FlowEngine.compute(root, UiRect::new(0.0, 0.0, w, h), FontMetrics::bundled())
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
}
