//! Shop UI. Pure layout + render; the purchase arithmetic lives in
//! `robot_buddy_domain::economy::shop`.
//!
//! Views: a catalog of cosmetics (owned ones greyed out); once the kid picks
//! something they can afford, the embedded subtraction ("you have 12, it costs
//! 5, how many left?") as a few answer tiles; at a trade desk, the pile laid
//! out in groups with the division as tiles; and the outfit-color swatches.
//! Picking the right answer completes the deal; a wrong pick just asks them to
//! recount (never a wrong-answer buzzer).
//!
//! Layout is declarative (see `ui::layout`): [`layout`] builds a node tree
//! and returns a [`ShopLayout`] whose `Frame` both [`draw_shop`] and
//! [`handle_click`] read, so what's drawn is what's tappable.

use std::collections::BTreeSet;

use crate::prelude::*;
use robot_buddy_domain::economy::shop::{ItemKind, ShopItem, ShopKind, TradeQuote};

use crate::input::FrameInput;
use crate::ui::layout::{
    self, button, col, paint, region, row, spacer, text, Fit, Frame, Justify, Kind, Node, Page,
};
pub use crate::ui::layout::UiRect;

/// What the kid is doing in the shop right now.
pub enum ShopView<'a> {
    /// Browsing the catalog.
    Browsing,
    /// Solving the purchase subtraction for `item` (you have `balance`, it costs
    /// `cost`); tap the remainder from `choices`.
    Buying { item: &'a ShopItem, balance: u32, cost: u32, choices: &'a [u32] },
    /// Picking an outfit color for the Color Change cosmetic; `current` is the
    /// index of the color worn right now.
    PickingColor { colors: &'a [(&'static str, Color)], current: usize },
    /// At a trade desk: so many of one currency make one of another — how
    /// many does this pile make? Tap the quotient.
    Trading { quote: &'a TradeQuote, choices: &'a [u32] },
}

impl ShopView<'_> {
    /// The answer tiles on screen (empty outside a sum).
    pub fn choices(&self) -> &[u32] {
        match self {
            ShopView::Buying { choices, .. } | ShopView::Trading { choices, .. } => choices,
            _ => &[],
        }
    }
}

/// Everything the shop panel shows.
pub struct ShopModel<'a> {
    pub shop: ShopKind,
    pub catalog: &'a [ShopItem],
    pub owned: &'a BTreeSet<String>,
    /// The kid's purse in this counter's currency.
    pub balance: u32,
    pub view: ShopView<'a>,
    pub message: Option<&'a str>,
    /// Catalog page asked for (clamped when laid out).
    pub page: usize,
}

/// Every element of the shop panel the painter or the hit-test cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopId {
    Panel,
    Title,
    Balance,
    /// Where the game draws the kid while they pick a color.
    Preview,
    Item(usize),
    ItemName(usize),
    ItemPrice(usize),
    ItemBlurb(usize),
    Prompt(usize),
    Pile,
    /// Answer tile by position in the view's `choices`.
    Answer(usize),
    AnswerLabel(usize),
    Swatch(usize),
    Message,
    PrevPage,
    PrevLabel,
    NextPage,
    NextLabel,
    Done,
    DoneLabel,
}

pub struct ShopLayout {
    pub frame: Frame<ShopId>,
    /// The catalog slice on screen (everything, unless the window is short).
    pub page: Page,
}

impl ShopLayout {
    pub fn panel(&self) -> UiRect {
        self.frame.rect(ShopId::Panel).expect("shop panel always laid out")
    }
    /// Catalog row for item `index`, if it's on the current page.
    pub fn item(&self, index: usize) -> Option<UiRect> {
        self.frame.rect(ShopId::Item(index))
    }
    /// Answer tile showing `value`.
    pub fn answer(&self, view: &ShopView, value: u32) -> Option<UiRect> {
        let i = view.choices().iter().position(|&v| v == value)?;
        self.frame.rect(ShopId::Answer(i))
    }
    pub fn swatch(&self, index: usize) -> Option<UiRect> {
        self.frame.rect(ShopId::Swatch(index))
    }
    pub fn done(&self) -> UiRect {
        self.frame.rect(ShopId::Done).expect("Done is always laid out")
    }
    pub fn preview(&self) -> Option<UiRect> {
        self.frame.rect(ShopId::Preview)
    }
}

const PANEL_W: f32 = 640.0;
const PANEL_H: f32 = 560.0;
const SCREEN_MARGIN: f32 = 20.0;

/// The label on a catalog row.
fn item_label(item: &ShopItem, owned: bool) -> String {
    match item.kind {
        // The trade desk is a standing offer, never "owned".
        ItemKind::Trade { rate, into } => format!("{}  ({} = {})", item.name, item.currency.count(rate), into.count(1)),
        _ if owned => format!("{}  (owned)", item.name),
        _ => item.name.clone(),
    }
}

fn prompts(view: &ShopView) -> Vec<String> {
    match view {
        ShopView::Browsing => vec![],
        ShopView::Buying { item, balance, cost, .. } => vec![
            format!("{} costs {}.", item.name, item.currency.count(*cost)),
            format!("You have {}. How many will you have left?", balance),
        ],
        ShopView::Trading { quote, .. } => vec![
            format!("{} make one {}.", quote.from.count(quote.rate), quote.into.singular()),
            format!("You have {}. How many {} is that?", quote.offered, quote.into.label()),
        ],
        ShopView::PickingColor { .. } => vec!["Which color do you want to wear?".into()],
    }
}

fn catalog_row(i: usize, item: &ShopItem, owned: bool) -> Node<ShopId> {
    let top = row()
        .gap(12.0)
        .child(text(item_label(item, owned), 24, Fit::shrink(14)).id(ShopId::ItemName(i)).grow(1.0))
        .child(text(format!("{} {}", item.cost, item.currency.tag()), 24, Fit::shrink(14)).id(ShopId::ItemPrice(i)));
    col()
        .id(ShopId::Item(i))
        .hit()
        .h(62.0)
        .min_h(44.0)
        .pad_xy(16.0, 6.0)
        .gap(4.0)
        .justify(Justify::Center)
        .child(top)
        // What it actually does, for anything whose name doesn't say.
        .maybe((!item.blurb.is_empty()).then(|| text(item.blurb.clone(), 18, Fit::shrink(12)).id(ShopId::ItemBlurb(i))))
}

fn answer_tiles(choices: &[u32]) -> Node<ShopId> {
    // Tiles give up height before anything clips (kid-sized floor: 60px).
    row().gap(18.0).justify(Justify::Center).min_h(60.0).children(choices.iter().enumerate().map(|(i, v)| {
        button(ShopId::Answer(i), ShopId::AnswerLabel(i), v.to_string(), 36, Fit::shrink(18))
            .size(90.0, 90.0)
            .min_w(48.0)
            
    }))
}

/// Rows of the pile needed for the trade quote (the grouping IS the division).
fn pile_rows(quote: &TradeQuote) -> usize {
    (pile_count(quote)).div_ceil(quote.rate.max(1) as usize).max(1)
}

fn pile_count(quote: &TradeQuote) -> usize {
    quote.offered.min(30) as usize
}

/// Centers + radius of every pearl in the pile, fitted into `r`. Laid out in
/// rows of `rate` so a kid can count the rows instead of dividing; rows and
/// pearls scale down together when the window is short.
pub fn pile_dots(quote: &TradeQuote, r: UiRect) -> Vec<(f32, f32, f32)> {
    let per_row = quote.rate.max(1) as usize;
    let rows = pile_rows(quote) as f32;
    let (step_x, step_y, stagger, radius, inset) = (26.0, 24.0, 4.0, 8.0, 12.0);
    let natural_w = inset + (per_row as f32 - 1.0) * step_x + (rows - 1.0) * stagger + 2.0 * radius;
    let natural_h = (rows - 1.0) * step_y + 2.0 * radius;
    let scale = (r.w / natural_w).min(r.h / natural_h).min(1.0).max(0.0);
    (0..pile_count(quote))
        .map(|i| {
            let (col, row) = ((i % per_row) as f32, (i / per_row) as f32);
            let cx = r.x + (inset + radius + col * step_x + row * stagger) * scale;
            let cy = r.y + (radius + row * step_y) * scale;
            (cx, cy, radius * scale)
        })
        .collect()
}

fn build(m: &ShopModel, page: Page) -> Node<ShopId> {
    let header = row()
        .fixed()
        .child(
            col()
                .grow(1.0)
                .gap(4.0)
                .child(text(m.shop.title(), 30, Fit::shrink(18)).id(ShopId::Title).center_text())
                .child(text(format!("You have {}", m.shop.currency().count(m.balance)), 22, Fit::shrink(14)).id(ShopId::Balance)),
        )
        .maybe(matches!(m.view, ShopView::PickingColor { .. }).then(|| region(56.0, 64.0).id(ShopId::Preview).fixed()));

    let questions = col()
        .gap(8.0)
        .pad_edges(0.0, 20.0, 0.0, 0.0)
        .fixed()
        .children(prompts(&m.view).into_iter().enumerate().map(|(i, p)| text(p, 24, Fit::wrap_lines(14, 2)).id(ShopId::Prompt(i))));

    let body: Node<ShopId> = match &m.view {
        ShopView::Browsing => col().gap(10.0).grow(1.0).min_h(0.0).children(page.rows().map(|i| {
            let item = &m.catalog[i];
            catalog_row(i, item, m.owned.contains(&item.id))
        })),
        ShopView::Buying { choices, .. } => col().grow(1.0).min_h(0.0).gap(12.0).children([questions, spacer(), answer_tiles(choices)]),
        ShopView::Trading { quote, choices } => {
            let rows = pile_rows(quote) as f32;
            col().grow(1.0).min_h(0.0).gap(12.0).children([
                questions,
                region(0.0, (rows - 1.0) * 24.0 + 16.0).auto_w().id(ShopId::Pile).grow(1.0).min_h(rows * 6.0),
                answer_tiles(choices),
            ])
        }
        ShopView::PickingColor { colors, .. } => {
            let grid = col().gap(18.0).children(colors.chunks(4).enumerate().map(|(r, chunk)| {
                row().gap(18.0).justify(Justify::Center).min_h(56.0).children(
                    (0..chunk.len()).map(|c| region(90.0, 90.0).id(ShopId::Swatch(r * 4 + c)).hit().min_w(40.0)),
                )
            }));
            col().grow(1.0).min_h(0.0).gap(24.0).children([questions, grid])
        }
    };

    let small_btn = |id, label_id, label: &str| button(id, label_id, label, 22, Fit::shrink(14)).size(100.0, 38.0).fixed();
    let footer = col()
        .fixed()
        .gap(4.0)
        // Reserved even when empty, so the shelf doesn't jump when Bolt talks.
        .child(text(m.message.unwrap_or(""), 24, Fit::wrap_lines(14, 2)).id(ShopId::Message).center_text().h(40.0).fixed())
        .child(
            row()
                .gap(12.0)
                .maybe(page.has_prev().then(|| small_btn(ShopId::PrevPage, ShopId::PrevLabel, "< Back")))
                .maybe(page.has_next().then(|| small_btn(ShopId::NextPage, ShopId::NextLabel, "More >")))
                .child(spacer())
                .child(small_btn(ShopId::Done, ShopId::DoneLabel, "Done")),
        );

    layout::centered_on_screen(
        col()
            .id(ShopId::Panel)
            .size(PANEL_W, PANEL_H)
            .min_h(0.0)
            .min_w(0.0)
            .pad_edges(32.0, 12.0, 16.0, 14.0)
            .gap(12.0)
            .children([header, body, footer]),
        SCREEN_MARGIN,
    )
}

/// Lay the shop out for this screen. Pure — `Game::step` hit-tests with it and
/// `Game::render` paints it.
pub fn layout(m: &ShopModel, screen: (f32, f32)) -> ShopLayout {
    let bounds = layout::screen_rect(screen);
    let (frame, page) = match m.view {
        ShopView::Browsing => layout::paged(m.catalog.len(), m.page, bounds, |p| build(m, p)),
        _ => {
            let page = Page::all(0);
            (layout::layout(&build(m, page), bounds), page)
        }
    };
    ShopLayout { frame, page }
}

/// An input outcome the game acts on.
pub enum ShopInput {
    SelectItem(usize),
    Answer(u32),
    PickColor(usize),
    /// Show catalog page `n`.
    Page(usize),
    Close,
}

pub fn handle_click(mx: f32, my: f32, layout: &ShopLayout, view: &ShopView) -> Option<ShopInput> {
    match layout.frame.hit_at(mx, my)? {
        ShopId::Done => Some(ShopInput::Close),
        ShopId::Item(i) => Some(ShopInput::SelectItem(i)),
        ShopId::Answer(i) => view.choices().get(i).map(|&v| ShopInput::Answer(v)),
        ShopId::Swatch(i) => Some(ShopInput::PickColor(i)),
        ShopId::PrevPage => Some(ShopInput::Page(layout.page.index.saturating_sub(1))),
        ShopId::NextPage => Some(ShopInput::Page(layout.page.index + 1)),
        _ => None,
    }
}

pub fn handle_key(input: &FrameInput, view: &ShopView) -> Option<ShopInput> {
    if input.pressed(KeyCode::Escape) {
        return Some(ShopInput::Close);
    }
    // Number keys pick an answer tile while buying, or a swatch while
    // choosing an outfit color (the two are never on screen together).
    let keys = [
        KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4,
        KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8,
    ];
    let pressed = keys.iter().position(|k| input.pressed(*k))?;
    match view {
        ShopView::Buying { choices, .. } | ShopView::Trading { choices, .. } => {
            choices.get(pressed).map(|&v| ShopInput::Answer(v))
        }
        ShopView::PickingColor { colors, .. } if pressed < colors.len() => Some(ShopInput::PickColor(pressed)),
        _ => None,
    }
}

// ─── Drawing ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const ROW_BG: Color = Color::new(0.16, 0.18, 0.28, 1.0);
const OWNED_BG: Color = Color::new(0.20, 0.30, 0.22, 1.0);
const POOR_BG: Color = Color::new(0.30, 0.18, 0.18, 1.0);
const TILE_BG: Color = Color::new(0.129, 0.588, 0.953, 1.0);
const BUTTON_BG: Color = Color::new(0.329, 0.431, 0.478, 1.0);
const PEARL: Color = Color::new(0.93, 0.96, 0.99, 1.0);

/// Paint the shop from its laid-out frame: styling only, no coordinates.
pub fn draw_shop(m: &ShopModel, layout: &ShopLayout) {
    let f = &layout.frame;
    paint::dim(f.bounds, 0.5);
    for el in f.elements() {
        let Some(id) = el.id else { continue };
        let r = el.rect;
        if let Kind::Text(t) = &el.kind {
            let color = match id {
                ShopId::Title | ShopId::ItemPrice(_) | ShopId::Message => GOLD,
                ShopId::ItemBlurb(_) => Color::new(1.0, 1.0, 1.0, 0.62),
                _ => WHITE,
            };
            paint::text(t, color);
            continue;
        }
        match id {
            ShopId::Panel => paint::boxed(r, DARK_BG, 4.0, GOLD),
            ShopId::Item(i) => {
                let item = &m.catalog[i];
                let is_owned = m.owned.contains(&item.id);
                let bg = if is_owned { OWNED_BG } else if m.balance >= item.cost { ROW_BG } else { POOR_BG };
                paint::boxed(r, bg, 2.0, Color::new(1.0, 1.0, 1.0, 0.25));
            }
            ShopId::Answer(_) => paint::boxed(r, TILE_BG, 2.0, Color::new(1.0, 1.0, 1.0, 0.4)),
            ShopId::Pile => {
                if let ShopView::Trading { quote, .. } = &m.view {
                    let c = paint::canvas(r);
                    for (x, y, rad) in pile_dots(quote, r) {
                        c.circle(x, y, rad, PEARL);
                        c.circle(x - rad / 4.0, y - rad / 4.0, rad * 0.375, Color::new(1.0, 1.0, 1.0, 0.9));
                    }
                }
            }
            ShopId::Swatch(i) => {
                if let ShopView::PickingColor { colors, current } = &m.view {
                    paint::fill(r, colors[i].1);
                    if i == *current {
                        // The color being worn right now gets a thick gold frame.
                        paint::outline(r.expand(3.0), 6.0, GOLD);
                    } else {
                        paint::outline(r, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
                    }
                }
            }
            ShopId::Done | ShopId::PrevPage | ShopId::NextPage => {
                paint::boxed(r, BUTTON_BG, 2.0, Color::new(1.0, 1.0, 1.0, 0.3))
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robot_buddy_domain::economy::shop::{quote_trade, Currency};

    #[test]
    fn the_pearl_pile_stays_inside_its_region() {
        let q = quote_trade(30, Currency::Pearls, 3, Currency::DumDums);
        for r in [UiRect::new(10.0, 20.0, 500.0, 60.0), UiRect::new(0.0, 0.0, 300.0, 400.0)] {
            let dots = pile_dots(&q, r);
            assert_eq!(dots.len(), 30);
            for (x, y, rad) in dots {
                assert!(r.contains_rect(&UiRect::new(x - rad, y - rad, 2.0 * rad, 2.0 * rad)), "{x},{y} r{rad} outside {r:?}");
            }
        }
    }
}
