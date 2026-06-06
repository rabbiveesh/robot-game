//! Quest runner UI. Pure layout + render. Opt-in (feature-flagged) while the
//! quest path is being playtested.
//!
//! Self-contained: a quest plays as a sequence of narrative beats and inline
//! multiple-choice math moments, so it never has to hand control to the
//! challenge state and come back. The game builds a `QuestView` from the domain
//! `QuestSession` (+ the generated puzzle choices) each frame; this module just
//! draws it and reports taps.

use macroquad::prelude::*;

use crate::input::FrameInput;

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

/// What the current quest step looks like to the player. Built by the game from
/// the domain step (+ generated puzzle choices for a MathPuzzle).
pub enum QuestView<'a> {
    Narrative { speaker: &'a str, lines: &'a [String] },
    Travel { label: String },
    Puzzle { prompt: &'a str, choices: &'a [i32] },
    /// A branching decision — the player taps one of the labelled options.
    Choice { prompt: &'a str, options: &'a [String] },
    Reward { dum_dums: u32 },
}

pub struct AnswerTile {
    pub rect: UiRect,
    pub value: i32,
}

/// A labelled, full-width button for a quest Choice step.
pub struct OptionButton {
    pub rect: UiRect,
    pub index: usize,
    pub label: String,
}

pub struct QuestLayout {
    pub panel: UiRect,
    /// Present for Narrative / Travel / Reward steps — tap to continue.
    pub continue_btn: Option<UiRect>,
    /// Present for a Puzzle step.
    pub answers: Vec<AnswerTile>,
    /// Present for a Choice step.
    pub options: Vec<OptionButton>,
}

pub enum QuestClick {
    Continue,
    Answer(i32),
    Choose(usize),
}

pub fn layout(view: &QuestView, screen: (f32, f32)) -> QuestLayout {
    let (sw, sh) = screen;
    let panel_w = (sw - 40.0).min(720.0);
    let panel_h = (sh - 40.0).min(420.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;
    let panel = UiRect { x: panel_x, y: panel_y, w: panel_w, h: panel_h };

    let mut continue_btn = None;
    let mut answers = Vec::new();
    let mut options = Vec::new();
    match view {
        QuestView::Puzzle { choices, .. } => {
            let tile = 84.0;
            let gap = 18.0;
            let n = choices.len();
            let total = tile * n as f32 + gap * (n as f32 - 1.0).max(0.0);
            let start_x = panel_x + (panel_w - total) / 2.0;
            let y = panel_y + panel_h - 150.0;
            for (i, &v) in choices.iter().enumerate() {
                answers.push(AnswerTile {
                    rect: UiRect { x: start_x + i as f32 * (tile + gap), y, w: tile, h: tile },
                    value: v,
                });
            }
        }
        // A Choice with no options is a degenerate beat — show a Continue
        // button so it can still be advanced (never soft-lock).
        QuestView::Choice { options: opts, .. } if opts.is_empty() => {
            continue_btn = Some(UiRect {
                x: panel_x + panel_w / 2.0 - 90.0,
                y: panel_y + panel_h - 64.0,
                w: 180.0,
                h: 44.0,
            });
        }
        QuestView::Choice { options: opts, .. } => {
            // Stacked full-width buttons, bottom-aligned.
            let bh = 46.0;
            let gap = 12.0;
            let bw = panel_w - 64.0;
            let n = opts.len();
            let block_h = n as f32 * bh + (n as f32 - 1.0).max(0.0) * gap;
            let start_y = panel_y + panel_h - 24.0 - block_h;
            for (i, label) in opts.iter().enumerate() {
                options.push(OptionButton {
                    rect: UiRect { x: panel_x + 32.0, y: start_y + i as f32 * (bh + gap), w: bw, h: bh },
                    index: i,
                    label: label.clone(),
                });
            }
        }
        _ => {
            continue_btn = Some(UiRect {
                x: panel_x + panel_w / 2.0 - 90.0,
                y: panel_y + panel_h - 64.0,
                w: 180.0,
                h: 44.0,
            });
        }
    }
    QuestLayout { panel, continue_btn, answers, options }
}

pub fn handle_click(mx: f32, my: f32, layout: &QuestLayout) -> Option<QuestClick> {
    if let Some(btn) = layout.continue_btn {
        if btn.contains(mx, my) {
            return Some(QuestClick::Continue);
        }
    }
    for tile in &layout.answers {
        if tile.rect.contains(mx, my) {
            return Some(QuestClick::Answer(tile.value));
        }
    }
    for opt in &layout.options {
        if opt.rect.contains(mx, my) {
            return Some(QuestClick::Choose(opt.index));
        }
    }
    None
}

pub fn handle_key(input: &FrameInput, layout: &QuestLayout) -> Option<QuestClick> {
    if layout.continue_btn.is_some() && (input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter)) {
        return Some(QuestClick::Continue);
    }
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
    for (i, key) in keys.iter().take(layout.answers.len()).enumerate() {
        if input.pressed(*key) {
            return Some(QuestClick::Answer(layout.answers[i].value));
        }
    }
    for (i, key) in keys.iter().take(layout.options.len()).enumerate() {
        if input.pressed(*key) {
            return Some(QuestClick::Choose(layout.options[i].index));
        }
    }
    None
}

// ─── Drawing ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const TILE_BG: Color = Color::new(0.129, 0.588, 0.953, 1.0);

fn wrap_text(text: &str, max_w: f32, size: u16) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let trial = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if measure_text(&trial, None, size, 1.0).width > max_w && !cur.is_empty() {
            lines.push(cur);
            cur = word.to_string();
        } else {
            cur = trial;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

pub fn draw(view: &QuestView, title: &str, message: Option<&str>, layout: &QuestLayout) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));
    let p = layout.panel;
    draw_rectangle(p.x, p.y, p.w, p.h, DARK_BG);
    draw_rectangle_lines(p.x, p.y, p.w, p.h, 4.0, GOLD);

    let tw = measure_text(title, None, 26, 1.0).width.min(p.w - 32.0);
    draw_text(title, p.x + p.w / 2.0 - tw / 2.0, p.y + 40.0, 26.0, GOLD);

    let body = match view {
        QuestView::Narrative { speaker, lines } => format!("{speaker}: {}", lines.join(" ")),
        QuestView::Travel { label } => label.clone(),
        QuestView::Puzzle { prompt, .. } => prompt.to_string(),
        QuestView::Choice { prompt, .. } => prompt.to_string(),
        QuestView::Reward { dum_dums } => format!("You earned {dum_dums} Dum Dums!"),
    };
    let mut y = p.y + 86.0;
    for line in wrap_text(&body, p.w - 56.0, 24) {
        draw_text(&line, p.x + 28.0, y, 24.0, WHITE);
        y += 30.0;
    }

    for tile in &layout.answers {
        let t = tile.rect;
        draw_rectangle(t.x, t.y, t.w, t.h, TILE_BG);
        draw_rectangle_lines(t.x, t.y, t.w, t.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
        let s = format!("{}", tile.value);
        let size = 34u16;
        let sw2 = measure_text(&s, None, size, 1.0).width;
        draw_text(&s, t.x + t.w / 2.0 - sw2 / 2.0, t.y + t.h / 2.0 + 12.0, size as f32, WHITE);
    }

    for opt in &layout.options {
        let r = opt.rect;
        draw_rectangle(r.x, r.y, r.w, r.h, TILE_BG);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
        let label = format!("{}. {}", opt.index + 1, opt.label);
        let lw = measure_text(&label, None, 22, 1.0).width;
        draw_text(&label, r.x + r.w / 2.0 - lw / 2.0, r.y + r.h / 2.0 + 8.0, 22.0, WHITE);
    }

    if let Some(msg) = message {
        let mw = measure_text(msg, None, 22, 1.0).width;
        draw_text(msg, p.x + p.w / 2.0 - mw / 2.0, p.y + p.h - 96.0, 22.0, GOLD);
    }

    if let Some(btn) = layout.continue_btn {
        draw_rectangle(btn.x, btn.y, btn.w, btn.h, TILE_BG);
        draw_rectangle_lines(btn.x, btn.y, btn.w, btn.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
        let label = "Continue";
        let lw = measure_text(label, None, 22, 1.0).width;
        draw_text(label, btn.x + btn.w / 2.0 - lw / 2.0, btn.y + btn.h / 2.0 + 8.0, 22.0, WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A Choice step must render one button per option and report the TAPPED
    // index — not always 0 (the bug this fixes).
    #[test]
    fn choice_options_map_to_their_own_index() {
        let opts = vec!["Confront".to_string(), "Set a trap".to_string()];
        let view = QuestView::Choice { prompt: "How?", options: &opts };
        let screen = (960.0, 720.0);
        let layout = layout(&view, screen);
        assert_eq!(layout.options.len(), 2, "one button per option");
        assert!(layout.continue_btn.is_none(), "a Choice has no Continue button");

        // Tapping the second option's centre yields Choose(1), not Choose(0).
        let r = layout.options[1].rect;
        match handle_click(r.x + r.w / 2.0, r.y + r.h / 2.0, &layout) {
            Some(QuestClick::Choose(1)) => {}
            other => panic!("expected Choose(1), got something else: {}",
                matches!(other, Some(QuestClick::Choose(0)))),
        }
        // First option → Choose(0).
        let r0 = layout.options[0].rect;
        assert!(matches!(
            handle_click(r0.x + r0.w / 2.0, r0.y + r0.h / 2.0, &layout),
            Some(QuestClick::Choose(0)),
        ));
    }
}
