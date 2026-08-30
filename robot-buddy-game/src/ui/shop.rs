//! Shop UI. Pure layout + render; the purchase arithmetic lives in
//! `robot_buddy_domain::economy::shop`.
//!
//! Two views: a catalog of cosmetics (owned ones greyed out), and — once the
//! kid picks something they can afford — the embedded subtraction ("you have
//! 12, it costs 5, how many left?") presented as a few answer tiles. Picking
//! the right remainder completes the purchase; a wrong pick just asks them to
//! recount (never a wrong-answer buzzer).

use crate::prelude::*;
use robot_buddy_domain::economy::shop::{ItemKind, ShopItem, ShopKind, TradeQuote};

use crate::input::FrameInput;

#[derive(Debug, Clone, Copy)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UiRect {
    pub fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + self.h
    }
}

pub struct ItemRow {
    pub rect: UiRect,
    pub index: usize,
}

pub struct AnswerTile {
    pub rect: UiRect,
    pub value: u32,
}

pub struct SwatchTile {
    pub rect: UiRect,
    pub index: usize,
}

pub struct ShopLayout {
    pub panel: UiRect,
    pub items: Vec<ItemRow>,
    pub answers: Vec<AnswerTile>,
    pub swatches: Vec<SwatchTile>,
    pub close_btn: UiRect,
}

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
    /// At Hermie's trade desk: so many pearls to a Dum Dum, how many Dum Dums
    /// does this pile make? Tap the quotient.
    Trading { quote: &'a TradeQuote, choices: &'a [u32] },
}

pub fn layout(catalog: &[ShopItem], view: &ShopView, screen: (f32, f32)) -> ShopLayout {
    let (sw, sh) = screen;
    let panel_w = (sw - 40.0).min(640.0);
    let panel_h = (sh - 40.0).min(560.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;

    let mut items = Vec::new();
    let mut answers = Vec::new();
    let mut swatches = Vec::new();

    match view {
        ShopView::Browsing => {
            // Tall enough for a name plus the one-line blurb underneath it.
            let row_h = 62.0;
            let gap = 10.0;
            let row_w = panel_w - 64.0;
            let start_x = panel_x + 32.0;
            let start_y = panel_y + 80.0;
            for i in 0..catalog.len() {
                items.push(ItemRow {
                    rect: UiRect { x: start_x, y: start_y + i as f32 * (row_h + gap), w: row_w, h: row_h },
                    index: i,
                });
            }
        }
        ShopView::Buying { choices, .. } | ShopView::Trading { choices, .. } => {
            let tile = 90.0;
            let gap = 18.0;
            let n = choices.len();
            let total = tile * n as f32 + gap * (n as f32 - 1.0).max(0.0);
            let start_x = panel_x + (panel_w - total) / 2.0;
            let y = panel_y + panel_h - 180.0;
            for (i, &v) in choices.iter().enumerate() {
                answers.push(AnswerTile {
                    rect: UiRect { x: start_x + i as f32 * (tile + gap), y, w: tile, h: tile },
                    value: v,
                });
            }
        }
        ShopView::PickingColor { colors, .. } => {
            // Big tappable swatches in rows of four.
            let tile = 90.0;
            let gap = 18.0;
            let per_row = 4;
            let total = tile * per_row as f32 + gap * (per_row as f32 - 1.0);
            let start_x = panel_x + (panel_w - total) / 2.0;
            let start_y = panel_y + 150.0;
            for i in 0..colors.len() {
                let col = (i % per_row) as f32;
                let row = (i / per_row) as f32;
                swatches.push(SwatchTile {
                    rect: UiRect {
                        x: start_x + col * (tile + gap),
                        y: start_y + row * (tile + gap),
                        w: tile,
                        h: tile,
                    },
                    index: i,
                });
            }
        }
    }

    let close_btn = UiRect { x: panel_x + panel_w - 116.0, y: panel_y + panel_h - 52.0, w: 100.0, h: 38.0 };

    ShopLayout { panel: UiRect { x: panel_x, y: panel_y, w: panel_w, h: panel_h }, items, answers, swatches, close_btn }
}

/// An input outcome the game acts on.
pub enum ShopInput {
    SelectItem(usize),
    Answer(u32),
    PickColor(usize),
    Close,
}

pub fn handle_click(mx: f32, my: f32, layout: &ShopLayout) -> Option<ShopInput> {
    if layout.close_btn.contains(mx, my) {
        return Some(ShopInput::Close);
    }
    for row in &layout.items {
        if row.rect.contains(mx, my) {
            return Some(ShopInput::SelectItem(row.index));
        }
    }
    for tile in &layout.answers {
        if tile.rect.contains(mx, my) {
            return Some(ShopInput::Answer(tile.value));
        }
    }
    for swatch in &layout.swatches {
        if swatch.rect.contains(mx, my) {
            return Some(ShopInput::PickColor(swatch.index));
        }
    }
    None
}

pub fn handle_key(input: &FrameInput, layout: &ShopLayout) -> Option<ShopInput> {
    if input.pressed(KeyCode::Escape) {
        return Some(ShopInput::Close);
    }
    // Number keys pick an answer tile while buying, or a swatch while
    // choosing an outfit color (the two are never on screen together).
    let keys = [
        KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4,
        KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8,
    ];
    for (i, key) in keys.iter().take(layout.answers.len()).enumerate() {
        if input.pressed(*key) {
            return Some(ShopInput::Answer(layout.answers[i].value));
        }
    }
    for (i, key) in keys.iter().take(layout.swatches.len()).enumerate() {
        if input.pressed(*key) {
            return Some(ShopInput::PickColor(layout.swatches[i].index));
        }
    }
    None
}

// ─── Drawing ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const ROW_BG: Color = Color::new(0.16, 0.18, 0.28, 1.0);
const OWNED_BG: Color = Color::new(0.20, 0.30, 0.22, 1.0);
const POOR_BG: Color = Color::new(0.30, 0.18, 0.18, 1.0);
const TILE_BG: Color = Color::new(0.129, 0.588, 0.953, 1.0);

pub fn draw_shop(
    catalog: &[ShopItem],
    owned: &std::collections::BTreeSet<String>,
    balance: u32,
    view: &ShopView,
    layout: &ShopLayout,
    message: Option<&str>,
    shop: ShopKind,
) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));

    let p = layout.panel;
    draw_rectangle(p.x, p.y, p.w, p.h, DARK_BG);
    draw_rectangle_lines(p.x, p.y, p.w, p.h, 4.0, GOLD);

    let title = shop.title();
    let tw = measure_text(title, None, 30, 1.0).width;
    draw_text(title, p.x + p.w / 2.0 - tw / 2.0, p.y + 38.0, 30.0, GOLD);
    let bal = format!("You have {balance} {}", shop.currency().label());
    draw_text(&bal, p.x + 32.0, p.y + 64.0, 22.0, WHITE);

    match view {
        ShopView::Browsing => {
            for row in &layout.items {
                let item = &catalog[row.index];
                let is_owned = owned.contains(&item.id);
                let affordable = balance >= item.cost;
                let bg = if is_owned { OWNED_BG } else if affordable { ROW_BG } else { POOR_BG };
                let r = row.rect;
                draw_rectangle(r.x, r.y, r.w, r.h, bg);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.25));
                let label = match item.kind {
                    // The trade desk is a standing offer, never "owned".
                    ItemKind::Trade { rate } =>
                        format!("{}  ({rate} pearls = 1 Dum Dum)", item.name),
                    _ if is_owned => format!("{}  (owned)", item.name),
                    _ => item.name.clone(),
                };
                let name_y = if item.blurb.is_empty() { r.y + 38.0 } else { r.y + 28.0 };
                draw_text(&label, r.x + 16.0, name_y, 24.0, WHITE);
                let price = format!("{} {}", item.cost, item.currency.tag());
                let pw = measure_text(&price, None, 24, 1.0).width;
                draw_text(&price, r.x + r.w - pw - 16.0, name_y, 24.0, GOLD);
                // What it actually does, for anything whose name doesn't say.
                if !item.blurb.is_empty() {
                    draw_text(&item.blurb, r.x + 16.0, r.y + 50.0, 18.0,
                        Color::new(1.0, 1.0, 1.0, 0.62));
                }
            }
        }
        ShopView::Buying { item, balance, cost, .. } => {
            let q = format!("{} costs {} Dum Dums.", item.name, cost);
            let q2 = format!("You have {}. How many will you have left?", balance);
            draw_text(&q, p.x + 32.0, p.y + 120.0, 24.0, WHITE);
            draw_text(&q2, p.x + 32.0, p.y + 152.0, 24.0, WHITE);
            for tile in &layout.answers {
                let t = tile.rect;
                draw_rectangle(t.x, t.y, t.w, t.h, TILE_BG);
                draw_rectangle_lines(t.x, t.y, t.w, t.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
                let s = format!("{}", tile.value);
                let size = 36u16;
                let sw2 = measure_text(&s, None, size, 1.0).width;
                draw_text(&s, t.x + t.w / 2.0 - sw2 / 2.0, t.y + t.h / 2.0 + 12.0, size as f32, WHITE);
            }
        }
        ShopView::Trading { quote, .. } => {
            let q = format!("{} pearls make one Dum Dum.", quote.rate);
            let q2 = format!("You have {}. How many Dum Dums is that?", quote.offered);
            draw_text(&q, p.x + 32.0, p.y + 120.0, 24.0, WHITE);
            draw_text(&q2, p.x + 32.0, p.y + 152.0, 24.0, WHITE);
            // Lay the pile out in rows of `rate` — the grouping IS the
            // division, so a kid can count the rows instead of dividing.
            let per_row = quote.rate.max(1) as usize;
            let pearl = Color::new(0.93, 0.96, 0.99, 1.0);
            for i in 0..quote.offered.min(30) as usize {
                let col = (i % per_row) as f32;
                let row = (i / per_row) as f32;
                let cx = p.x + 44.0 + col * 26.0 + row * 4.0;
                let cy = p.y + 196.0 + row * 24.0;
                draw_circle(cx, cy, 8.0, pearl);
                draw_circle(cx - 2.0, cy - 2.0, 3.0, Color::new(1.0, 1.0, 1.0, 0.9));
            }
            for tile in &layout.answers {
                let t = tile.rect;
                draw_rectangle(t.x, t.y, t.w, t.h, TILE_BG);
                draw_rectangle_lines(t.x, t.y, t.w, t.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
                let s = format!("{}", tile.value);
                let size = 36u16;
                let sw2 = measure_text(&s, None, size, 1.0).width;
                draw_text(&s, t.x + t.w / 2.0 - sw2 / 2.0, t.y + t.h / 2.0 + 12.0, size as f32, WHITE);
            }
        }
        ShopView::PickingColor { colors, current } => {
            draw_text("Which color do you want to wear?", p.x + 32.0, p.y + 120.0, 24.0, WHITE);
            for tile in &layout.swatches {
                let t = tile.rect;
                let (_, color) = colors[tile.index];
                draw_rectangle(t.x, t.y, t.w, t.h, color);
                if tile.index == *current {
                    // The color being worn right now gets a thick gold frame.
                    draw_rectangle_lines(t.x - 3.0, t.y - 3.0, t.w + 6.0, t.h + 6.0, 6.0, GOLD);
                } else {
                    draw_rectangle_lines(t.x, t.y, t.w, t.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
                }
            }
        }
    }

    if let Some(msg) = message {
        let mw = measure_text(msg, None, 24, 1.0).width;
        draw_text(msg, p.x + p.w / 2.0 - mw / 2.0, p.y + p.h - 76.0, 24.0, GOLD);
    }

    let c = layout.close_btn;
    draw_rectangle(c.x, c.y, c.w, c.h, Color::new(0.329, 0.431, 0.478, 1.0));
    draw_rectangle_lines(c.x, c.y, c.w, c.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.3));
    let cl = "Done";
    let clw = measure_text(cl, None, 22, 1.0).width;
    draw_text(cl, c.x + c.w / 2.0 - clw / 2.0, c.y + c.h / 2.0 + 8.0, 22.0, WHITE);
}
