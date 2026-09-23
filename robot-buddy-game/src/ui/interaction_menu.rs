//! Interaction menu (Talk / Give / etc.). Split into pure layout + input handling
//! and a separate draw step so the game loop can run without a macroquad context.

use crate::prelude::*;
use crate::input::FrameInput;

pub struct MenuOption {
    pub option_type: String,
    pub label: String,
    pub key: usize, // 1-based
}

pub enum MenuAction {
    Select(String), // option_type
    Dismiss,
}

pub struct Layout {
    pub strip: (f32, f32, f32, f32), // background strip rect
    pub buttons: Vec<Button>,
}

pub struct Button {
    pub rect: (f32, f32, f32, f32),
    pub option_type: String,
    pub label: String,
    pub key: usize,
}

/// Buttons never shrink below this — smaller than a small thumb is useless.
const MIN_BTN_W: f32 = 110.0;

pub fn layout(options: &[MenuOption], screen: (f32, f32)) -> Layout {
    let (sw, sh) = screen;
    let btn_h = 56.0;
    let gap = 12.0;
    let count = options.len().max(1);
    let avail = sw - 40.0;

    // Shrink buttons to fit the screen width once an NPC stacks several options
    // (Talk + Give + Swag + four puzzle types). Past the point where they'd be
    // thumb-hostile, wrap onto extra rows instead of running off-screen.
    let mut rows = 1usize;
    while rows < 3 {
        let per_row = count.div_ceil(rows);
        let needed = per_row as f32 * MIN_BTN_W + (per_row as f32 - 1.0) * gap;
        if needed <= avail { break; }
        rows += 1;
    }
    let per_row = count.div_ceil(rows);
    let rows_used = count.div_ceil(per_row); // a wrap may leave the last row short

    let btn_w = ((avail - (per_row as f32 - 1.0) * gap) / per_row as f32)
        .min(200.0).max(MIN_BTN_W);
    let row_w = per_row as f32 * btn_w + (per_row as f32 - 1.0) * gap;
    let start_x = sw / 2.0 - row_w / 2.0;
    let block_h = rows_used as f32 * btn_h + (rows_used as f32 - 1.0) * gap;
    let top_y = sh - 220.0 - (block_h - btn_h);

    let strip = (start_x - 12.0, top_y - 10.0, row_w + 24.0, block_h + 20.0);

    let buttons = options.iter().enumerate().map(|(i, opt)| {
        let row = i / per_row;
        let col = i % per_row;
        // Center a short final row under the full ones.
        let in_row = if row == rows_used - 1 { count - row * per_row } else { per_row };
        let row_offset = (per_row - in_row) as f32 * (btn_w + gap) / 2.0;
        Button {
            rect: (
                start_x + row_offset + col as f32 * (btn_w + gap),
                top_y + row as f32 * (btn_h + gap),
                btn_w,
                btn_h,
            ),
            option_type: opt.option_type.clone(),
            label: opt.label.clone(),
            key: opt.key,
        }
    }).collect();

    Layout { strip, buttons }
}

pub fn handle_input(layout: &Layout, input: &FrameInput) -> Option<MenuAction> {
    let (mx, my) = input.mouse_pos;
    for btn in &layout.buttons {
        let (bx, by, bw, bh) = btn.rect;
        let hover = mx >= bx && mx <= bx + bw && my >= by && my <= by + bh;
        if input.mouse_clicked && hover {
            return Some(MenuAction::Select(btn.option_type.clone()));
        }
        let kc = match btn.key {
            1 => Some(KeyCode::Key1),
            2 => Some(KeyCode::Key2),
            3 => Some(KeyCode::Key3),
            4 => Some(KeyCode::Key4),
            5 => Some(KeyCode::Key5),
            6 => Some(KeyCode::Key6),
            7 => Some(KeyCode::Key7),
            8 => Some(KeyCode::Key8),
            9 => Some(KeyCode::Key9),
            _ => None,
        };
        if let Some(kc) = kc {
            if input.pressed(kc) {
                return Some(MenuAction::Select(btn.option_type.clone()));
            }
        }
    }
    if input.pressed(KeyCode::Escape) {
        return Some(MenuAction::Dismiss);
    }
    None
}

pub fn draw(layout: &Layout, mouse_pos: (f32, f32)) {
    let (sx, sy, sw, sh) = layout.strip;
    draw_rectangle(sx, sy, sw, sh, Color::new(0.078, 0.078, 0.157, 0.85));

    // One font size for the whole strip: the largest that fits EVERY label
    // inside its button with padding. Buttons shrink when a puzzler stacks
    // six options, so a fixed size bleeds out of the box ("Spot the
    // Pattern" et al.); a shared size keeps the row visually uniform.
    let pad = 10.0;
    let mut font: u16 = 26;
    for btn in &layout.buttons {
        let label = format!("[{}] {}", btn.key, btn.label);
        while font > 12 && measure_text(&label, None, font, 1.0).width > btn.rect.2 - pad {
            font -= 1;
        }
    }

    let (mx, my) = mouse_pos;
    for btn in &layout.buttons {
        let (bx, by, bw, bh) = btn.rect;
        let hover = mx >= bx && mx <= bx + bw && my >= by && my <= by + bh;
        let color = if hover {
            Color::from_rgba(50, 120, 200, 255)
        } else {
            Color::from_rgba(33, 96, 170, 255)
        };
        draw_rectangle(bx, by, bw, bh, color);
        draw_rectangle_lines(bx, by, bw, bh, 1.5, Color::new(1.0, 1.0, 1.0, 0.3));

        let label = format!("[{}] {}", btn.key, btn.label);
        let m = measure_text(&label, None, font, 1.0);
        draw_text(
            &label,
            bx + bw / 2.0 - m.width / 2.0,
            by + bh / 2.0 + m.height / 2.0,
            font as f32,
            WHITE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(n: usize) -> Vec<MenuOption> {
        (0..n).map(|i| MenuOption {
            option_type: format!("opt{i}"),
            label: "Balance the Scale".into(), // the longest real label
            key: i + 1,
        }).collect()
    }

    /// A puzzler who also takes gifts and swag stacks seven options. However
    /// many there are, every button has to stay on screen and stay big enough
    /// to hit — that's what the row wrapping is for.
    #[test]
    fn every_button_stays_on_screen_and_thumb_sized() {
        for count in 1..=7 {
            for screen in [(480.0, 800.0), (800.0, 600.0), (1280.0, 720.0)] {
                let l = layout(&options(count), screen);
                assert_eq!(l.buttons.len(), count);
                for b in &l.buttons {
                    let (x, y, w, h) = b.rect;
                    assert!(x >= 0.0 && x + w <= screen.0,
                        "{count} options at {screen:?}: button runs off screen ({x}..{})", x + w);
                    assert!(w >= MIN_BTN_W, "{count} options at {screen:?}: button too small ({w})");
                    assert!(y >= 0.0 && y + h <= screen.1,
                        "{count} options at {screen:?}: button off the bottom");
                }
            }
        }
    }

    /// Buttons must never overlap — a tap has to mean one thing.
    #[test]
    fn buttons_never_overlap() {
        let l = layout(&options(7), (800.0, 600.0));
        for (i, a) in l.buttons.iter().enumerate() {
            for b in l.buttons.iter().skip(i + 1) {
                let (ax, ay, aw, ah) = a.rect;
                let (bx, by, bw, bh) = b.rect;
                let overlaps = ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah;
                assert!(!overlaps, "buttons {i} and its neighbour overlap: {:?} vs {:?}", a.rect, b.rect);
            }
        }
    }
}
