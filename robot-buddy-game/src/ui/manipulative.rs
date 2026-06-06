//! CRA manipulative UI — the hands-on alternative to the multiple-choice quiz.
//! Pure layout + render; the domain reducers do the math.
//!
//! Wraps the concrete and representational manipulative sessions behind one
//! `Manip` enum so `game.rs` can drive any of them through a single state.
//! Opt-in (feature-flagged) while the path is being playtested.

use macroquad::prelude::*;
use robot_buddy_domain::logic::manipulate_concrete::{
    ConcreteAction, ConcreteKind, ConcretePhase, ConcreteSession,
};
use robot_buddy_domain::logic::number_line::{
    NumberLineAction, NumberLinePhase, NumberLineSession,
};

use crate::input::FrameInput;

/// The active manipulative, holding its domain session.
pub enum Manip {
    Concrete(ConcreteSession),
    NumberLine(NumberLineSession),
}

impl Manip {
    pub fn is_complete(&self) -> bool {
        match self {
            Manip::Concrete(s) => s.phase == ConcretePhase::Complete,
            Manip::NumberLine(s) => s.phase == NumberLinePhase::Complete,
        }
    }
}

/// A domain action produced by tapping a button, applied by the game via the
/// matching reducer.
#[derive(Clone, Copy)]
pub enum ManipInput {
    Concrete(ConcreteAction),
    NumberLine(NumberLineAction),
}

#[derive(Clone, Copy)]
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

pub struct Button {
    pub rect: UiRect,
    pub label: String,
    pub input: ManipInput,
}

pub struct ManipLayout {
    pub panel: UiRect,
    pub buttons: Vec<Button>,
}

// ─── Layout ─────────────────────────────────────────────

pub fn layout(manip: &Manip, screen: (f32, f32)) -> ManipLayout {
    let (sw, sh) = screen;
    let panel_w = (sw - 40.0).min(720.0);
    let panel_h = (sh - 40.0).min(520.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;
    let panel = UiRect { x: panel_x, y: panel_y, w: panel_w, h: panel_h };

    let specs: Vec<(String, ManipInput)> = match manip {
        Manip::Concrete(s) => concrete_buttons(s),
        Manip::NumberLine(_) => vec![
            ("◀ Back".into(), ManipInput::NumberLine(NumberLineAction::JumpBackward { n: 1 })),
            ("Forward ▶".into(), ManipInput::NumberLine(NumberLineAction::JumpForward { n: 1 })),
        ],
    };

    let n = specs.len().max(1);
    let bw = 150.0_f32.min((panel_w - 48.0) / n as f32 - 12.0);
    let bh = 56.0;
    let gap = 16.0;
    let total = bw * n as f32 + gap * (n as f32 - 1.0);
    let start_x = panel_x + (panel_w - total) / 2.0;
    let y = panel_y + panel_h - 96.0;
    let buttons = specs
        .into_iter()
        .enumerate()
        .map(|(i, (label, input))| Button {
            rect: UiRect { x: start_x + i as f32 * (bw + gap), y, w: bw, h: bh },
            label,
            input,
        })
        .collect();

    ManipLayout { panel, buttons }
}

fn concrete_buttons(s: &ConcreteSession) -> Vec<(String, ManipInput)> {
    use ConcreteKind::*;
    match s.puzzle.kind {
        Count | BuildTower => vec![
            ("Add one".into(), ManipInput::Concrete(ConcreteAction::Place { group: 0 })),
            ("Undo".into(), ManipInput::Concrete(ConcreteAction::Remove { group: 0 })),
        ],
        AddGroups => {
            // Undo removes from whichever bucket has items (prefer the second).
            let undo_group = if s.bucket_b > 0 { 1 } else { 0 };
            vec![
                ("+ Red".into(), ManipInput::Concrete(ConcreteAction::Place { group: 0 })),
                ("+ Blue".into(), ManipInput::Concrete(ConcreteAction::Place { group: 1 })),
                ("Undo".into(), ManipInput::Concrete(ConcreteAction::Remove { group: undo_group })),
            ]
        }
        TakeAway => vec![
            ("Take one".into(), ManipInput::Concrete(ConcreteAction::Remove { group: 0 })),
            ("Put back".into(), ManipInput::Concrete(ConcreteAction::Place { group: 0 })),
        ],
    }
}

// ─── Input ──────────────────────────────────────────────

pub fn handle_click(mx: f32, my: f32, manip: &Manip, layout: &ManipLayout) -> Option<ManipInput> {
    if manip.is_complete() {
        return None;
    }
    layout.buttons.iter().find(|b| b.rect.contains(mx, my)).map(|b| b.input)
}

pub fn handle_key(manip: &Manip, input: &FrameInput, layout: &ManipLayout) -> Option<ManipInput> {
    if manip.is_complete() {
        return None;
    }
    // Arrow keys drive the number line; number keys map to buttons in order.
    if let Manip::NumberLine(_) = manip {
        if input.pressed(KeyCode::Left) {
            return Some(ManipInput::NumberLine(NumberLineAction::JumpBackward { n: 1 }));
        }
        if input.pressed(KeyCode::Right) {
            return Some(ManipInput::NumberLine(NumberLineAction::JumpForward { n: 1 }));
        }
    }
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3];
    for (i, key) in keys.iter().take(layout.buttons.len()).enumerate() {
        if input.pressed(*key) {
            return Some(layout.buttons[i].input);
        }
    }
    None
}

// ─── Drawing ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const WIN_GREEN: Color = Color::new(0.412, 0.941, 0.682, 1.0);
const BTN_BG: Color = Color::new(0.129, 0.588, 0.953, 1.0);
const RED: Color = Color::new(0.90, 0.30, 0.30, 1.0);
const BLUE: Color = Color::new(0.30, 0.55, 0.95, 1.0);

pub fn draw(manip: &Manip, prompt: &str, layout: &ManipLayout) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));
    let p = layout.panel;
    draw_rectangle(p.x, p.y, p.w, p.h, DARK_BG);
    draw_rectangle_lines(p.x, p.y, p.w, p.h, 4.0, GOLD);

    let done = manip.is_complete();
    let header = if done { "YOU DID IT!" } else { prompt };
    let hc = if done { WIN_GREEN } else { GOLD };
    let hw = measure_text(header, None, 26, 1.0).width.min(p.w - 32.0);
    draw_text(header, p.x + p.w / 2.0 - hw / 2.0, p.y + 42.0, 26.0, hc);

    match manip {
        Manip::Concrete(s) => draw_concrete(s, p),
        Manip::NumberLine(s) => draw_number_line(s, p),
    }

    if done {
        let msg = "Tap or press SPACE to continue";
        let mw = measure_text(msg, None, 22, 1.0).width;
        if (get_time() * 4.0).sin() > 0.0 {
            draw_text(msg, p.x + p.w / 2.0 - mw / 2.0, p.y + p.h - 30.0, 22.0, GOLD);
        }
    } else {
        for b in &layout.buttons {
            draw_rectangle(b.rect.x, b.rect.y, b.rect.w, b.rect.h, BTN_BG);
            draw_rectangle_lines(b.rect.x, b.rect.y, b.rect.w, b.rect.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
            let tw = measure_text(&b.label, None, 24, 1.0).width;
            draw_text(&b.label, b.rect.x + b.rect.w / 2.0 - tw / 2.0, b.rect.y + b.rect.h / 2.0 + 8.0, 24.0, WHITE);
        }
    }
}

fn draw_dot_row(cx_start: f32, y: f32, count: u8, color: Color) -> f32 {
    let r = 16.0;
    let gap = 10.0;
    let mut x = cx_start;
    for _ in 0..count {
        draw_circle(x + r, y, r, color);
        draw_circle_lines(x + r, y, r, 1.5, Color::new(0.0, 0.0, 0.0, 0.4));
        x += r * 2.0 + gap;
    }
    x
}

fn draw_concrete(s: &ConcreteSession, p: UiRect) {
    let target_line = format!("Make {}   (now: {})", s.puzzle.target, s.total());
    let tw = measure_text(&target_line, None, 24, 1.0).width;
    draw_text(&target_line, p.x + p.w / 2.0 - tw / 2.0, p.y + 86.0, 24.0, WHITE);

    let y = p.y + p.h / 2.0;
    let start_x = p.x + 40.0;
    let after_a = draw_dot_row(start_x, y, s.bucket_a, RED);
    draw_dot_row(after_a + 14.0, y, s.bucket_b, BLUE);
}

fn draw_number_line(s: &NumberLineSession, p: UiRect) {
    let max = s.puzzle.max as usize;
    let line_y = p.y + p.h / 2.0;
    let left = p.x + 50.0;
    let right = p.x + p.w - 50.0;
    let span = right - left;
    let step = span / max as f32;
    draw_line(left, line_y, right, line_y, 3.0, WHITE);

    for i in 0..=max {
        let x = left + i as f32 * step;
        draw_line(x, line_y - 8.0, x, line_y + 8.0, 2.0, WHITE);
        let label = format!("{i}");
        let lw = measure_text(&label, None, 18, 1.0).width;
        draw_text(&label, x - lw / 2.0, line_y + 30.0, 18.0, Color::new(0.8, 0.85, 0.95, 1.0));
        if i as u8 == s.puzzle.target {
            draw_circle_lines(x, line_y, 18.0, 3.0, GOLD);
        }
    }
    // The character marker at the current position.
    let mx = left + s.position as f32 * step;
    draw_circle(mx, line_y - 26.0, 12.0, WIN_GREEN);
    draw_line(mx, line_y - 14.0, mx, line_y, 2.0, WIN_GREEN);
}
