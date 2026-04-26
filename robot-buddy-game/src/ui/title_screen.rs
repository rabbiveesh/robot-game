use macroquad::prelude::*;
use crate::save::{SaveSlots, Gender};
use crate::input::FrameInput;

// ─── TITLE SCREEN ───────────────────────────────────────

pub enum TitleAction {
    NewGame(usize),      // slot index
    LoadGame(usize),     // slot index
    DeleteSlot(usize),   // slot index
}

pub struct TitleLayout {
    pub screen: (f32, f32),
    pub slots: Vec<SlotLayout>,
}

pub struct SlotLayout {
    pub idx: usize,
    pub rect: (f32, f32, f32, f32),
    pub primary_btn: (f32, f32, f32, f32), // LOAD or NEW
    pub primary_action: TitleActionKind,
    pub delete_btn: Option<(f32, f32, f32, f32)>, // X button (only for filled slots)
}

#[derive(Clone, Copy)]
pub enum TitleActionKind { NewGame, LoadGame }

pub fn layout_title(slots: &SaveSlots, screen: (f32, f32)) -> TitleLayout {
    let (sw, _) = screen;
    let slot_w = 400.0;
    let slot_h = 70.0;
    let slot_x = sw / 2.0 - slot_w / 2.0;
    let slot_start_y = 230.0;
    let slot_gap = 10.0;

    let slots_layout = slots.iter().enumerate().map(|(i, slot)| {
        let sy = slot_start_y + i as f32 * (slot_h + slot_gap);

        let btn_w = 70.0;
        let btn_h = 32.0;
        let btn_x = slot_x + slot_w - btn_w - 8.0;
        let btn_y = sy + (slot_h - btn_h) / 2.0;

        let filled = slot.is_some();
        let primary_action = if filled { TitleActionKind::LoadGame } else { TitleActionKind::NewGame };

        let delete_btn = if filled {
            let del_x = btn_x - 30.0;
            let del_y = btn_y + 4.0;
            let del_sz = 24.0;
            Some((del_x, del_y, del_sz, del_sz))
        } else {
            None
        };

        SlotLayout {
            idx: i,
            rect: (slot_x, sy, slot_w, slot_h),
            primary_btn: (btn_x, btn_y, btn_w, btn_h),
            primary_action,
            delete_btn,
        }
    }).collect();

    TitleLayout { screen, slots: slots_layout }
}

pub fn handle_title_input(layout: &TitleLayout, input: &FrameInput) -> Option<TitleAction> {
    let (mx, my) = input.mouse_pos;

    // Mouse clicks on slots
    if input.mouse_clicked {
        for slot in &layout.slots {
            // Delete button takes precedence (drawn on top)
            if let Some((dx, dy, dw, dh)) = slot.delete_btn {
                if mx >= dx && mx <= dx + dw && my >= dy && my <= dy + dh {
                    return Some(TitleAction::DeleteSlot(slot.idx));
                }
            }
            let (bx, by, bw, bh) = slot.primary_btn;
            if mx >= bx && mx <= bx + bw && my >= by && my <= by + bh {
                return Some(match slot.primary_action {
                    TitleActionKind::LoadGame => TitleAction::LoadGame(slot.idx),
                    TitleActionKind::NewGame => TitleAction::NewGame(slot.idx),
                });
            }
        }
    }

    // 1/2/3 keyboard shortcuts
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3];
    for (i, k) in keys.iter().enumerate() {
        if input.pressed(*k) {
            if let Some(slot) = layout.slots.get(i) {
                return Some(match slot.primary_action {
                    TitleActionKind::LoadGame => TitleAction::LoadGame(i),
                    TitleActionKind::NewGame => TitleAction::NewGame(i),
                });
            }
        }
    }

    None
}

pub fn draw_title(layout: &TitleLayout, slots: &SaveSlots, time: f32, mouse_pos: (f32, f32)) {
    let (sw, sh) = layout.screen;
    let (mx, my) = mouse_pos;

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

    for (slot_l, save) in layout.slots.iter().zip(slots.iter()) {
        let (sx, sy, sw_, sh_) = slot_l.rect;

        // Slot background
        let hover = mx >= sx && mx <= sx + sw_ && my >= sy && my <= sy + sh_;
        let bg = if hover {
            Color::from_rgba(35, 35, 60, 255)
        } else {
            Color::from_rgba(25, 25, 45, 255)
        };
        draw_rectangle(sx, sy, sw_, sh_, bg);
        draw_rectangle_lines(sx, sy, sw_, sh_, 2.0, Color::from_rgba(80, 80, 120, 255));

        // Slot label
        let label = format!("FILE {}", slot_l.idx + 1);
        draw_text(&label, sx + 12.0, sy + 25.0, 16.0, Color::from_rgba(120, 120, 150, 255));

        let (bx, by, bw, bh) = slot_l.primary_btn;
        let btn_hover = mx >= bx && mx <= bx + bw && my >= by && my <= by + bh;

        if let Some(save) = save {
            // Filled slot — show name, playtime
            draw_text(&save.name, sx + 12.0, sy + 50.0, 22.0, WHITE);
            let info = format!("{}  |  {}", save.play_time_display(), save.date_display());
            let iw = measure_text(&info, None, 14, 1.0).width;
            draw_text(&info, sx + sw_ - iw - 100.0, sy + 50.0, 14.0,
                Color::from_rgba(150, 150, 170, 255));

            // LOAD button
            draw_rectangle(bx, by, bw, bh, if btn_hover {
                Color::from_rgba(0, 200, 100, 255)
            } else {
                Color::from_rgba(0, 160, 80, 255)
            });
            let ltw = measure_text("LOAD", None, 16, 1.0).width;
            draw_text("LOAD", bx + bw / 2.0 - ltw / 2.0, by + 22.0, 16.0, WHITE);

            // Delete (X)
            if let Some((dx, dy, _dw, _dh)) = slot_l.delete_btn {
                let del_hover = mx >= dx && mx <= dx + 24.0 && my >= dy && my <= dy + 24.0;
                draw_text("X", dx + 6.0, dy + 18.0, 18.0, if del_hover {
                    Color::from_rgba(244, 67, 54, 255)
                } else {
                    Color::from_rgba(120, 80, 80, 255)
                });
            }
        } else {
            // Empty slot
            draw_text("- empty -", sx + 12.0, sy + 50.0, 18.0,
                Color::from_rgba(80, 80, 100, 255));

            // NEW button
            draw_rectangle(bx, by, bw, bh, if btn_hover {
                Color::from_rgba(33, 150, 243, 255)
            } else {
                Color::from_rgba(25, 118, 200, 255)
            });
            let ntw = measure_text("NEW", None, 16, 1.0).width;
            draw_text("NEW", bx + bw / 2.0 - ntw / 2.0, by + 22.0, 16.0, WHITE);
        }
    }

    // Controls hint
    let hint = "1/2/3 to select slot  |  Arrow keys Move  |  SPACE Talk";
    let hint_w = measure_text(hint, None, 14, 1.0).width;
    draw_text(hint, sw / 2.0 - hint_w / 2.0, sh - 30.0, 14.0,
        Color::from_rgba(100, 100, 120, 255));
}

// ─── NEW GAME SCREEN ────────────────────────────────────

pub const BAND_NAMES: &[&str] = &[
    "Add <5", "+/- <10", "+/- <15", "+/- <20", "x1 x2",
    "+/- <50", "+/- <100", "x1-5", "x1-12", "Divide",
];

pub struct NewGameForm {
    pub name: String,
    pub gender: Gender,
    pub math_band: u8,
    pub slot: usize,
    cursor_blink: f32,
}

pub enum NewGameAction {
    Start,
    Back,
}

pub struct FormLayout {
    pub screen: (f32, f32),
    pub boy_btn: (f32, f32, f32, f32),
    pub girl_btn: (f32, f32, f32, f32),
    pub band_left: (f32, f32, f32, f32),
    pub band_right: (f32, f32, f32, f32),
    pub start_btn: (f32, f32, f32, f32),
    pub start_enabled: bool,
}

pub fn layout_form(form: &NewGameForm, screen: (f32, f32)) -> FormLayout {
    let (sw, _) = screen;

    let btn_w = 120.0;
    let btn_h = 50.0;
    let gap = 20.0;
    let boy_x = sw / 2.0 - btn_w - gap / 2.0;
    let girl_x = sw / 2.0 + gap / 2.0;
    let btn_y = 310.0;

    // Band picker arrow click regions (rough — original did the same loose rect)
    let band_arrow_y = 405.0;
    let band_arrow_h = 20.0;
    // Left half of horizontal centerline
    let band_left = (sw / 2.0 - 100.0, band_arrow_y, 90.0, band_arrow_h);
    let band_right = (sw / 2.0 + 10.0, band_arrow_y, 90.0, band_arrow_h);

    let start_w = 260.0;
    let start_h = 50.0;
    let start_x = sw / 2.0 - start_w / 2.0;
    let start_y = 450.0;

    FormLayout {
        screen,
        boy_btn: (boy_x, btn_y, btn_w, btn_h),
        girl_btn: (girl_x, btn_y, btn_w, btn_h),
        band_left,
        band_right,
        start_btn: (start_x, start_y, start_w, start_h),
        start_enabled: !form.name.is_empty(),
    }
}

impl NewGameForm {
    pub fn new(slot: usize) -> Self {
        NewGameForm {
            name: String::new(),
            gender: Gender::Boy,
            math_band: 1,
            slot,
            cursor_blink: 0.0,
        }
    }

    /// Update cursor + accept text input. Pure logic, no rendering or layout dependency.
    pub fn update(&mut self, dt: f32, input: &FrameInput) {
        self.cursor_blink += dt;
        for ch in &input.chars_typed {
            if self.name.len() < 20 && (ch.is_alphanumeric() || *ch == ' ' || *ch == '-') {
                self.name.push(*ch);
            }
        }
        if input.pressed(KeyCode::Backspace) {
            self.name.pop();
        }
    }

    /// Apply mouse / keyboard actions to gender + band fields.
    pub fn handle_form_clicks(&mut self, layout: &FormLayout, input: &FrameInput) {
        let (mx, my) = input.mouse_pos;

        if input.mouse_clicked {
            let in_rect = |(x, y, w, h): (f32, f32, f32, f32)| {
                mx >= x && mx <= x + w && my >= y && my <= y + h
            };
            if in_rect(layout.boy_btn) { self.gender = Gender::Boy; }
            if in_rect(layout.girl_btn) { self.gender = Gender::Girl; }
            if in_rect(layout.band_left) && self.math_band > 1 { self.math_band -= 1; }
            if in_rect(layout.band_right) && self.math_band < 10 { self.math_band += 1; }
        }

        if input.pressed(KeyCode::Tab) {
            self.gender = match self.gender {
                Gender::Boy => Gender::Girl,
                Gender::Girl => Gender::Boy,
            };
        }
        if input.pressed(KeyCode::Left) && self.math_band > 1 { self.math_band -= 1; }
        if input.pressed(KeyCode::Right) && self.math_band < 10 { self.math_band += 1; }
    }

    /// Detect Start / Back actions. Pure logic.
    pub fn handle_action(&self, layout: &FormLayout, input: &FrameInput) -> Option<NewGameAction> {
        let (mx, my) = input.mouse_pos;
        let (sx, sy, sw, sh) = layout.start_btn;
        let start_hover = mx >= sx && mx <= sx + sw && my >= sy && my <= sy + sh;

        if layout.start_enabled && input.mouse_clicked && start_hover {
            return Some(NewGameAction::Start);
        }
        if layout.start_enabled && input.pressed(KeyCode::Enter) {
            return Some(NewGameAction::Start);
        }
        if input.pressed(KeyCode::Escape) {
            return Some(NewGameAction::Back);
        }
        None
    }

    /// Render only. No input reads, no state mutation.
    pub fn draw(&self, layout: &FormLayout, mouse_pos: (f32, f32)) {
        let (sw, sh) = layout.screen;
        let (mx, my) = mouse_pos;

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
        let name_label = "What's your name, hero?";
        let nlw = measure_text(name_label, None, 22, 1.0).width;
        draw_text(name_label, sw / 2.0 - nlw / 2.0, 180.0, 22.0, WHITE);

        let input_w = 300.0;
        let input_h = 40.0;
        let input_x = sw / 2.0 - input_w / 2.0;
        let input_y = 200.0;
        draw_rectangle(input_x, input_y, input_w, input_h, Color::from_rgba(15, 15, 30, 255));
        draw_rectangle_lines(input_x, input_y, input_w, input_h, 2.0,
            Color::from_rgba(0, 230, 118, 255));

        let display_name = if self.name.is_empty() { "Type your name..." } else { &self.name };
        let name_color = if self.name.is_empty() {
            Color::from_rgba(80, 80, 100, 255)
        } else {
            WHITE
        };
        draw_text(display_name, input_x + 12.0, input_y + 28.0, 22.0, name_color);

        if (self.cursor_blink * 2.0).sin() > 0.0 {
            let cursor_x = input_x + 12.0 + measure_text(&self.name, None, 22, 1.0).width + 2.0;
            draw_line(cursor_x, input_y + 8.0, cursor_x, input_y + 32.0, 2.0,
                Color::from_rgba(0, 230, 118, 255));
        }

        // Gender picker
        let gender_label = "Pick your character:";
        let glw = measure_text(gender_label, None, 22, 1.0).width;
        draw_text(gender_label, sw / 2.0 - glw / 2.0, 290.0, 22.0, WHITE);

        let (boy_x, btn_y, btn_w, btn_h) = layout.boy_btn;
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

        let (girl_x, _, _, _) = layout.girl_btn;
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

        // Level picker
        let level_label = "Starting math level:";
        let llw = measure_text(level_label, None, 22, 1.0).width;
        draw_text(level_label, sw / 2.0 - llw / 2.0, 390.0, 22.0, WHITE);

        let band_name = BAND_NAMES.get((self.math_band - 1) as usize).unwrap_or(&"???");
        let band_display = format!("<  {}  >", band_name);
        let bdw = measure_text(&band_display, None, 22, 1.0).width;
        draw_text(&band_display, sw / 2.0 - bdw / 2.0, 418.0, 22.0,
            Color::from_rgba(255, 213, 79, 255));

        // Start button
        let (start_x, start_y, start_w, start_h) = layout.start_btn;
        let start_hover = mx >= start_x && mx <= start_x + start_w
            && my >= start_y && my <= start_y + start_h;

        let start_color = if !layout.start_enabled {
            Color::from_rgba(60, 60, 80, 255)
        } else if start_hover {
            Color::from_rgba(0, 200, 100, 255)
        } else {
            Color::from_rgba(0, 160, 80, 255)
        };
        draw_rectangle(start_x, start_y, start_w, start_h, start_color);
        let stw = measure_text("START ADVENTURE!", None, 24, 1.0).width;
        draw_text("START ADVENTURE!", start_x + start_w / 2.0 - stw / 2.0, start_y + 33.0,
            24.0, if layout.start_enabled { WHITE } else { Color::from_rgba(80, 80, 100, 255) });

        // Back hint
        let back = "ESC to go back";
        let bw = measure_text(back, None, 14, 1.0).width;
        draw_text(back, sw / 2.0 - bw / 2.0, sh - 30.0, 14.0,
            Color::from_rgba(100, 100, 120, 255));
    }
}
