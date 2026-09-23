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
    /// Lay the tree out inside `bounds` (node 0 is the root). Contract every
    /// engine must keep:
    /// * every placed child lies inside its parent's padding box, and
    /// * siblings never overlap.
    ///
    /// Anything that can't satisfy both is returned as `None` (clipped).
    /// Text leaves are measured with [`super::text`] against `metrics`.
    fn compute(&self, tree: &LayoutTree, bounds: UiRect, metrics: &dyn TextMetrics) -> Rects;
}
