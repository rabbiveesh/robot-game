//! The panel-facing vocabulary: a declarative node tree.
//!
//! Panels describe *structure and sizes* here and never compute a coordinate.
//! The vocabulary is deliberately flexbox-shaped — and flexbox **only** (no
//! grid, no absolute positioning) — so every field maps 1:1 onto a
//! `taffy::Style` field. See `ui/layout/mod.rs` for the mapping table and
//! `docs/adr/004-ui-layout.md` for the swap recipe.

/// Main axis of a container (`taffy::FlexDirection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Row,
    Column,
}

/// Cross-axis placement (`taffy::AlignItems` / `AlignSelf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

/// Main-axis distribution of leftover space (`taffy::JustifyContent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
}

/// A preferred size (`taffy::Dimension`). Percentages aren't needed yet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dim {
    Auto,
    Px(f32),
}

/// Padding (`taffy::Rect<LengthPercentage>`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Edges {
    pub fn all(v: f32) -> Self {
        Edges { left: v, right: v, top: v, bottom: v }
    }
    pub fn xy(x: f32, y: f32) -> Self {
        Edges { left: x, right: x, top: y, bottom: y }
    }
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

/// Box style. Every field is a `taffy::Style` field of the same name.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub direction: Direction,
    pub padding: Edges,
    pub gap: f32,
    pub width: Dim,
    pub height: Dim,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub align_items: Align,
    pub align_self: Option<Align>,
    pub justify_content: Justify,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            direction: Direction::Column,
            padding: Edges::default(),
            gap: 0.0,
            width: Dim::Auto,
            height: Dim::Auto,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            flex_grow: 0.0,
            // CSS / taffy default: items may shrink.
            flex_shrink: 1.0,
            align_items: Align::Stretch,
            align_self: None,
            justify_content: Justify::Start,
        }
    }
}

/// What a text leaf does when it doesn't fit at its preferred size. There is
/// no default — every text node states its policy, so "runs off the panel"
/// can't happen by omission.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fit {
    /// One line; shrink the font down to `min` to fit the width.
    Shrink { min: u16 },
    /// Word-wrap at the preferred size; shrink toward `min` only if the lines
    /// don't fit the height. `max_lines == 0` means unbounded.
    Wrap { min: u16, max_lines: usize },
    /// Prefer one line, shrinking down to `min`; only then wrap (at `min`).
    ShrinkThenWrap { min: u16, max_lines: usize },
    /// One line at the preferred size; cut with "..." if too wide. The only
    /// policy under which truncation is intended rather than a layout bug.
    Ellipsis,
}

impl Fit {
    pub fn shrink(min: u16) -> Self {
        Fit::Shrink { min }
    }
    pub fn wrap(min: u16) -> Self {
        Fit::Wrap { min, max_lines: 0 }
    }
    pub fn wrap_lines(min: u16, max_lines: usize) -> Self {
        Fit::Wrap { min, max_lines }
    }
    pub fn shrink_then_wrap(min: u16, max_lines: usize) -> Self {
        Fit::ShrinkThenWrap { min, max_lines }
    }
    pub fn min_size(&self, size: u16) -> u16 {
        match *self {
            Fit::Shrink { min } | Fit::Wrap { min, .. } | Fit::ShrinkThenWrap { min, .. } => min.min(size),
            Fit::Ellipsis => size,
        }
    }
    pub fn max_lines(&self) -> usize {
        match *self {
            Fit::Wrap { max_lines, .. } | Fit::ShrinkThenWrap { max_lines, .. } => max_lines,
            Fit::Shrink { .. } | Fit::Ellipsis => 1,
        }
    }
}

/// Horizontal placement of each line inside the text node's box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextSpec {
    pub text: String,
    /// Preferred (largest) font size in px.
    pub size: u16,
    pub fit: Fit,
    pub align: TextAlign,
    /// Extra space between wrapped lines at the preferred size (scales down
    /// with the font).
    pub line_gap: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Content<Id> {
    Container(Vec<Node<Id>>),
    Text(TextSpec),
    /// A leaf reserved for custom painting (a sprite, the pearl pile, a CRA
    /// visual). Sized by its style; the panel paints inside its rect.
    Region,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node<Id> {
    pub style: Style,
    pub content: Content<Id>,
    pub id: Option<Id>,
    /// A tap target: `Frame::hit_at` reports this node's id.
    pub hit: bool,
}

// ─── Constructors ───────────────────────────────────────

/// A vertical stack.
pub fn col<Id>() -> Node<Id> {
    Node { style: Style::default(), content: Content::Container(Vec::new()), id: None, hit: false }
}

/// A horizontal stack. Children are vertically centered by default.
pub fn row<Id>() -> Node<Id> {
    let mut n = col();
    n.style.direction = Direction::Row;
    n.style.align_items = Align::Center;
    n
}

/// A text leaf. `fit` is required: say what happens when it doesn't fit.
pub fn text<Id>(s: impl Into<String>, size: u16, fit: Fit) -> Node<Id> {
    Node {
        style: Style::default(),
        content: Content::Text(TextSpec {
            text: s.into(),
            size,
            fit,
            align: TextAlign::Left,
            line_gap: (size as f32 * 0.25).round(),
        }),
        id: None,
        hit: false,
    }
}

/// A custom-painted leaf of the given preferred size.
pub fn region<Id>(w: f32, h: f32) -> Node<Id> {
    let mut n = Node { style: Style::default(), content: Content::Region, id: None, hit: false };
    n.style.width = Dim::Px(w);
    n.style.height = Dim::Px(h);
    n
}

/// Flexible empty space that soaks up leftover room.
pub fn spacer<Id>() -> Node<Id> {
    let mut n = Node { style: Style::default(), content: Content::Region, id: None, hit: false };
    n.style.flex_grow = 1.0;
    n.style.min_width = Some(0.0);
    n.style.min_height = Some(0.0);
    n
}

/// Fixed-size empty space.
pub fn gap_box<Id>(w: f32, h: f32) -> Node<Id> {
    region(w, h).fixed()
}

/// A tappable box with a centered label: the common kid-sized button. The box
/// gets `id`; the label gets `label_id`.
pub fn button<Id>(id: Id, label_id: Id, label: impl Into<String>, size: u16, fit: Fit) -> Node<Id> {
    row().id(id).hit().justify(Justify::Center).child(text(label, size, fit).id(label_id).center_text())
}

// ─── Chainable modifiers ────────────────────────────────

impl<Id> Node<Id> {
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
    /// Make this node a tap target (it must also have an id).
    pub fn hit(mut self) -> Self {
        self.hit = true;
        self
    }
    pub fn pad(mut self, v: f32) -> Self {
        self.style.padding = Edges::all(v);
        self
    }
    pub fn pad_xy(mut self, x: f32, y: f32) -> Self {
        self.style.padding = Edges::xy(x, y);
        self
    }
    pub fn pad_edges(mut self, left: f32, top: f32, right: f32, bottom: f32) -> Self {
        self.style.padding = Edges { left, right, top, bottom };
        self
    }
    pub fn gap(mut self, v: f32) -> Self {
        self.style.gap = v;
        self
    }
    pub fn w(mut self, v: f32) -> Self {
        self.style.width = Dim::Px(v);
        self
    }
    pub fn h(mut self, v: f32) -> Self {
        self.style.height = Dim::Px(v);
        self
    }
    /// Content/stretch width instead of a fixed one.
    pub fn auto_w(mut self) -> Self {
        self.style.width = Dim::Auto;
        self
    }
    /// Content height instead of a fixed one.
    pub fn auto_h(mut self) -> Self {
        self.style.height = Dim::Auto;
        self
    }
    pub fn size(self, w: f32, h: f32) -> Self {
        self.w(w).h(h)
    }
    pub fn min_w(mut self, v: f32) -> Self {
        self.style.min_width = Some(v);
        self
    }
    pub fn max_w(mut self, v: f32) -> Self {
        self.style.max_width = Some(v);
        self
    }
    pub fn min_h(mut self, v: f32) -> Self {
        self.style.min_height = Some(v);
        self
    }
    pub fn max_h(mut self, v: f32) -> Self {
        self.style.max_height = Some(v);
        self
    }
    pub fn grow(mut self, v: f32) -> Self {
        self.style.flex_grow = v;
        self
    }
    pub fn shrink(mut self, v: f32) -> Self {
        self.style.flex_shrink = v;
        self
    }
    /// Never shrink: reserved space (headers, footers, buttons).
    pub fn fixed(self) -> Self {
        self.shrink(0.0)
    }
    pub fn align(mut self, a: Align) -> Self {
        self.style.align_items = a;
        self
    }
    pub fn align_self(mut self, a: Align) -> Self {
        self.style.align_self = Some(a);
        self
    }
    pub fn justify(mut self, j: Justify) -> Self {
        self.style.justify_content = j;
        self
    }
    pub fn child(mut self, c: Node<Id>) -> Self {
        match &mut self.content {
            Content::Container(kids) => kids.push(c),
            _ => panic!("child() on a leaf node"),
        }
        self
    }
    pub fn children(mut self, cs: impl IntoIterator<Item = Node<Id>>) -> Self {
        match &mut self.content {
            Content::Container(kids) => kids.extend(cs),
            _ => panic!("children() on a leaf node"),
        }
        self
    }
    /// Add `c` only when present — keeps conditional structure declarative.
    pub fn maybe(self, c: Option<Node<Id>>) -> Self {
        match c {
            Some(c) => self.child(c),
            None => self,
        }
    }

    fn text_spec(&mut self) -> &mut TextSpec {
        match &mut self.content {
            Content::Text(t) => t,
            _ => panic!("text modifier on a non-text node"),
        }
    }
    pub fn center_text(mut self) -> Self {
        self.text_spec().align = TextAlign::Center;
        self
    }
    pub fn right_text(mut self) -> Self {
        self.text_spec().align = TextAlign::Right;
        self
    }
    pub fn line_gap(mut self, v: f32) -> Self {
        self.text_spec().line_gap = v;
        self
    }

    /// Nodes in this subtree, self included (pre-order count).
    pub fn subtree_len(&self) -> usize {
        1 + match &self.content {
            Content::Container(kids) => kids.iter().map(|k| k.subtree_len()).sum(),
            _ => 0,
        }
    }
}
