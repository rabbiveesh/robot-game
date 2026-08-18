//! "Give Swag" picker. Pure layout + render; who-wears-what lives in
//! `robot_buddy_domain::economy::wardrobe`.
//!
//! One view: the pieces the kid is wearing, as big tappable rows, with the
//! buddy they'd go to named at the top. Tap one and it's theirs — no math gate
//! here, the arithmetic already happened at Bolt's counter.

use crate::prelude::*;
use robot_buddy_domain::economy::shop::ShopItem;

use crate::input::FrameInput;
use crate::ui::shop::UiRect;

pub struct ItemRow {
    pub rect: UiRect,
    pub index: usize,
}

pub struct SwagLayout {
    pub panel: UiRect,
    pub items: Vec<ItemRow>,
    pub close_btn: UiRect,
    /// Where to draw the recipient so the kid sees the outfit land on them.
    pub preview: (f32, f32),
}

pub fn layout(items: &[ShopItem], screen: (f32, f32)) -> SwagLayout {
    let (sw, sh) = screen;
    let panel_w = (sw - 40.0).min(560.0);
    let panel_h = (sh - 40.0).min(480.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;

    let row_h = 54.0;
    let gap = 10.0;
    let row_w = panel_w - 64.0;
    let start_x = panel_x + 32.0;
    let start_y = panel_y + 108.0;
    let rows = items.iter().enumerate().map(|(i, _)| ItemRow {
        rect: UiRect { x: start_x, y: start_y + i as f32 * (row_h + gap), w: row_w, h: row_h },
        index: i,
    }).collect();

    SwagLayout {
        panel: UiRect { x: panel_x, y: panel_y, w: panel_w, h: panel_h },
        items: rows,
        close_btn: UiRect { x: panel_x + panel_w - 116.0, y: panel_y + panel_h - 52.0, w: 100.0, h: 38.0 },
        preview: (panel_x + panel_w - 76.0, panel_y + 30.0),
    }
}

pub enum SwagInput {
    Give(usize),
    Close,
}

pub fn handle_click(mx: f32, my: f32, layout: &SwagLayout) -> Option<SwagInput> {
    if layout.close_btn.contains(mx, my) {
        return Some(SwagInput::Close);
    }
    layout.items.iter()
        .find(|row| row.rect.contains(mx, my))
        .map(|row| SwagInput::Give(row.index))
}

pub fn handle_key(input: &FrameInput, layout: &SwagLayout) -> Option<SwagInput> {
    if input.pressed(KeyCode::Escape) {
        return Some(SwagInput::Close);
    }
    let keys = [
        KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4,
        KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8,
    ];
    keys.iter().take(layout.items.len()).enumerate()
        .find(|(_, key)| input.pressed(**key))
        .map(|(i, _)| SwagInput::Give(i))
}

// ─── Drawing ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const ROW_BG: Color = Color::new(0.16, 0.18, 0.28, 1.0);

/// Draw the picker. `items` are the pieces the kid can hand over (already
/// filtered to what they're wearing); `taken` marks the ones the recipient
/// already has one of, which stay on screen but read as unavailable.
pub fn draw(
    recipient: &str,
    items: &[ShopItem],
    taken: &std::collections::BTreeSet<String>,
    layout: &SwagLayout,
    message: Option<&str>,
) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));

    let p = layout.panel;
    draw_rectangle(p.x, p.y, p.w, p.h, DARK_BG);
    draw_rectangle_lines(p.x, p.y, p.w, p.h, 4.0, GOLD);

    let title = format!("Dress up {recipient}!");
    let tw = measure_text(&title, None, 30, 1.0).width;
    draw_text(&title, p.x + p.w / 2.0 - tw / 2.0, p.y + 44.0, 30.0, GOLD);
    draw_text("Tap something you're wearing to give it away.",
        p.x + 32.0, p.y + 78.0, 22.0, WHITE);

    for row in &layout.items {
        let item = &items[row.index];
        let already = taken.contains(&item.id);
        let r = row.rect;
        draw_rectangle(r.x, r.y, r.w, r.h, ROW_BG);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.25));
        let label = format!("[{}] {}", row.index + 1, item.name);
        let color = if already { Color::new(1.0, 1.0, 1.0, 0.45) } else { WHITE };
        draw_text(&label, r.x + 16.0, r.y + 34.0, 24.0, color);
        if already {
            let note = format!("{recipient} has one");
            let nw = measure_text(&note, None, 20, 1.0).width;
            draw_text(&note, r.x + r.w - nw - 16.0, r.y + 34.0, 20.0, Color::new(1.0, 1.0, 1.0, 0.45));
        }
    }

    if items.is_empty() {
        draw_text("You're not wearing any swag right now!",
            p.x + 32.0, p.y + 140.0, 24.0, GOLD);
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
