use macroquad::prelude::*;
use crate::settings::{self, TextSpeed};

pub enum SettingsResult {
    Close,
    BackToTitle,
}

const PANEL_BG: Color = Color::new(0.086, 0.129, 0.243, 1.0);      // #16213E
const ACCENT: Color = Color::new(0.0, 0.902, 0.463, 1.0);          // #00E676
const LABEL_GRAY: Color = Color::new(0.690, 0.745, 0.773, 1.0);    // #B0BEC5
const BTN_OFF: Color = Color::new(0.216, 0.278, 0.310, 1.0);       // #37474F
const BTN_TXT_OFF: Color = Color::new(0.565, 0.643, 0.682, 1.0);   // #90A4AE
const HINT_GRAY: Color = Color::new(0.329, 0.431, 0.478, 1.0);     // #546E7A

struct Row {
    rect: (f32, f32, f32, f32),
    action: RowAction,
}

#[derive(Clone, Copy)]
enum RowAction {
    ToggleTts,
    SetSpeed(TextSpeed),
    BackToTitle,
    Done,
}

fn round_rect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    draw_rectangle(x + r, y, w - 2.0 * r, h, color);
    draw_rectangle(x, y + r, w, h - 2.0 * r, color);
    draw_circle(x + r, y + r, r, color);
    draw_circle(x + w - r, y + r, r, color);
    draw_circle(x + r, y + h - r, r, color);
    draw_circle(x + w - r, y + h - r, r, color);
}

/// Render the overlay and capture click regions for hit testing.
fn layout() -> (f32, f32, f32, f32, Vec<Row>) {
    let sw = screen_width();
    let sh = screen_height();
    let panel_w = (sw - 80.0).min(480.0);
    let panel_h = 470.0;
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;
    let mut rows = Vec::new();

    // TTS toggle — full-width pill
    let pad = 28.0;
    let ts_y = panel_y + 80.0;
    let ts_h = 56.0;
    rows.push(Row {
        rect: (panel_x + pad, ts_y, panel_w - pad * 2.0, ts_h),
        action: RowAction::ToggleTts,
    });

    // Text speed — three side-by-side buttons
    let speed_y = ts_y + ts_h + 54.0;
    let speed_h = 48.0;
    let speed_gap = 10.0;
    let speed_w = (panel_w - pad * 2.0 - speed_gap * 2.0) / 3.0;
    for (i, ts) in [TextSpeed::Slow, TextSpeed::Normal, TextSpeed::Fast].iter().enumerate() {
        let x = panel_x + pad + i as f32 * (speed_w + speed_gap);
        rows.push(Row {
            rect: (x, speed_y, speed_w, speed_h),
            action: RowAction::SetSpeed(*ts),
        });
    }

    // Back to title — full-width
    let btt_y = speed_y + speed_h + 54.0;
    let btt_h = 48.0;
    rows.push(Row {
        rect: (panel_x + pad, btt_y, panel_w - pad * 2.0, btt_h),
        action: RowAction::BackToTitle,
    });

    // Done — full-width, bottom
    let done_y = panel_y + panel_h - 72.0;
    let done_h = 52.0;
    rows.push(Row {
        rect: (panel_x + pad, done_y, panel_w - pad * 2.0, done_h),
        action: RowAction::Done,
    });

    (panel_x, panel_y, panel_w, panel_h, rows)
}

pub fn draw() {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.75));

    let (panel_x, panel_y, panel_w, panel_h, rows) = layout();

    round_rect(panel_x, panel_y, panel_w, panel_h, 16.0, PANEL_BG);
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 3.0, ACCENT);

    // Title
    let title = "Settings";
    let tw = measure_text(title, None, 36, 1.0).width;
    draw_text(title, panel_x + panel_w / 2.0 - tw / 2.0, panel_y + 48.0, 36.0, ACCENT);

    for row in &rows {
        let (x, y, w, h) = row.rect;
        match row.action {
            RowAction::ToggleTts => {
                let on = settings::tts_enabled();
                let bg = if on { ACCENT } else { BTN_OFF };
                let fg = if on { Color::from_rgba(26, 26, 46, 255) } else { BTN_TXT_OFF };
                round_rect(x, y, w, h, 8.0, bg);
                let label = if on { "Read dialogue aloud: ON" } else { "Read dialogue aloud: OFF" };
                let lw = measure_text(label, None, 22, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 8.0, 22.0, fg);
            }
            RowAction::SetSpeed(ts) => {
                let active = settings::text_speed() == ts;
                let bg = if active { ACCENT } else { BTN_OFF };
                let fg = if active { Color::from_rgba(26, 26, 46, 255) } else { BTN_TXT_OFF };
                round_rect(x, y, w, h, 8.0, bg);
                let label = ts.label();
                let lw = measure_text(label, None, 22, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 8.0, 22.0, fg);
            }
            RowAction::BackToTitle => {
                round_rect(x, y, w, h, 8.0, BTN_OFF);
                let label = "Back to title screen";
                let lw = measure_text(label, None, 22, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 8.0, 22.0, BTN_TXT_OFF);
            }
            RowAction::Done => {
                round_rect(x, y, w, h, 10.0, ACCENT);
                let label = "Done";
                let lw = measure_text(label, None, 26, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 9.0, 26.0, Color::from_rgba(26, 26, 46, 255));
            }
        }
    }

    // Section labels drawn over layout rects
    draw_text("Text speed", panel_x + 28.0, panel_y + 80.0 + 56.0 + 34.0, 18.0, LABEL_GRAY);

    // Hint
    let hint = "Press T or ESC to close";
    let hw = measure_text(hint, None, 18, 1.0).width;
    draw_text(hint, panel_x + panel_w / 2.0 - hw / 2.0, panel_y + panel_h - 12.0, 18.0, HINT_GRAY);
}

/// Handle input; returns a result if the overlay should close.
pub fn handle_input() -> Option<SettingsResult> {
    if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::T) {
        return Some(SettingsResult::Close);
    }

    if !is_mouse_button_pressed(MouseButton::Left) {
        return None;
    }
    let (mx, my) = mouse_position();
    let (_, _, _, _, rows) = layout();
    for row in rows {
        let (x, y, w, h) = row.rect;
        if mx >= x && mx <= x + w && my >= y && my <= y + h {
            match row.action {
                RowAction::ToggleTts => {
                    settings::toggle_tts();
                    if !settings::tts_enabled() {
                        crate::audio::tts::cancel();
                    }
                }
                RowAction::SetSpeed(ts) => settings::set_text_speed(ts),
                RowAction::BackToTitle => return Some(SettingsResult::BackToTitle),
                RowAction::Done => return Some(SettingsResult::Close),
            }
            return None;
        }
    }
    None
}
