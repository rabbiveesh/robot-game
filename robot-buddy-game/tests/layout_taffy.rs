//! Engine differential test: the whole layout sweep, with every `layout()`
//! call also computed by taffy (the externally maintained flexbox engine we
//! may swap to) and compared node by node against `FlowEngine`.
//!
//! `TaffyEngine` below is also the reference implementation of the swap in
//! ADR-004: move it to `src/ui/layout/taffy.rs`, promote the dependency, and
//! point `DefaultEngine` at it.
//!
//! Debug builds only (`layout::with_engine` is debug-only); taffy is a
//! dev-dependency, so none of this reaches the WASM.
#![cfg(debug_assertions)]

mod sweep;

use std::cell::RefCell;
use std::rc::Rc;

use robot_buddy_game::ui::layout::engine::{LayoutEngine, LayoutTree, LeafKind};
use robot_buddy_game::ui::layout::flow::FlowEngine;
use robot_buddy_game::ui::layout::{text, with_engine, Align, Dim, Direction, Justify, Len, TextMetrics, UiRect};
use taffy::prelude::*;

// ─── The taffy engine ───────────────────────────────────

pub struct TaffyEngine;

fn dim(d: Dim) -> Dimension {
    match d {
        Dim::Auto => Dimension::auto(),
        Dim::Px(v) => Dimension::length(v),
        Dim::Percent(f) => Dimension::percent(f),
    }
}

fn len(l: Option<Len>) -> LengthPercentageAuto {
    match l {
        None => LengthPercentageAuto::auto(),
        Some(Len::Px(v)) => LengthPercentageAuto::length(v),
        Some(Len::Percent(f)) => LengthPercentageAuto::percent(f),
    }
}

fn align(a: Align) -> AlignItems {
    match a {
        Align::Start => AlignItems::START,
        Align::Center => AlignItems::CENTER,
        Align::End => AlignItems::END,
        Align::Stretch => AlignItems::STRETCH,
    }
}

/// `node::Style` → `taffy::Style`, field for field (the table in
/// `ui/layout/mod.rs`).
fn style(s: &robot_buddy_game::ui::layout::Style) -> Style {
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
            Justify::Start => JustifyContent::START,
            Justify::Center => JustifyContent::CENTER,
            Justify::End => JustifyContent::END,
            Justify::SpaceBetween => JustifyContent::SPACE_BETWEEN,
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
        // 1. Build bottom-up; each node carries its arena index as context.
        let mut t: TaffyTree<usize> = TaffyTree::new();
        // The layout pipeline rounds (layout::engine::round_edges); taffy's
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
                st.min_size = Size { width: auto(), height: auto() };
                st.max_size = Size { width: auto(), height: auto() };
            }
            let kids: Vec<NodeId> = node.children.iter().map(|&c| ids[c]).collect();
            ids[i] = if kids.is_empty() {
                t.new_leaf_with_context(st, i).unwrap()
            } else {
                t.new_with_children(st, &kids).unwrap()
            };
        }
        // 2. Lay out, measuring text leaves with the shared text module.
        let avail = Size { width: AvailableSpace::Definite(bounds.w), height: AvailableSpace::Definite(bounds.h) };
        t.compute_layout_with_measure(ids[0], avail, |input, _id, ctx, st| {
            taffy::compute_leaf_layout(input, st, |_, _| 0.0, |known, space| {
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
        })
        .unwrap();
        // 3. Absolute rects: sum the relative locations.
        let mut out = vec![UiRect::default(); n];
        fn walk(t: &TaffyTree<usize>, tree: &LayoutTree, ids: &[NodeId], i: usize, x: f32, y: f32, out: &mut [UiRect]) {
            let l = t.layout(ids[i]).unwrap();
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

// ─── The differential engine ────────────────────────────

/// Both engines are exact and share rounding and clipping, so they must agree
/// to float noise — not just to the pixel.
const TOLERANCE: f32 = 0.01;

/// Lays out with `FlowEngine` (so the sweep's assertions run on it, as in
/// production) and records the first node of each layout where taffy's
/// unrounded rect differs by more than [`TOLERANCE`] on any edge.
#[derive(Default)]
struct Differential {
    mismatches: RefCell<Vec<String>>,
    layouts: RefCell<usize>,
}

fn describe(tree: &LayoutTree, i: usize) -> String {
    let kind = match tree.nodes[i].kind {
        LeafKind::Text(s) => format!("text {:?}", s.text),
        LeafKind::Region => "region".into(),
        LeafKind::Container => format!("{:?}", tree.nodes[i].style.direction),
    };
    format!("node {i} ({kind})")
}

impl LayoutEngine for Differential {
    fn compute(&self, tree: &LayoutTree, bounds: UiRect, m: &dyn TextMetrics) -> Vec<UiRect> {
        let ours = FlowEngine.compute(tree, bounds, m);
        let theirs = TaffyEngine.compute(tree, bounds, m);
        *self.layouts.borrow_mut() += 1;
        let close = |p: &UiRect, q: &UiRect| {
            (p.x - q.x).abs() <= TOLERANCE
                && (p.y - q.y).abs() <= TOLERANCE
                && (p.right() - q.right()).abs() <= TOLERANCE
                && (p.bottom() - q.bottom()).abs() <= TOLERANCE
        };
        // Report the first disagreeing node of each layout (its descendants
        // usually disagree because it does).
        if let Some(i) = (0..ours.len()).find(|&i| !close(&ours[i], &theirs[i])) {
            let mut path = Vec::new();
            let mut parent = (0..i).rev().find(|&p| tree.nodes[p].children.contains(&i));
            while let Some(p) = parent {
                path.push(describe(tree, p));
                parent = (0..p).rev().find(|&q| tree.nodes[q].children.contains(&p));
            }
            self.mismatches.borrow_mut().push(format!(
                "{} at {:?}: flow {:?} vs taffy {:?}; in {}",
                describe(tree, i),
                (bounds.w, bounds.h),
                ours[i],
                theirs[i],
                path.join(" < ")
            ));
        }
        ours
    }
}

fn differential(name: &str, body: fn()) {
    let d = Rc::new(Differential::default());
    with_engine(d.clone(), body);
    let mismatches = d.mismatches.borrow();
    let layouts = *d.layouts.borrow();
    assert!(layouts > 0, "{name}: the sweep laid nothing out through layout()");
    if !mismatches.is_empty() {
        let mut uniq: Vec<&String> = mismatches.iter().collect();
        uniq.sort();
        uniq.dedup();
        panic!(
            "{name}: FlowEngine and taffy disagree on {} of {layouts} layouts ({} distinct):\n  - {}",
            mismatches.len(),
            uniq.len(),
            uniq.iter().take(12).map(|s| s.as_str()).collect::<Vec<_>>().join("\n  - ")
        );
    }
}

/// The CSS behaviours `flow.rs` pins in its unit tests, checked against
/// taffy directly: so "FlowEngine does what a browser does" is a fact, not
/// a claim.
#[test]
fn pinned_css_fixtures_match_taffy() {
    use robot_buddy_game::ui::layout::{col, region, row, text, Fit, FontMetrics, Node};
    let kids = || (0..3).map(|_| region::<()>(10.0, 50.0).fixed());
    let label = || text::<()>("aaaa bbbb cccc dddd", 16, Fit::wrap(16));
    let fixtures: Vec<(&str, Node<()>, (f32, f32))> = vec![
        ("fixed cross size overflows", col().align(Align::Center).child(region(300.0, 20.0)), (200.0, 100.0)),
        ("row cross overflow", row().child(region(20.0, 150.0)), (100.0, 100.0)),
        ("percent width + max", col().pad(20.0).align(Align::Center).child(region(0.0, 20.0).w_pct(1.0).max_w(760.0)), (360.0, 100.0)),
        ("percent min height", col().child(region(10.0, 0.0).auto_h().min_h_pct(0.25)), (100.0, 400.0)),
        ("justify end overflow", col().justify(Justify::End).children(kids()), (10.0, 120.0)),
        ("justify center overflow", col().justify(Justify::Center).children(kids()), (10.0, 120.0)),
        ("space-between overflow", col().justify(Justify::SpaceBetween).children(kids()), (10.0, 120.0)),
        ("fit-content, roomy", col().align(Align::Start).child(col().child(label())), (300.0, 100.0)),
        ("fit-content, tight", col().align(Align::Start).child(col().child(label())), (100.0, 100.0)),
        ("fit-content, min-content overflow", col().align(Align::Start).child(col().child(label())), (20.0, 100.0)),
        (
            "nested column can't shrink below its rows' preferred heights",
            col().child(col().children((0..3).map(|_| region::<()>(10.0, 50.0).min_h(20.0)))),
            (10.0, 90.0),
        ),
        (
            "...unless it says min_h(0)",
            col().child(col().min_h(0.0).children((0..3).map(|_| region::<()>(10.0, 50.0).min_h(20.0)))),
            (10.0, 90.0),
        ),
        (
            "shrink scales by the inner basis",
            row().children([region::<()>(100.0, 10.0).pad_xy(40.0, 0.0), region(100.0, 10.0)]),
            (150.0, 10.0),
        ),
    ];
    let m = FontMetrics::bundled();
    for (name, root, (w, h)) in fixtures {
        let tree = LayoutTree::new(&root);
        let bounds = UiRect::new(0.0, 0.0, w, h);
        let (ours, theirs) = (FlowEngine.compute(&tree, bounds, m), TaffyEngine.compute(&tree, bounds, m));
        for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
            let far = (a.x - b.x).abs().max((a.y - b.y).abs()).max((a.w - b.w).abs()).max((a.h - b.h).abs());
            assert!(far <= TOLERANCE, "{name}: node {i} flow {a:?} vs taffy {b:?}");
        }
    }
}

macro_rules! differential_tests {
    ($($name:ident => $body:path),* $(,)?) => {
        $(#[test] fn $name() { differential(stringify!($name), $body) })*
    };
}

differential_tests! {
    bolt_catalog => sweep::bolt_catalog_is_sane_everywhere,
    hermie_catalog => sweep::hermie_catalog_is_sane_everywhere,
    hermies_full_shelf => sweep::hermies_full_shelf_fits_one_page_at_the_default_window,
    buying_and_trading => sweep::buying_and_trading_are_sane_everywhere,
    swag_picker => sweep::swag_picker_is_sane_with_a_full_wardrobe,
    tall_wardrobe => sweep::a_tall_screen_shows_the_whole_wardrobe_on_one_page,
    empty_swag => sweep::empty_swag_picker_is_sane,
    challenge_phases => sweep::challenge_sweep::every_challenge_phase_is_sane_everywhere,
    challenge_buttons_stay_put => sweep::challenge_sweep::a_wrong_answer_does_not_move_the_answer_buttons,
    challenge_taps => sweep::challenge_sweep::tapping_a_drawn_button_answers_it_even_under_a_wrapped_question,
    dialogue => sweep::dialogue_lines_are_sane_everywhere,
    settings => sweep::settings_overlay_is_sane_everywhere,
    quest => sweep::quest_beats_are_sane_everywhere,
}
