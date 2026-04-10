use macroquad::prelude::*;
use crate::save::{SaveSlots, Gender};

// ─── TITLE SCREEN ───────────────────────────────────────

pub enum TitleAction {
    NewGame(usize),      // slot index
    LoadGame(usize),     // slot index
    DeleteSlot(usize),   // slot index
}

pub fn draw_title_screen(slots: &SaveSlots, time: f32) -> Option<TitleAction> {
    let sw = screen_width();
    let sh = screen_height();

    // Background
    clear_background(Color::from_rgba(26, 26, 46, 255));

    // Stars
    for i in 0..50 {
        let seed = i as f32 * 137.5;
        let sx = (seed * 7.3).sin() * 0.5 + 0.5;
        let sy = (seed * 13.1).cos() * 0.5 + 0.5;
        let alpha = ((time * 1.5 + seed).sin() * 0.5 + 0.5) as f32;
        draw_circle(sx * sw, sy * sh * 0.4, 1.5, Color::new(1.0, 1.0, 1.0, alpha * 0.6));
    }

    // Title
    let title = "ROBOT BUDDY";
    let tw = measure_text(title, None, 48, 1.0).width;
    draw_text(title, sw / 2.0 - tw / 2.0, 80.0, 48.0, Color::from_rgba(0, 230, 118, 255));

    let sub = "ADVENTURE";
    let subw = measure_text(sub, None, 36, 1.0).width;
    draw_text(sub, sw / 2.0 - subw / 2.0, 120.0, 36.0, Color::from_rgba(255, 213, 79, 255));

    let tagline = "A Math RPG";
    let tw2 = measure_text(tagline, None, 18, 1.0).width;
    draw_text(tagline, sw / 2.0 - tw2 / 2.0, 150.0, 18.0, Color::from_rgba(100, 149, 237, 255));

    // Save slots header
    let header = "SAVE FILES";
    let hw = measure_text(header, None, 20, 1.0).width;
    draw_text(header, sw / 2.0 - hw / 2.0, 210.0, 20.0, Color::from_rgba(150, 150, 150, 255));

    // Draw 3 save slots
    let slot_w = 400.0;
    let slot_h = 70.0;
    let slot_x = sw / 2.0 - slot_w / 2.0;
    let slot_start_y = 230.0;
    let slot_gap = 10.0;

    let mut action: Option<TitleAction> = None;
    let (mx, my) = mouse_position();
    let clicked = is_mouse_button_pressed(MouseButton::Left);

    for (i, slot) in slots.iter().enumerate() {
        let sy = slot_start_y + i as f32 * (slot_h + slot_gap);

        // Slot background
        let hover = mx >= slot_x && mx <= slot_x + slot_w && my >= sy && my <= sy + slot_h;
        let bg = if hover {
            Color::from_rgba(35, 35, 60, 255)
        } else {
            Color::from_rgba(25, 25, 45, 255)
        };
        draw_rectangle(slot_x, sy, slot_w, slot_h, bg);
        draw_rectangle_lines(slot_x, sy, slot_w, slot_h, 2.0,
            Color::from_rgba(80, 80, 120, 255));

        // Slot label
        let label = format!("FILE {}", i + 1);
        draw_text(&label, slot_x + 12.0, sy + 25.0, 16.0, Color::from_rgba(120, 120, 150, 255));

        if let Some(save) = slot {
            // Filled slot — show name, playtime
            draw_text(&save.name, slot_x + 12.0, sy + 50.0, 22.0, WHITE);
            let info = format!("{}  |  {}", save.play_time_display(), save.date_display());
            let iw = measure_text(&info, None, 14, 1.0).width;
            draw_text(&info, slot_x + slot_w - iw - 100.0, sy + 50.0, 14.0,
                Color::from_rgba(150, 150, 170, 255));

            // LOAD button
            let btn_w = 70.0;
            let btn_h = 32.0;
            let btn_x = slot_x + slot_w - btn_w - 8.0;
            let btn_y = sy + (slot_h - btn_h) / 2.0;
            let btn_hover = mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
            draw_rectangle(btn_x, btn_y, btn_w, btn_h, if btn_hover {
                Color::from_rgba(0, 200, 100, 255)
            } else {
                Color::from_rgba(0, 160, 80, 255)
            });
            let ltw = measure_text("LOAD", None, 16, 1.0).width;
            draw_text("LOAD", btn_x + btn_w / 2.0 - ltw / 2.0, btn_y + 22.0, 16.0, WHITE);

            if clicked && btn_hover {
                action = Some(TitleAction::LoadGame(i));
            }

            // Delete (X) button
            let del_x = btn_x - 30.0;
            let del_y = btn_y + 4.0;
            let del_sz = 24.0;
            let del_hover = mx >= del_x && mx <= del_x + del_sz && my >= del_y && my <= del_y + del_sz;
            draw_text("X", del_x + 6.0, del_y + 18.0, 18.0, if del_hover {
                Color::from_rgba(244, 67, 54, 255)
            } else {
                Color::from_rgba(120, 80, 80, 255)
            });

            if clicked && del_hover {
                action = Some(TitleAction::DeleteSlot(i));
            }
        } else {
            // Empty slot
            draw_text("- empty -", slot_x + 12.0, sy + 50.0, 18.0,
                Color::from_rgba(80, 80, 100, 255));

            // NEW button
            let btn_w = 70.0;
            let btn_h = 32.0;
            let btn_x = slot_x + slot_w - btn_w - 8.0;
            let btn_y = sy + (slot_h - btn_h) / 2.0;
            let btn_hover = mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
            draw_rectangle(btn_x, btn_y, btn_w, btn_h, if btn_hover {
                Color::from_rgba(33, 150, 243, 255)
            } else {
                Color::from_rgba(25, 118, 200, 255)
            });
            let ntw = measure_text("NEW", None, 16, 1.0).width;
            draw_text("NEW", btn_x + btn_w / 2.0 - ntw / 2.0, btn_y + 22.0, 16.0, WHITE);

            if clicked && btn_hover {
                action = Some(TitleAction::NewGame(i));
            }
        }
    }

    // Keyboard shortcuts
    for (i, key) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3].iter().enumerate() {
        if is_key_pressed(*key) {
            if slots[i].is_some() {
                action = Some(TitleAction::LoadGame(i));
            } else {
                action = Some(TitleAction::NewGame(i));
            }
        }
    }

    // Controls hint
    let hint = "1/2/3 to select slot  |  Arrow keys Move  |  SPACE Talk";
    let hint_w = measure_text(hint, None, 14, 1.0).width;
    draw_text(hint, sw / 2.0 - hint_w / 2.0, sh - 30.0, 14.0,
        Color::from_rgba(100, 100, 120, 255));

    action
}

// ─── NEW GAME SCREEN ────────────────────────────────────

pub struct NewGameForm {
    pub name: String,
    pub gender: Gender,
    pub slot: usize,
    cursor_blink: f32,
}

pub enum NewGameAction {
    Start,
    Back,
}

impl NewGameForm {
    pub fn new(slot: usize) -> Self {
        NewGameForm {
            name: String::new(),
            gender: Gender::Boy,
            slot,
            cursor_blink: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.cursor_blink += dt;

        // Text input
        if let Some(ch) = get_char_pressed() {
            if self.name.len() < 20 && ch.is_alphanumeric() || ch == ' ' || ch == '-' {
                if self.name.len() < 20 {
                    self.name.push(ch);
                }
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.name.pop();
        }
    }

    pub fn draw(&self) -> Option<NewGameAction> {
        let sw = screen_width();
        let sh = screen_height();

        clear_background(Color::from_rgba(26, 26, 46, 255));

        // Header
        let header = "NEW ADVENTURE";
        let hw = measure_text(header, None, 36, 1.0).width;
        draw_text(header, sw / 2.0 - hw / 2.0, 80.0, 36.0, Color::from_rgba(0, 230, 118, 255));

        let slot_label = format!("File {}", self.slot + 1);
        let slw = measure_text(&slot_label, None, 18, 1.0).width;
        draw_text(&slot_label, sw / 2.0 - slw / 2.0, 110.0, 18.0,
            Color::from_rgba(150, 150, 170, 255));

        // Name input
        let label = "What's your name, hero?";
        let lw = measure_text(label, None, 22, 1.0).width;
        draw_text(label, sw / 2.0 - lw / 2.0, 180.0, 22.0, WHITE);

        let input_w = 300.0;
        let input_h = 40.0;
        let input_x = sw / 2.0 - input_w / 2.0;
        let input_y = 200.0;
        draw_rectangle(input_x, input_y, input_w, input_h, Color::from_rgba(15, 15, 30, 255));
        draw_rectangle_lines(input_x, input_y, input_w, input_h, 2.0,
            Color::from_rgba(0, 230, 118, 255));

        let display_name = if self.name.is_empty() {
            "Type your name..."
        } else {
            &self.name
        };
        let name_color = if self.name.is_empty() {
            Color::from_rgba(80, 80, 100, 255)
        } else {
            WHITE
        };
        draw_text(display_name, input_x + 12.0, input_y + 28.0, 22.0, name_color);

        // Cursor blink
        if (self.cursor_blink * 2.0).sin() > 0.0 {
            let cursor_x = input_x + 12.0 + measure_text(&self.name, None, 22, 1.0).width + 2.0;
            draw_line(cursor_x, input_y + 8.0, cursor_x, input_y + 32.0, 2.0,
                Color::from_rgba(0, 230, 118, 255));
        }

        // Gender picker
        let gender_label = "Pick your character:";
        let glw = measure_text(gender_label, None, 22, 1.0).width;
        draw_text(gender_label, sw / 2.0 - glw / 2.0, 290.0, 22.0, WHITE);

        let btn_w = 120.0;
        let btn_h = 50.0;
        let gap = 20.0;
        let boy_x = sw / 2.0 - btn_w - gap / 2.0;
        let girl_x = sw / 2.0 + gap / 2.0;
        let btn_y = 310.0;

        let (mx, my) = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let mut action: Option<NewGameAction> = None;

        // Boy button
        let boy_selected = self.gender == Gender::Boy;
        let boy_hover = mx >= boy_x && mx <= boy_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
        draw_rectangle(boy_x, btn_y, btn_w, btn_h, if boy_selected {
            Color::from_rgba(33, 150, 243, 255)
        } else {
            Color::from_rgba(40, 40, 70, 255)
        });
        draw_rectangle_lines(boy_x, btn_y, btn_w, btn_h, 2.0, if boy_selected || boy_hover {
            Color::from_rgba(33, 150, 243, 255)
        } else {
            Color::from_rgba(80, 80, 120, 255)
        });
        let btw = measure_text("Boy", None, 24, 1.0).width;
        draw_text("Boy", boy_x + btn_w / 2.0 - btw / 2.0, btn_y + 33.0, 24.0, WHITE);

        // Girl button
        let girl_selected = self.gender == Gender::Girl;
        let girl_hover = mx >= girl_x && mx <= girl_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
        draw_rectangle(girl_x, btn_y, btn_w, btn_h, if girl_selected {
            Color::from_rgba(233, 30, 99, 255)
        } else {
            Color::from_rgba(40, 40, 70, 255)
        });
        draw_rectangle_lines(girl_x, btn_y, btn_w, btn_h, 2.0, if girl_selected || girl_hover {
            Color::from_rgba(233, 30, 99, 255)
        } else {
            Color::from_rgba(80, 80, 120, 255)
        });
        let gtw = measure_text("Girl", None, 24, 1.0).width;
        draw_text("Girl", girl_x + btn_w / 2.0 - gtw / 2.0, btn_y + 33.0, 24.0, WHITE);

        // Start button
        let start_w = 260.0;
        let start_h = 50.0;
        let start_x = sw / 2.0 - start_w / 2.0;
        let start_y = 420.0;
        let start_hover = mx >= start_x && mx <= start_x + start_w
            && my >= start_y && my <= start_y + start_h;
        let can_start = !self.name.is_empty();

        let start_color = if !can_start {
            Color::from_rgba(60, 60, 80, 255)
        } else if start_hover {
            Color::from_rgba(0, 200, 100, 255)
        } else {
            Color::from_rgba(0, 160, 80, 255)
        };
        draw_rectangle(start_x, start_y, start_w, start_h, start_color);
        let stw = measure_text("START ADVENTURE!", None, 24, 1.0).width;
        draw_text("START ADVENTURE!", start_x + start_w / 2.0 - stw / 2.0, start_y + 33.0,
            24.0, if can_start { WHITE } else { Color::from_rgba(80, 80, 100, 255) });

        if can_start && clicked && start_hover {
            action = Some(NewGameAction::Start);
        }
        if can_start && is_key_pressed(KeyCode::Enter) {
            action = Some(NewGameAction::Start);
        }

        // Back button
        let back = "ESC to go back";
        let bw = measure_text(back, None, 14, 1.0).width;
        draw_text(back, sw / 2.0 - bw / 2.0, sh - 30.0, 14.0,
            Color::from_rgba(100, 100, 120, 255));

        if is_key_pressed(KeyCode::Escape) {
            action = Some(NewGameAction::Back);
        }

        // Handle gender clicks (return none, mutation happens via returned action)
        // We can't mutate self here, so we handle it in the caller
        action
    }

    pub fn handle_gender_click(&mut self) {
        let sw = screen_width();
        let btn_w = 120.0;
        let gap = 20.0;
        let boy_x = sw / 2.0 - btn_w - gap / 2.0;
        let girl_x = sw / 2.0 + gap / 2.0;
        let btn_y = 310.0;
        let btn_h = 50.0;
        let (mx, my) = mouse_position();

        if is_mouse_button_pressed(MouseButton::Left) {
            if mx >= boy_x && mx <= boy_x + btn_w && my >= btn_y && my <= btn_y + btn_h {
                self.gender = Gender::Boy;
            }
            if mx >= girl_x && mx <= girl_x + btn_w && my >= btn_y && my <= btn_y + btn_h {
                self.gender = Gender::Girl;
            }
        }

        // Tab to toggle
        if is_key_pressed(KeyCode::Tab) {
            self.gender = match self.gender {
                Gender::Boy => Gender::Girl,
                Gender::Girl => Gender::Boy,
            };
        }
    }
}
