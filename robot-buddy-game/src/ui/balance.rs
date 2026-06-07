//! Balance-scale puzzle UI.
//!
//! Single entrypoint surface (mirrors `ui::patterns`):
//!   - `layout(session, screen)` — pure, hit-testable rectangles
//!   - `draw_balance(session, layout, time)` — render the scale + choices
//!   - `handle_click(mx, my, session, layout)` → BalanceInput
//!   - `handle_key(session, input)` → BalanceInput

use macroquad::prelude::*;
use robot_buddy_domain::logic::balance::{
    BalanceAction, BalanceItem, BalancePhase, BalanceSession,
};
use robot_buddy_domain::types::Operation;

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

pub struct ChoiceBound {
    pub rect: UiRect,
    pub value: i32,
}

pub struct BalanceLayout {
    pub panel: UiRect,
    pub fulcrum: (f32, f32),
    pub beam_half: f32,
    pub choices: Vec<ChoiceBound>,
}

pub enum BalanceInput {
    Action(BalanceAction),
}

// ─── Layout (pure) ──────────────────────────────────────

pub fn layout(session: &BalanceSession, screen: (f32, f32)) -> BalanceLayout {
    let (sw, sh) = screen;
    let panel_w = (sw - 40.0).min(720.0);
    let panel_h = (sh - 40.0).min(520.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;

    let fulcrum = (panel_x + panel_w / 2.0, panel_y + 250.0);
    let beam_half = (panel_w * 0.32).min(220.0);

    let n = session.puzzle.choices.len();
    let ch_size = 70.0;
    let ch_gap = 16.0;
    let total = ch_size * n as f32 + ch_gap * (n as f32 - 1.0).max(0.0);
    let start_x = panel_x + (panel_w - total) / 2.0;
    let ch_y = panel_y + panel_h - 110.0;
    let mut choices = Vec::with_capacity(n);
    for (i, &v) in session.puzzle.choices.iter().enumerate() {
        choices.push(ChoiceBound {
            rect: UiRect { x: start_x + i as f32 * (ch_size + ch_gap), y: ch_y, w: ch_size, h: ch_size },
            value: v,
        });
    }

    BalanceLayout {
        panel: UiRect { x: panel_x, y: panel_y, w: panel_w, h: panel_h },
        fulcrum,
        beam_half,
        choices,
    }
}

// ─── Input ──────────────────────────────────────────────

pub fn handle_click(
    mx: f32,
    my: f32,
    session: &BalanceSession,
    layout: &BalanceLayout,
) -> Option<BalanceInput> {
    if session.phase == BalancePhase::Complete {
        return None;
    }
    for ch in &layout.choices {
        if ch.rect.contains(mx, my) {
            return Some(BalanceInput::Action(BalanceAction::Guess { value: ch.value }));
        }
    }
    None
}

pub fn handle_key(session: &BalanceSession, input: &FrameInput) -> Option<BalanceInput> {
    if session.phase == BalancePhase::Complete {
        return None;
    }
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
    for (i, key) in keys.iter().take(session.puzzle.choices.len()).enumerate() {
        if input.pressed(*key) {
            let value = session.puzzle.choices[i];
            return Some(BalanceInput::Action(BalanceAction::Guess { value }));
        }
    }
    None
}

// ─── Drawing ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const WIN_GREEN: Color = Color::new(0.412, 0.941, 0.682, 1.0);
const BEAM: Color = Color::new(0.80, 0.55, 0.25, 1.0);
const PAN: Color = Color::new(0.65, 0.70, 0.80, 1.0);
const KNOWN_BG: Color = Color::new(0.92, 0.93, 0.97, 1.0);
const UNKNOWN_BG: Color = Color::new(0.95, 0.80, 0.30, 1.0);
const CHOICE_BG: Color = Color::new(0.129, 0.588, 0.953, 1.0);

fn op_symbol(op: Operation) -> &'static str {
    match op {
        Operation::Add => "+",
        Operation::Sub => "-",
        Operation::Multiply => "x",
        Operation::Divide => "/",
        Operation::NumberBond => "+",
    }
}

fn item_label(item: &BalanceItem) -> String {
    match item {
        BalanceItem::Known { value } => format!("{value}"),
        BalanceItem::Unknown => "?".into(),
        BalanceItem::Op { op } => op_symbol(*op).into(),
    }
}

/// Vertical pan offsets `(left, right)` in pixels; a positive value means that
/// pan drops. The heavier pan (more weight under the kid's last guess) drops —
/// the physically-correct direction.
fn pan_offsets(session: &BalanceSession) -> (f32, f32) {
    if session.phase == BalancePhase::Complete {
        return (0.0, 0.0);
    }
    match session.last_wrong {
        Some(guess) => {
            // tilt() > 0 → right side heavier → right pan drops.
            let lean = (session.puzzle.tilt(guess).signum() as f32) * 26.0;
            (-lean, lean)
        }
        None => (0.0, 0.0),
    }
}

fn draw_pan(center_x: f32, center_y: f32, items: &[BalanceItem]) {
    // Pan dish.
    draw_line(center_x - 46.0, center_y, center_x + 46.0, center_y, 4.0, PAN);
    draw_line(center_x - 46.0, center_y, center_x, center_y + 18.0, 3.0, PAN);
    draw_line(center_x + 46.0, center_y, center_x, center_y + 18.0, 3.0, PAN);

    // Blocks sitting on the dish.
    let box_w = 34.0;
    let gap = 6.0;
    let total = items.len() as f32 * box_w + (items.len() as f32 - 1.0).max(0.0) * gap;
    let mut x = center_x - total / 2.0;
    let y = center_y - box_w - 4.0;
    for item in items {
        let is_op = matches!(item, BalanceItem::Op { .. });
        let label = item_label(item);
        if is_op {
            // Operators float between blocks, no box.
            let size = 26u16;
            let tw = measure_text(&label, None, size, 1.0).width;
            draw_text(&label, x + box_w / 2.0 - tw / 2.0, y + box_w / 2.0 + 9.0, size as f32, GOLD);
        } else {
            let bg = if matches!(item, BalanceItem::Unknown) { UNKNOWN_BG } else { KNOWN_BG };
            draw_rectangle(x, y, box_w, box_w, bg);
            draw_rectangle_lines(x, y, box_w, box_w, 2.0, Color::new(0.0, 0.0, 0.0, 0.4));
            let size = 24u16;
            let tw = measure_text(&label, None, size, 1.0).width;
            draw_text(&label, x + box_w / 2.0 - tw / 2.0, y + box_w / 2.0 + 8.0, size as f32, Color::new(0.08, 0.08, 0.12, 1.0));
        }
        x += box_w + gap;
    }
}

pub fn draw_balance(session: &BalanceSession, layout: &BalanceLayout, _time: f32) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));

    let p = layout.panel;
    draw_rectangle(p.x, p.y, p.w, p.h, DARK_BG);
    draw_rectangle_lines(p.x, p.y, p.w, p.h, 4.0, GOLD);

    let solved = session.phase == BalancePhase::Complete;
    let header = if solved { "BALANCED!" } else { "Make it balance!" };
    let header_color = if solved { WIN_GREEN } else { GOLD };
    let hw = measure_text(header, None, 32, 1.0).width;
    draw_text(header, p.x + p.w / 2.0 - hw / 2.0, p.y + 46.0, 32.0, header_color);

    let (fx, fy) = layout.fulcrum;
    let half = layout.beam_half;

    // Stand + fulcrum triangle.
    draw_line(fx, fy, fx, fy + 120.0, 6.0, BEAM);
    draw_triangle(
        Vec2::new(fx - 22.0, fy + 120.0),
        Vec2::new(fx + 22.0, fy + 120.0),
        Vec2::new(fx, fy + 80.0),
        BEAM,
    );

    // Beam tilts: left pan rises/falls opposite the right pan.
    let (left_dy, right_dy) = pan_offsets(session);
    let left_y = fy + left_dy;
    let right_y = fy + right_dy;
    draw_line(fx - half, left_y, fx + half, right_y, 6.0, BEAM);

    // Hangers + pans.
    draw_line(fx - half, left_y, fx - half, left_y + 40.0, 2.0, PAN);
    draw_line(fx + half, right_y, fx + half, right_y + 40.0, 2.0, PAN);
    draw_pan(fx - half, left_y + 40.0, &session.puzzle.left_side);
    draw_pan(fx + half, right_y + 40.0, &session.puzzle.right_side);

    if !solved {
        for ch in &layout.choices {
            let bounced = session.last_wrong == Some(ch.value);
            let tint = if bounced { Color::new(1.0, 0.4, 0.4, 1.0) } else { CHOICE_BG };
            draw_rectangle(ch.rect.x, ch.rect.y, ch.rect.w, ch.rect.h, tint);
            draw_rectangle_lines(ch.rect.x, ch.rect.y, ch.rect.w, ch.rect.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
            let label = format!("{}", ch.value);
            let size = 30u16;
            let tw = measure_text(&label, None, size, 1.0).width;
            draw_text(&label, ch.rect.x + ch.rect.w / 2.0 - tw / 2.0, ch.rect.y + ch.rect.h / 2.0 + 10.0, size as f32, WHITE);
        }
        let hint = "What does the ? weigh? Tap a number.";
        let hw = measure_text(hint, None, 22, 1.0).width;
        draw_text(hint, p.x + p.w / 2.0 - hw / 2.0, p.y + p.h - 24.0, 22.0, Color::new(0.8, 0.85, 0.95, 1.0));
    } else {
        let dismiss = "Tap or press SPACE to continue";
        let dw = measure_text(dismiss, None, 24, 1.0).width;
        if (get_time() * 4.0).sin() > 0.0 {
            draw_text(dismiss, p.x + p.w / 2.0 - dw / 2.0, p.y + p.h - 30.0, 24.0, GOLD);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robot_buddy_domain::logic::balance::{BalanceItem, BalancePuzzle, BalanceSession};
    use robot_buddy_domain::types::Operation;

    #[test]
    fn heavier_pan_drops() {
        // Puzzle: ? + 3 = 5 (answer 2). left = [?, +, 3], right = [5].
        let puzzle = BalancePuzzle {
            left_side: vec![
                BalanceItem::Unknown,
                BalanceItem::Op { op: Operation::Add },
                BalanceItem::Known { value: 3 },
            ],
            right_side: vec![BalanceItem::Known { value: 5 }],
            correct_answer: 2,
            choices: vec![1, 2, 3],
        };
        let mut s = BalanceSession::new(puzzle);
        // Guess too high (4): left = 4+3 = 7 > right 5 → LEFT is heavier → left drops.
        s.last_wrong = Some(4);
        let (left, right) = pan_offsets(&s);
        assert!(left > right, "left (heavier) pan should drop: left={left}, right={right}");

        // Guess too low (0): left = 0+3 = 3 < right 5 → RIGHT heavier → right drops.
        s.last_wrong = Some(0);
        let (left, right) = pan_offsets(&s);
        assert!(right > left, "right (heavier) pan should drop: left={left}, right={right}");
    }
}
