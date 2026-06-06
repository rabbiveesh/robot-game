//! Shop UI. Pure layout + render; the purchase arithmetic lives in
//! `robot_buddy_domain::economy::shop`.
//!
//! Two views: a catalog of cosmetics (owned ones greyed out), and — once the
//! kid picks something they can afford — the embedded subtraction ("you have
//! 12, it costs 5, how many left?") presented as a few answer tiles. Picking
//! the right remainder completes the purchase; a wrong pick just asks them to
//! recount (never a wrong-answer buzzer).

use macroquad::prelude::*;
use robot_buddy_domain::economy::shop::ShopItem;

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

pub struct ShopLayout {
    pub panel: UiRect,
    pub items: Vec<ItemRow>,
    pub answers: Vec<AnswerTile>,
    pub close_btn: UiRect,
}

/// What the kid is doing in the shop right now.
pub enum ShopView<'a> {
    /// Browsing the catalog.
    Browsing,
    /// Solving the purchase subtraction for `item` (you have `balance`, it costs
    /// `cost`); tap the remainder from `choices`.
    Buying { item: &'a ShopItem, balance: u32, cost: u32, choices: &'a [u32] },
}

pub fn layout(catalog: &[ShopItem], view: &ShopView, screen: (f32, f32)) -> ShopLayout {
    let (sw, sh) = screen;
    let panel_w = (sw - 40.0).min(640.0);
    let panel_h = (sh - 40.0).min(560.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;

    let mut items = Vec::new();
    let mut answers = Vec::new();

    match view {
        ShopView::Browsing => {
            let row_h = 54.0;
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
        ShopView::Buying { choices, .. } => {
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
    }

    let close_btn = UiRect { x: panel_x + panel_w - 116.0, y: panel_y + panel_h - 52.0, w: 100.0, h: 38.0 };

    ShopLayout { panel: UiRect { x: panel_x, y: panel_y, w: panel_w, h: panel_h }, items, answers, close_btn }
}

/// An input outcome the game acts on.
pub enum ShopInput {
    SelectItem(usize),
    Answer(u32),
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
    None
}

pub fn handle_key(input: &FrameInput, layout: &ShopLayout) -> Option<ShopInput> {
    if input.pressed(KeyCode::Escape) {
        return Some(ShopInput::Close);
    }
    // Number keys pick an answer tile while buying.
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
    for (i, key) in keys.iter().take(layout.answers.len()).enumerate() {
        if input.pressed(*key) {
            return Some(ShopInput::Answer(layout.answers[i].value));
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
    owned: &std::collections::HashSet<String>,
    balance: u32,
    view: &ShopView,
    layout: &ShopLayout,
    message: Option<&str>,
) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));

    let p = layout.panel;
    draw_rectangle(p.x, p.y, p.w, p.h, DARK_BG);
    draw_rectangle_lines(p.x, p.y, p.w, p.h, 4.0, GOLD);

    let title = "Bolt's Shop";
    let tw = measure_text(title, None, 30, 1.0).width;
    draw_text(title, p.x + p.w / 2.0 - tw / 2.0, p.y + 38.0, 30.0, GOLD);
    let bal = format!("You have {balance} Dum Dums");
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
                let label = if is_owned {
                    format!("{}  (owned)", item.name)
                } else {
                    format!("{}", item.name)
                };
                draw_text(&label, r.x + 16.0, r.y + 34.0, 24.0, WHITE);
                let price = format!("{} DD", item.cost);
                let pw = measure_text(&price, None, 24, 1.0).width;
                draw_text(&price, r.x + r.w - pw - 16.0, r.y + 34.0, 24.0, GOLD);
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
