//! The engine seam: node tree in, one rect per node out.
//!
//! An engine only answers "where does every box go". It knows nothing about
//! ids, hit targets, painting, or how a text leaf's lines are broken — that is
//! [`Frame::resolve`](super::frame::Frame::resolve)'s job, shared by all
//! engines. Swapping the in-house [`FlowEngine`](super::flow::FlowEngine) for
//! a taffy-backed one is therefore one new impl of this trait plus flipping
//! the `DefaultEngine` alias in `layout/mod.rs`.

use super::metrics::TextMetrics;
use super::node::Node;
use super::rect::UiRect;

/// Rects for every node of a tree, in pre-order (index 0 is the root). `None`
/// means the node didn't fit in its parent and was clipped out, together with
/// its whole subtree.
pub type Rects = Vec<Option<UiRect>>;

pub trait LayoutEngine {
    /// Lay `root` out inside `bounds`. Contract every engine must keep:
    /// * every placed child lies inside its parent's padding box, and
    /// * siblings never overlap;
    /// anything that can't satisfy both is returned as `None` (clipped).
    /// Text leaves are measured with [`super::text`] against `metrics`.
    fn compute<Id>(&self, root: &Node<Id>, bounds: UiRect, metrics: &dyn TextMetrics) -> Rects;
}
