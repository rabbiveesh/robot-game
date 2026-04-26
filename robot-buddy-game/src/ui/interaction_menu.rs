//! Interaction menu (Talk / Give / etc.). Split into pure layout + input handling
//! and a separate draw step so the game loop can run without a macroquad context.

use macroquad::prelude::*;
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

pub fn layout(options: &[MenuOption], screen: (f32, f32)) -> Layout {
    let (sw, sh) = screen;
    let btn_w = 200.0;
    let btn_h = 56.0;
    let gap = 12.0;
    let count = options.len() as f32;
    let total_w = count * btn_w + (count - 1.0).max(0.0) * gap;
    let start_x = sw / 2.0 - total_w / 2.0;
    let y = sh - 220.0;

    let strip = (start_x - 12.0, y - 10.0, total_w + 24.0, btn_h + 20.0);

    let buttons = options.iter().enumerate().map(|(i, opt)| Button {
        rect: (start_x + i as f32 * (btn_w + gap), y, btn_w, btn_h),
        option_type: opt.option_type.clone(),
        label: opt.label.clone(),
        key: opt.key,
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
        let tw = measure_text(&label, None, 26, 1.0).width;
        draw_text(&label, bx + bw / 2.0 - tw / 2.0, by + 37.0, 26.0, WHITE);
    }
}
