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

pub fn draw_interaction_menu(options: &[MenuOption], input: &FrameInput) -> Option<MenuAction> {
    let sw = screen_width();
    let sh = screen_height();

    let btn_w = 200.0;
    let btn_h = 56.0;
    let gap = 12.0;
    let total_w = options.len() as f32 * btn_w + (options.len() as f32 - 1.0) * gap;
    let start_x = sw / 2.0 - total_w / 2.0;
    let y = sh - 220.0;

    // Background strip
    draw_rectangle(start_x - 12.0, y - 10.0, total_w + 24.0, btn_h + 20.0,
        Color::new(0.078, 0.078, 0.157, 0.85));

    let (mx, my) = input.mouse_pos;
    let clicked = input.mouse_clicked;

    for (i, opt) in options.iter().enumerate() {
        let bx = start_x + i as f32 * (btn_w + gap);

        let hover = mx >= bx && mx <= bx + btn_w && my >= y && my <= y + btn_h;
        let color = if hover {
            Color::from_rgba(50, 120, 200, 255)
        } else {
            Color::from_rgba(33, 96, 170, 255)
        };
        draw_rectangle(bx, y, btn_w, btn_h, color);
        draw_rectangle_lines(bx, y, btn_w, btn_h, 1.5, Color::new(1.0, 1.0, 1.0, 0.3));

        let label = format!("[{}] {}", opt.key, opt.label);
        let tw = measure_text(&label, None, 26, 1.0).width;
        draw_text(&label, bx + btn_w / 2.0 - tw / 2.0, y + 37.0, 26.0, WHITE);

        // Click
        if clicked && hover {
            return Some(MenuAction::Select(opt.option_type.clone()));
        }

        // Key press
        let key = match opt.key {
            1 => KeyCode::Key1,
            2 => KeyCode::Key2,
            3 => KeyCode::Key3,
            _ => continue,
        };
        if input.pressed(key) {
            return Some(MenuAction::Select(opt.option_type.clone()));
        }
    }

    // Escape dismisses
    if input.pressed(KeyCode::Escape) {
        return Some(MenuAction::Dismiss);
    }

    None
}
