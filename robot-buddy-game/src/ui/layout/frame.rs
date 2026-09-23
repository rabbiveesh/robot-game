//! The resolved layout both halves of a panel consume: `draw_*` loops over it
//! to paint, `handle_click` asks it what was tapped. Because both read the
//! same `Frame`, a hit rect can't drift away from the thing drawn there.

use std::fmt::Debug;

use super::engine::Rects;
use super::metrics::TextMetrics;
use super::node::{Content, Node, TextAlign};
use super::rect::UiRect;
use super::text;

/// One placed line of text: its line box and the baseline to draw it on.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedLine {
    pub text: String,
    pub rect: UiRect,
    pub baseline: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacedText {
    /// Font size the lines were fitted at.
    pub size: u16,
    pub lines: Vec<PlacedLine>,
    /// The text's policy couldn't fit it and it was cut (a layout bug; see
    /// [`text::Shaped::overflowed`]).
    pub overflowed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// A container (drawn only if the panel styles its id).
    Box,
    /// A custom-painted leaf.
    Region,
    Text(PlacedText),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element<Id> {
    pub id: Option<Id>,
    pub rect: UiRect,
    /// Index of the nearest emitted ancestor in `Frame::elements`.
    pub parent: Option<usize>,
    pub hit: bool,
    pub kind: Kind,
}

/// A node the engine dropped because it didn't fit.
#[derive(Debug, Clone, PartialEq)]
pub struct Clipped<Id> {
    /// Pre-order index of the node in the panel's tree.
    pub node: usize,
    pub id: Option<Id>,
    /// The text, if it was a text leaf.
    pub text: Option<String>,
}

impl<Id: Debug> Clipped<Id> {
    /// Human-readable: the id, or the text, or "region (node n)". Built on
    /// demand so release builds don't format every panel's ids.
    pub fn what(&self) -> String {
        match (&self.id, &self.text) {
            (Some(id), _) => format!("{id:?}"),
            (None, Some(t)) => format!("text {t:?}"),
            (None, None) => format!("region (node {})", self.node),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frame<Id> {
    pub bounds: UiRect,
    elements: Vec<Element<Id>>,
    clipped: Vec<Clipped<Id>>,
}

impl<Id: Copy + PartialEq + Debug> Frame<Id> {
    /// Turn an engine's rects into placed elements. Every node that has an id,
    /// is a text leaf, or is a region becomes an element (in pre-order, so
    /// paint order = tree order and parents precede children); anonymous
    /// containers only pass structure through. Every clipped node that would
    /// have become an element — with or without an id — is recorded in
    /// [`Frame::clipped`], so nothing can vanish silently.
    pub fn resolve(root: &Node<Id>, rects: &Rects, bounds: UiRect, m: &dyn TextMetrics) -> Self {
        let mut f = Frame { bounds, elements: Vec::new(), clipped: Vec::new() };
        let mut idx = 0;
        f.walk(root, rects, None, &mut idx, m);
        f
    }

    fn walk(&mut self, n: &Node<Id>, rects: &Rects, parent: Option<usize>, idx: &mut usize, m: &dyn TextMetrics) {
        let me = *idx;
        *idx += 1;
        let Some(rect) = rects[me] else {
            self.collect_clipped(n, me);
            *idx += n.subtree_len() - 1;
            return;
        };
        let kind = match &n.content {
            Content::Text(spec) => {
                let shaped = text::shape(spec, rect.w, rect.h, m);
                let lh = m.line_height(shaped.size);
                let gap = text::line_gap(spec, shaped.size);
                let block = text::block_height(spec, shaped.lines.len(), shaped.size, m);
                // Lines are centered vertically in the node's box.
                let mut y = rect.y + ((rect.h - block) / 2.0).max(0.0);
                let lines = shaped
                    .lines
                    .into_iter()
                    .map(|t| {
                        let w = m.width(&t, shaped.size).min(rect.w);
                        let x = match spec.align {
                            TextAlign::Left => rect.x,
                            TextAlign::Center => rect.x + (rect.w - w) / 2.0,
                            TextAlign::Right => rect.right() - w,
                        };
                        let line = PlacedLine {
                            text: t,
                            rect: UiRect::new(x, y, w, lh),
                            baseline: y + m.ascent(shaped.size),
                        };
                        y += lh + gap;
                        line
                    })
                    .collect();
                Some(Kind::Text(PlacedText { size: shaped.size, lines, overflowed: shaped.overflowed }))
            }
            Content::Region => Some(Kind::Region),
            Content::Container(_) => n.id.is_some().then_some(Kind::Box),
        };
        let parent_for_kids = match kind {
            Some(kind) => {
                self.elements.push(Element { id: n.id, rect, parent, hit: n.hit, kind });
                Some(self.elements.len() - 1)
            }
            None => parent,
        };
        if let Content::Container(kids) = &n.content {
            for c in kids {
                self.walk(c, rects, parent_for_kids, idx, m);
            }
        }
    }

    /// Record `n` (pre-order index `at`) and its subtree as clipped.
    fn collect_clipped(&mut self, n: &Node<Id>, at: usize) {
        // Anonymous containers aren't elements; their children are recorded.
        if n.id.is_some() || !matches!(n.content, Content::Container(_)) {
            let text = match &n.content {
                Content::Text(t) => Some(t.text.clone()),
                _ => None,
            };
            self.clipped.push(Clipped { node: at, id: n.id, text });
        }
        if let Content::Container(kids) = &n.content {
            let mut i = at + 1;
            for c in kids {
                self.collect_clipped(c, i);
                i += c.subtree_len();
            }
        }
    }

    /// Every placed element, parents before children.
    pub fn elements(&self) -> &[Element<Id>] {
        &self.elements
    }

    /// Every node that didn't fit and was dropped (ids or not).
    pub fn clipped(&self) -> &[Clipped<Id>] {
        &self.clipped
    }

    pub fn get(&self, id: Id) -> Option<&Element<Id>> {
        self.elements.iter().find(|e| e.id == Some(id))
    }

    pub fn rect(&self, id: Id) -> Option<UiRect> {
        self.get(id).map(|e| e.rect)
    }

    pub fn text(&self, id: Id) -> Option<&PlacedText> {
        match &self.get(id)?.kind {
            Kind::Text(t) => Some(t),
            _ => None,
        }
    }

    /// The deepest tap target under the point.
    pub fn hit_at(&self, x: f32, y: f32) -> Option<Id> {
        self.elements.iter().rev().find(|e| e.hit && e.rect.contains(x, y)).and_then(|e| e.id)
    }

    /// Every tap target, in tree order.
    pub fn hits(&self) -> impl Iterator<Item = (Id, UiRect)> + '_ {
        self.elements.iter().filter(|e| e.hit).filter_map(|e| e.id.map(|id| (id, e.rect)))
    }

    /// `a` is `b` or one of its ancestors.
    pub fn is_ancestor_or_self(&self, a: usize, mut b: usize) -> bool {
        loop {
            if a == b {
                return true;
            }
            match self.elements[b].parent {
                Some(p) => b = p,
                None => return false,
            }
        }
    }
}
