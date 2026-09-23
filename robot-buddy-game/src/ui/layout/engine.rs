//! The engine seam: node tree in, one rect per node out.
//!
//! An engine only answers "where does every box go". It knows nothing about
//! ids, hit targets, painting, or how a text leaf's lines are broken — that is
//! [`Frame::resolve`](super::frame::Frame::resolve)'s job, shared by all
//! engines. Swapping the in-house [`FlowEngine`](super::flow::FlowEngine) for
//! a taffy-backed one is therefore one new impl of this trait plus flipping
//! the `DefaultEngine` alias in `layout/mod.rs`.

use super::metrics::TextMetrics;
use super::node::{Content, Node, Style, TextSpec};
use super::rect::UiRect;

/// Rects for every node of a tree, in pre-order (index 0 is the root). `None`
/// means the node didn't fit in its parent and was clipped out, together with
/// its whole subtree.
pub type Rects = Vec<Option<UiRect>>;

/// What a node is, as far as an engine cares.
#[derive(Debug, Clone, Copy)]
pub enum LeafKind<'a> {
    Container,
    Text(&'a TextSpec),
    Region,
}

/// One node of a [`LayoutTree`].
#[derive(Debug, Clone)]
pub struct TreeNode<'a> {
    pub style: &'a Style,
    pub kind: LeafKind<'a>,
    /// Indices of the children, in order.
    pub children: Vec<usize>,
}

/// A panel's node tree flattened into a pre-order arena, with the panel's ids
/// stripped. Engines work on this, so they aren't generic over each panel's id
/// type (one copy of the engine in the binary, and an object-safe trait). It
/// is also exactly the shape `taffy::TaffyTree::new_with_children` wants.
#[derive(Debug, Clone)]
pub struct LayoutTree<'a> {
    pub nodes: Vec<TreeNode<'a>>,
}

impl<'a> LayoutTree<'a> {
    pub fn new<Id>(root: &'a Node<Id>) -> Self {
        fn push<'a, Id>(n: &'a Node<Id>, out: &mut Vec<TreeNode<'a>>) -> usize {
            let me = out.len();
            let kind = match &n.content {
                Content::Container(_) => LeafKind::Container,
                Content::Text(t) => LeafKind::Text(t),
                Content::Region => LeafKind::Region,
            };
            out.push(TreeNode { style: &n.style, kind, children: Vec::new() });
            if let Content::Container(kids) = &n.content {
                let children = kids.iter().map(|k| push(k, out)).collect();
                out[me].children = children;
            }
            me
        }
        let mut nodes = Vec::with_capacity(root.subtree_len());
        push(root, &mut nodes);
        LayoutTree { nodes }
    }
}

pub trait LayoutEngine {
    /// Lay the tree out with the root filling `bounds` (node 0 is the root;
    /// its own size style is ignored). Returns one absolute, **unrounded**
    /// rect per node, in pre-order. The contract every engine keeps (a
    /// differential test held `FlowEngine` to taffy within 0.01px before it
    /// was retired; see ADR-004):
    ///
    /// * **CSS flexbox semantics** for the vocabulary in `node.rs`, including
    ///   overflow: a child that doesn't fit is *not* capped or dropped here;
    ///   it pokes out of its parent exactly as it would in a browser (and
    ///   `justify`/`align` Center/End on overflow push it out of the *start*
    ///   side too).
    ///
    /// Text leaves are measured with [`super::text`] against `metrics`. What
    /// every engine shares happens after: [`round_edges`] then [`clip`].
    fn compute(&self, tree: &LayoutTree, bounds: UiRect, metrics: &dyn TextMetrics) -> Vec<UiRect>;
}

/// Round absolute rects to whole pixels: each edge goes to the nearest pixel
/// independently, so abutting boxes stay abutting and a box that fit its
/// parent still does (rounding is monotonic). Shared by every engine — taffy
/// runs with its own rounding off, because it rounds *relative* offsets, which
/// can push a snugly fitting child a pixel out of its parent.
pub fn round_edges(rects: &mut [UiRect]) {
    for r in rects {
        let (x0, y0) = (r.x.round(), r.y.round());
        let (x1, y1) = ((r.x + r.w).round(), (r.y + r.h).round());
        *r = UiRect::new(x0, y0, x1 - x0, y1 - y0);
    }
}

/// The shared clip post-pass: turn CSS overflow into "didn't fit". A node
/// whose box leaves its parent's content box (its rect minus padding) by more
/// than [`EPS`](super::rect::EPS), in any direction, is `None` together with
/// its whole subtree. `Frame::clipped` reports them; paging and the sanity
/// sweep read that.
pub fn clip(tree: &LayoutTree, raw: &[UiRect]) -> Rects {
    fn walk(tree: &LayoutTree, raw: &[UiRect], n: usize, out: &mut Rects) {
        out[n] = Some(raw[n]);
        let p = tree.nodes[n].style.padding;
        let content = raw[n].inset(p.left, p.top, p.right, p.bottom);
        for &c in &tree.nodes[n].children {
            if content.contains_rect(&raw[c]) {
                walk(tree, raw, c, out);
            }
        }
    }
    let mut out = vec![None; raw.len()];
    if !raw.is_empty() {
        walk(tree, raw, 0, &mut out);
    }
    out
}
