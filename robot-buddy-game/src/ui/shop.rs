//! Shop UI, as a Bevy UI panel; the purchase arithmetic lives in
//! `robot_buddy_domain::economy::shop`.
//!
//! Views: a catalog of cosmetics (owned ones greyed out); once the kid picks
//! something they can afford, the embedded subtraction ("you have 12, it costs
//! 5, how many left?") as a few answer tiles; at a trade desk, the pile laid
//! out in groups with the division as tiles; and the outfit-color swatches.
//! Picking the right answer completes the deal; a wrong pick just asks them to
//! recount (never a wrong-answer buzzer). A long catalog scrolls.

use std::collections::BTreeSet;

use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::{
    default, AlignItems, Bundle, ChildSpawner, Children, JustifyContent, Node, SpawnRelated, UiRect as Edges, Val,
};
use bevy::text::Justify;
use bevy::ui::{percent, px, BackgroundColor, BorderColor};

use crate::prelude::*;
use crate::save::Gender;
use robot_buddy_domain::economy::shop::{ItemKind, ShopItem, ShopKind, TradeQuote};

use super::kit::{self, backdrop, button, col, fitted, Art, Panel};
use super::views::UiViews;
use super::{swatches, ArtId, UiAction};
use crate::input::FrameInput;
pub use crate::ui::rect::UiRect;

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
}

/// The shop's buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopId {
    Item(usize),
    /// Answer tile by position in the view's `choices`.
    Answer(usize),
    Swatch(usize),
    Done,
}
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
    let scale = (r.w / natural_w).min(r.h / natural_h).clamp(0.0, 1.0);
    (0..pile_count(quote))
        .map(|i| {
            let (col, row) = ((i % per_row) as f32, (i / per_row) as f32);
            let cx = r.x + (inset + radius + col * step_x + row * stagger) * scale;
            let cy = r.y + (radius + row * step_y) * scale;
            (cx, cy, radius * scale)
        })
        .collect()
}

const PANEL_W: f32 = 640.0;
const PANEL_H: f32 = 560.0;
const SCREEN_MARGIN: f32 = 20.0;

/// The kid as they'd look in Bolt's mirror.
#[derive(Clone, PartialEq, Debug)]
pub struct KidLook {
    pub gender: Gender,
    pub worn: BTreeSet<String>,
    pub color: String,
}

/// One catalog row as shown.
#[derive(Clone, PartialEq, Debug)]
pub struct ShopRow {
    pub label: String,
    pub price: String,
    pub blurb: String,
    pub bg: Color,
}

/// Everything the shop panel shows, owned (see [`ShopModel`] for the live,
/// borrowed version the game reads input against).
#[derive(Clone, PartialEq, Debug)]
pub struct ShopPanelView {
    pub title: String,
    pub balance: String,
    pub prompts: Vec<String>,
    /// The catalog, while browsing.
    pub rows: Vec<ShopRow>,
    pub choices: Vec<u32>,
    /// The trade desk's pile.
    pub pile: Option<TradeQuote>,
    /// Swatch colours and the one worn, while picking a colour.
    pub swatches: Option<(Vec<Color>, usize)>,
    /// The kid, while picking a colour.
    pub preview: Option<KidLook>,
    pub message: Option<String>,
}

impl ShopPanelView {
    pub fn new(m: &ShopModel, look: KidLook) -> Self {
        let browsing = matches!(m.view, ShopView::Browsing);
        ShopPanelView {
            title: m.shop.title().to_string(),
            balance: format!("You have {}", m.shop.currency().count(m.balance)),
            prompts: prompts(&m.view),
            rows: if browsing {
                m.catalog.iter().map(|item| {
                    let owned = m.owned.contains(&item.id);
                    let bg = if owned { OWNED_BG } else if m.balance >= item.cost { ROW_BG } else { POOR_BG };
                    ShopRow {
                        label: item_label(item, owned),
                        price: format!("{} {}", item.cost, item.currency.tag()),
                        blurb: item.blurb.clone(),
                        bg,
                    }
                }).collect()
            } else {
                vec![]
            },
            choices: m.view.choices().to_vec(),
            pile: match &m.view {
                ShopView::Trading { quote, .. } => Some(**quote),
                _ => None,
            },
            swatches: match &m.view {
                ShopView::PickingColor { colors, current } => Some((colors.iter().map(|c| c.1).collect(), *current)),
                _ => None,
            },
            preview: matches!(m.view, ShopView::PickingColor { .. }).then_some(look),
            message: m.message.map(str::to_string),
        }
    }
}

pub struct ShopPanel;

impl Panel for ShopPanel {
    type View = ShopPanelView;
    const Z: i32 = 40;
    fn view(views: &UiViews) -> Option<&ShopPanelView> {
        views.shop.as_ref()
    }
    fn bundle(v: &ShopPanelView) -> impl Bundle {
        let v = v.clone();
        (backdrop(0.5, SCREEN_MARGIN), Children::spawn(SpawnWith(move |p: &mut ChildSpawner| spawn_panel(p, &v))))
    }
}

fn spawn_panel(p: &mut ChildSpawner, v: &ShopPanelView) {
    let panel = Node {
        // PANEL_W wide, or the whole screen if that's narrower.
        width: percent(100),
        max_width: px(PANEL_W),
        height: px(PANEL_H),
        max_height: percent(100),
        min_height: px(0),
        padding: Edges::new(px(32), px(16), px(12), px(14)),
        row_gap: px(12),
        border: Edges::all(px(4)),
        ..col()
    };
    p.spawn((panel, BackgroundColor(kit::color(DARK_BG)), BorderColor::all(kit::color(GOLD)))).with_children(|p| {
        // Header: title and purse, and the kid's mirror while picking a colour.
        p.spawn(Node { flex_shrink: 0.0, ..default() }).with_children(|p| {
            p.spawn(Node { flex_grow: 1.0, min_width: px(0), row_gap: px(4), ..col() }).with_children(|p| {
                p.spawn(fitted(v.title.clone(), 30.0, 18.0, false, GOLD, Justify::Center));
                p.spawn(fitted(v.balance.clone(), 22.0, 14.0, false, WHITE, Justify::Left));
            });
            if v.preview.is_some() {
                p.spawn((Node { width: px(56), height: px(64), flex_shrink: 0.0, ..default() }, Art(ArtId::ShopPreview)));
            }
        });
        // The body takes the room left; a long catalog scrolls inside it.
        p.spawn(Node { flex_grow: 1.0, min_height: px(0), row_gap: px(12), ..col() }).with_children(|p| body(p, v));
        // Footer: Bolt's message (room reserved even when he's quiet, so the
        // shelf doesn't jump when he talks), then Done.
        p.spawn(Node { flex_shrink: 0.0, row_gap: px(4), ..col() }).with_children(|p| {
            p.spawn(Node { height: px(40), flex_shrink: 0.0, ..col() })
                .with_child(kit::filled(v.message.clone().unwrap_or_default(), 24.0, 12.0, GOLD, Justify::Center));
            p.spawn(Node { justify_content: JustifyContent::End, ..default() }).with_children(|p| {
                let node = Node { width: px(100), height: px(38), min_width: px(72), flex_shrink: 1.0, ..kit::centered() };
                p.spawn(button(UiAction::Shop(ShopId::Done), node, BUTTON_BG, Color::new(1.0, 1.0, 1.0, 0.3)))
                    .with_child(fitted("Done", 22.0, 14.0, false, WHITE, Justify::Center));
            });
        });
    });
}

fn body(p: &mut ChildSpawner, v: &ShopPanelView) {
    if !v.prompts.is_empty() {
        p.spawn(Node { row_gap: px(8), padding: Edges::top(px(20)), flex_shrink: 0.0, ..col() }).with_children(|p| {
            for prompt in &v.prompts {
                p.spawn(kit::text(prompt.clone(), 24.0, WHITE));
            }
        });
    }
    if !v.rows.is_empty() {
        p.spawn(kit::scroll_list("shop", 10.0)).with_children(|p| {
            for (i, row) in v.rows.iter().enumerate() {
                catalog_row(p, i, row);
            }
        });
    }
    if let Some(quote) = &v.pile {
        let rows = pile_rows(quote) as f32;
        let pile = Node { height: px((rows - 1.0) * 24.0 + 16.0), flex_grow: 1.0, min_height: px(rows * 6.0), ..default() };
        p.spawn((pile, Art(ArtId::ShopPile)));
    } else if !v.choices.is_empty() {
        // Pushes the tiles to the bottom.
        p.spawn(Node { flex_grow: 1.0, ..default() });
    }
    if !v.choices.is_empty() {
        // Tiles give up height before anything clips (kid-sized floor: 60px):
        // the row shrinks, the tiles stretch to it.
        let row = Node {
            column_gap: px(18),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            height: px(90),
            min_height: px(60),
            ..default()
        };
        p.spawn(row).with_children(|p| {
            for (i, v) in v.choices.iter().enumerate() {
                let node = Node { width: px(90), min_width: px(48), ..kit::centered() };
                p.spawn(button(UiAction::Shop(ShopId::Answer(i)), node, TILE_BG, Color::new(1.0, 1.0, 1.0, 0.4)))
                    .with_child(fitted(v.to_string(), 36.0, 18.0, false, WHITE, Justify::Center));
            }
        });
    }
    if let Some((colors, current)) = &v.swatches {
        swatches::grid(p, colors, *current, |i| UiAction::Shop(ShopId::Swatch(i)));
    }
}

fn catalog_row(p: &mut ChildSpawner, i: usize, row: &ShopRow) {
    let has_blurb = !row.blurb.is_empty();
    // A blurb may need a second line on a narrow phone, so its row sizes to
    // its content; plain rows keep a fixed height.
    let node = Node {
        min_height: px(if has_blurb { 62.0 } else { 44.0 }),
        height: if has_blurb { Val::Auto } else { px(62) },
        flex_shrink: 0.0,
        padding: Edges::axes(px(16), px(6)),
        row_gap: px(4),
        justify_content: JustifyContent::Center,
        ..col()
    };
    p.spawn(button(UiAction::Shop(ShopId::Item(i)), node, row.bg, Color::new(1.0, 1.0, 1.0, 0.25))).with_children(|p| {
        p.spawn(Node { column_gap: px(12), ..default() }).with_children(|p| {
            p.spawn(Node { flex_grow: 1.0, min_width: px(0), ..col() })
                .with_child(fitted(row.label.clone(), 24.0, 14.0, true, WHITE, Justify::Left));
            p.spawn(fitted(row.price.clone(), 24.0, 14.0, false, GOLD, Justify::Left));
        });
        // What it actually does, for anything whose name doesn't say.
        if has_blurb {
            p.spawn(kit::text(row.blurb.clone(), 18.0, Color::new(1.0, 1.0, 1.0, 0.62)));
        }
    });
}

/// Paint art slot `id` (`w`×`h`, local coordinates).
pub fn draw_art(id: ArtId, v: &ShopPanelView, w: f32, h: f32) {
    match id {
        ArtId::ShopPile => {
            if let Some(quote) = &v.pile {
                for (x, y, rad) in pile_dots(quote, UiRect::new(0.0, 0.0, w, h)) {
                    draw_circle(x, y, rad, PEARL);
                    draw_circle(x - rad / 4.0, y - rad / 4.0, rad * 0.375, Color::new(1.0, 1.0, 1.0, 0.9));
                }
            }
        }
        ArtId::ShopPreview => {
            if let Some(look) = &v.preview {
                use crate::sprites::{self, Dir};
                let (px, py) = (4.0, 14.0);
                let time = get_time() as f32;
                match look.gender {
                    Gender::Boy => sprites::player::draw_player_boy(px, py, Dir::Down, 0, time),
                    Gender::Girl => sprites::player::draw_player_girl(px, py, Dir::Down, 0, time),
                }
                sprites::player::draw_player_cosmetics(px, py, Dir::Down, 0, &look.worn, &look.color);
            }
        }
        _ => {}
    }
}

/// An input outcome the game acts on.
pub enum ShopInput {
    SelectItem(usize),
    Answer(u32),
    PickColor(usize),
    Close,
}

pub fn handle_click(input: &FrameInput, view: &ShopView) -> Option<ShopInput> {
    let Some(UiAction::Shop(id)) = input.ui_press else { return None };
    match id {
        ShopId::Done => Some(ShopInput::Close),
        ShopId::Item(i) => Some(ShopInput::SelectItem(i)),
        ShopId::Answer(i) => view.choices().get(i).map(|&v| ShopInput::Answer(v)),
        ShopId::Swatch(i) => Some(ShopInput::PickColor(i)),
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

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const ROW_BG: Color = Color::new(0.16, 0.18, 0.28, 1.0);
const OWNED_BG: Color = Color::new(0.20, 0.30, 0.22, 1.0);
const POOR_BG: Color = Color::new(0.30, 0.18, 0.18, 1.0);
const TILE_BG: Color = Color::new(0.129, 0.588, 0.953, 1.0);
const BUTTON_BG: Color = Color::new(0.329, 0.431, 0.478, 1.0);
const PEARL: Color = Color::new(0.93, 0.96, 0.99, 1.0);

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
