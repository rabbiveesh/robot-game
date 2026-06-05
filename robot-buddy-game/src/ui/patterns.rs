//! Pattern-sequence puzzle UI.
//!
//! Single entrypoint surface for the game-side glue (mirrors `ui::kenken`):
//!   - `layout(session, screen)` — pure, hit-testable rectangles
//!   - `draw_pattern(session, layout, time)` — render
//!   - `handle_click(mx, my, session, layout)` → PatternInput
//!   - `handle_key(session, input)` → PatternInput
//!
//! `PatternInput` is the only type game.rs cares about — it wraps a domain
//! `PatternAction`.

use macroquad::prelude::*;
use robot_buddy_domain::logic::patterns::{
    PatternAction, PatternElement, PatternPhase, PatternSession,
};

use crate::input::FrameInput;

// ─── Layout types ───────────────────────────────────────

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

pub struct ChoiceBound {
    pub rect: UiRect,
    pub index: usize,
}

pub struct PatternLayout {
    pub panel: UiRect,
    /// Boxes showing the visible sequence, plus a trailing "?" slot.
    pub sequence: Vec<UiRect>,
    pub blank: UiRect,
    pub choices: Vec<ChoiceBound>,
}

pub enum PatternInput {
    Action(PatternAction),
}

// ─── Layout (pure) ──────────────────────────────────────

pub fn layout(session: &PatternSession, screen: (f32, f32)) -> PatternLayout {
    let (sw, sh) = screen;
    let panel_w = (sw - 40.0).min(760.0);
    let panel_h = (sh - 40.0).min(520.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;

    let n_seq = session.puzzle.visible_elements.len() + 1; // + blank slot
    let gap = 12.0;
    let max_box = 84.0;
    let avail = panel_w - 48.0;
    let box_size = (((avail - gap * (n_seq as f32 - 1.0)) / n_seq as f32).floor()).min(max_box);
    let total_w = box_size * n_seq as f32 + gap * (n_seq as f32 - 1.0);
    let start_x = panel_x + (panel_w - total_w) / 2.0;
    let seq_y = panel_y + 96.0;

    let mut sequence = Vec::with_capacity(n_seq - 1);
    for i in 0..(n_seq - 1) {
        sequence.push(UiRect {
            x: start_x + i as f32 * (box_size + gap),
            y: seq_y,
            w: box_size,
            h: box_size,
        });
    }
    let blank = UiRect {
        x: start_x + (n_seq - 1) as f32 * (box_size + gap),
        y: seq_y,
        w: box_size,
        h: box_size,
    };

    // Choices laid out in a centered row below the sequence.
    let n_ch = session.puzzle.choices.len();
    let ch_size = 80.0;
    let ch_gap = 16.0;
    let ch_total = ch_size * n_ch as f32 + ch_gap * (n_ch as f32 - 1.0).max(0.0);
    let ch_start_x = panel_x + (panel_w - ch_total) / 2.0;
    let ch_y = seq_y + box_size + 72.0;
    let mut choices = Vec::with_capacity(n_ch);
    for i in 0..n_ch {
        choices.push(ChoiceBound {
            rect: UiRect {
                x: ch_start_x + i as f32 * (ch_size + ch_gap),
                y: ch_y,
                w: ch_size,
                h: ch_size,
            },
            index: i,
        });
    }

    PatternLayout {
        panel: UiRect { x: panel_x, y: panel_y, w: panel_w, h: panel_h },
        sequence,
        blank,
        choices,
    }
}

// ─── Input handling ─────────────────────────────────────

pub fn handle_click(
    mx: f32,
    my: f32,
    session: &PatternSession,
    layout: &PatternLayout,
) -> Option<PatternInput> {
    if session.phase == PatternPhase::Complete {
        return None;
    }
    for ch in &layout.choices {
        if ch.rect.contains(mx, my) {
            return Some(PatternInput::Action(PatternAction::Select { choice: ch.index }));
        }
    }
    None
}

pub fn handle_key(session: &PatternSession, input: &FrameInput) -> Option<PatternInput> {
    if session.phase == PatternPhase::Complete {
        return None;
    }
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
    let n = session.puzzle.choices.len();
    for (i, key) in keys.iter().take(n).enumerate() {
        if input.pressed(*key) {
            return Some(PatternInput::Action(PatternAction::Select { choice: i }));
        }
    }
    None
}

// ─── Drawing ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const WIN_GREEN: Color = Color::new(0.412, 0.941, 0.682, 1.0);
const SLOT_BG: Color = Color::new(0.16, 0.18, 0.28, 1.0);
const BLANK_BG: Color = Color::new(0.22, 0.20, 0.10, 1.0);
const CHOICE_BG: Color = Color::new(0.129, 0.588, 0.953, 1.0);

/// Stable color per named element so the same sprite/color/shape always renders
/// the same hue across the sequence and the choices.
fn element_color(el: &PatternElement) -> Color {
    let key = match el {
        PatternElement::Color { color } => color.clone(),
        PatternElement::Sprite { name } => name.clone(),
        PatternElement::Shape { shape } => shape.clone(),
        // Numbers render as a light tile; the digit carries the meaning.
        PatternElement::Number { .. } => return Color::new(0.92, 0.93, 0.97, 1.0),
    };
    match key.as_str() {
        "red" => Color::new(0.90, 0.30, 0.30, 1.0),
        "blue" => Color::new(0.30, 0.55, 0.95, 1.0),
        "green" => Color::new(0.35, 0.80, 0.45, 1.0),
        "yellow" => Color::new(0.95, 0.85, 0.30, 1.0),
        "purple" => Color::new(0.70, 0.45, 0.90, 1.0),
        // Sprites / shapes get a deterministic palette by first byte.
        other => {
            let h = other.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
            let r = 0.4 + ((h & 0xFF) as f32 / 255.0) * 0.5;
            let g = 0.4 + (((h >> 8) & 0xFF) as f32 / 255.0) * 0.5;
            let b = 0.4 + (((h >> 16) & 0xFF) as f32 / 255.0) * 0.5;
            Color::new(r, g, b, 1.0)
        }
    }
}

fn element_glyph(el: &PatternElement) -> String {
    match el {
        PatternElement::Number { value } => format!("{value}"),
        PatternElement::Shape { shape } => match shape.as_str() {
            "circle" => "●",
            "square" => "■",
            "triangle" => "▲",
            "diamond" => "◆",
            "heart" => "♥",
            _ => "★",
        }
        .into(),
        PatternElement::Sprite { name } => name.chars().next().unwrap_or('?').to_uppercase().to_string(),
        PatternElement::Color { .. } => String::new(),
    }
}

fn draw_element(rect: UiRect, el: &PatternElement) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, element_color(el));
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, Color::new(0.0, 0.0, 0.0, 0.35));
    let glyph = element_glyph(el);
    if !glyph.is_empty() {
        let size = (rect.w * 0.5) as u16;
        let tw = measure_text(&glyph, None, size, 1.0).width;
        draw_text(
            &glyph,
            rect.x + rect.w / 2.0 - tw / 2.0,
            rect.y + rect.h / 2.0 + size as f32 * 0.35,
            size as f32,
            Color::new(0.08, 0.08, 0.12, 1.0),
        );
    }
}

pub fn draw_pattern(session: &PatternSession, layout: &PatternLayout, _time: f32) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));

    let p = layout.panel;
    draw_rectangle(p.x, p.y, p.w, p.h, DARK_BG);
    draw_rectangle_lines(p.x, p.y, p.w, p.h, 4.0, GOLD);

    let solved = session.phase == PatternPhase::Complete;
    let header = if solved { "YOU GOT IT!" } else { "What comes next?" };
    let header_color = if solved { WIN_GREEN } else { GOLD };
    let hw = measure_text(header, None, 32, 1.0).width;
    draw_text(header, p.x + p.w / 2.0 - hw / 2.0, p.y + 46.0, 32.0, header_color);

    // The visible sequence.
    for (i, rect) in layout.sequence.iter().enumerate() {
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, SLOT_BG);
        draw_element(*rect, &session.puzzle.visible_elements[i]);
    }

    // The blank slot — shows "?" until solved, then the answer.
    let b = layout.blank;
    if solved {
        draw_element(b, &session.puzzle.correct_answer);
    } else {
        draw_rectangle(b.x, b.y, b.w, b.h, BLANK_BG);
        draw_rectangle_lines(b.x, b.y, b.w, b.h, 3.0, GOLD);
        let q = "?";
        let size = (b.w * 0.6) as u16;
        let tw = measure_text(q, None, size, 1.0).width;
        draw_text(q, b.x + b.w / 2.0 - tw / 2.0, b.y + b.h / 2.0 + size as f32 * 0.35, size as f32, GOLD);
    }

    // Choices (hidden once solved).
    if !solved {
        for ch in &layout.choices {
            let bounced = session.last_wrong == Some(ch.index);
            // A just-bounced wrong choice flashes — natural "try again" cue.
            let tint = if bounced { Color::new(1.0, 0.4, 0.4, 1.0) } else { CHOICE_BG };
            draw_rectangle(ch.rect.x, ch.rect.y, ch.rect.w, ch.rect.h, tint);
            draw_rectangle_lines(ch.rect.x, ch.rect.y, ch.rect.w, ch.rect.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
            draw_element(
                UiRect { x: ch.rect.x + 8.0, y: ch.rect.y + 8.0, w: ch.rect.w - 16.0, h: ch.rect.h - 16.0 },
                &session.puzzle.choices[ch.index],
            );
        }
        let hint = "Tap the piece that comes next";
        let hw = measure_text(hint, None, 22, 1.0).width;
        draw_text(hint, p.x + p.w / 2.0 - hw / 2.0, b.y + b.h + 44.0, 22.0, Color::new(0.8, 0.85, 0.95, 1.0));
    } else {
        let dismiss = "Tap or press SPACE to continue";
        let dw = measure_text(dismiss, None, 24, 1.0).width;
        if (get_time() * 4.0).sin() > 0.0 {
            draw_text(dismiss, p.x + p.w / 2.0 - dw / 2.0, p.y + p.h - 40.0, 24.0, GOLD);
        }
    }
}
