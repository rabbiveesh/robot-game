use crate::prelude::*;
use crate::ui::layout::{self, col, gap_box, paint, row, spacer, Fit, Frame, Justify, Kind};
use crate::audio;
use crate::settings;

pub struct DialogueLine {
    pub speaker: String,
    pub text: String,
}

pub struct DialogueBox {
    lines: Vec<DialogueLine>,
    current_line: usize,
    char_index: usize,
    char_timer: f32,
    waiting_for_input: bool,
    pub active: bool,
}

impl DialogueBox {
    pub fn new() -> Self {
        DialogueBox {
            lines: vec![],
            current_line: 0,
            char_index: 0,
            char_timer: 0.0,
            waiting_for_input: false,
            active: false,
        }
    }

    pub fn start(&mut self, lines: Vec<DialogueLine>) {
        // Speak the first line
        if let Some(line) = lines.first() {
            audio::tts::speak(&line.speaker, &line.text);
        }
        self.lines = lines;
        self.current_line = 0;
        self.char_index = 0;
        self.char_timer = 0.0;
        self.waiting_for_input = false;
        self.active = true;
    }

    /// True if the typewriter is still revealing characters (not yet waiting for input).
    pub fn is_typewriting(&self) -> bool {
        self.active && !self.waiting_for_input
    }

    /// Read-only view of the queued lines. Used by integration tests to assert
    /// on who says what during a scene without driving the typewriter.
    pub fn lines(&self) -> &[DialogueLine] {
        &self.lines
    }

    pub fn advance(&mut self) {
        if !self.active { return; }
        if !self.waiting_for_input {
            // Skip typewriter — show full line
            if let Some(line) = self.lines.get(self.current_line) {
                self.char_index = line.text.len();
                self.waiting_for_input = true;
            }
            return;
        }
        self.current_line += 1;
        if self.current_line >= self.lines.len() {
            self.active = false;
            audio::tts::cancel();
            return;
        }
        // Speak the next line
        if let Some(line) = self.lines.get(self.current_line) {
            audio::tts::speak(&line.speaker, &line.text);
        }
        self.char_index = 0;
        self.char_timer = 0.0;
        self.waiting_for_input = false;
    }

    pub fn update(&mut self, dt: f32) {
        if !self.active || self.waiting_for_input { return; }
        if let Some(line) = self.lines.get(self.current_line) {
            self.char_timer += dt;
            let char_speed = settings::char_speed_seconds();
            while self.char_timer >= char_speed && self.char_index < line.text.len() {
                self.char_timer -= char_speed;
                // Advance by one character (handle UTF-8 properly)
                let remaining = &line.text[self.char_index..];
                if let Some(c) = remaining.chars().next() {
                    self.char_index += c.len_utf8();
                }
            }
            if self.char_index >= line.text.len() {
                self.waiting_for_input = true;
            }
        }
    }

    /// Draw the current line. Layout comes from [`layout`]; the typewriter
    /// only reveals a prefix of lines broken on the FULL text, so words never
    /// hop to the next line mid-sentence and the box never grows mid-line.
    pub fn draw(&self, screen: (f32, f32)) {
        if !self.active { return; }
        let Some(line) = self.lines.get(self.current_line) else { return };
        let frame = layout(&line.speaker, &line.text, screen);
        let border = speaker_color(&line.speaker);
        let visible = line.text[..self.char_index.min(line.text.len())].chars().count();
        for el in frame.elements() {
            match (el.id, &el.kind) {
                (Some(DialogueId::Area), _) => {
                    // The box starts halfway down the name tab, which sits
                    // across its top edge.
                    let b = el.rect.inset(0.0, TAB_OVERHANG, 0.0, 0.0);
                    paint::fill(b, Color::from_rgba(20, 20, 40, 230));
                    paint::outline(b, 3.0, border);
                }
                (Some(DialogueId::Tab), _) => paint::fill(el.rect, border),
                (Some(DialogueId::Speaker), Kind::Text(t)) => paint::text(t, Color::from_rgba(26, 26, 46, 255)),
                (Some(DialogueId::Body), Kind::Text(t)) => paint::text_prefix(t, visible, WHITE),
                (Some(DialogueId::Continue), Kind::Text(t)) if self.waiting_for_input && paint::blink(6.0) => {
                    paint::text(t, Color::from_rgba(150, 150, 150, 255));
                }
                _ => {}
            }
        }
    }
}

/// How far the speaker's name tab rises above the box's top edge.
const TAB_OVERHANG: f32 = 18.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueId {
    /// The whole dialogue area: the name tab plus the box under it.
    Area,
    Tab,
    Speaker,
    Body,
    /// The "SPACE >" prompt, in a footer the body can never run into.
    Continue,
}

/// Lay out one dialogue line along the bottom of the screen. Pure. The box is
/// at least as tall as it always was and grows (up to 60% of the screen) for
/// a long line; past that the text shrinks. "SPACE >" has its own reserved
/// footer row.
pub fn layout(speaker: &str, text: &str, screen: (f32, f32)) -> Frame<DialogueId> {
    let (_, sh) = screen;
    let area = col()
        .id(DialogueId::Area)
        .min_h(170.0 + TAB_OVERHANG)
        .max_h((sh * 0.6).max(170.0 + TAB_OVERHANG))
        .min_w(0.0)
        .child(
            row().h(34.0).fixed().child(gap_box(15.0, 0.0)).child(
                row()
                    .id(DialogueId::Tab)
                    .h(34.0)
                    .pad_xy(12.0, 0.0)
                    .shrink(1.0)
                    .child(layout::text(speaker, 26, Fit::shrink(14)).id(DialogueId::Speaker)),
            ),
        )
        .child(
            col()
                .grow(1.0)
                .pad_edges(20.0, 12.0, 20.0, 8.0)
                .gap(6.0)
                .child(layout::text(text, 28, Fit::wrap(16)).line_gap(4.0).id(DialogueId::Body))
                .child(spacer())
                .child(
                    row().justify(Justify::End).fixed().child(
                        layout::text("SPACE >", 20, Fit::shrink(14)).id(DialogueId::Continue),
                    ),
                ),
        );
    let root = col().pad(10.0).justify(Justify::End).child(area);
    layout::layout(&root, layout::screen_rect(screen))
}

fn speaker_color(speaker: &str) -> Color {
    match speaker {
        "Sparky" => Color::from_rgba(0, 230, 118, 255),
        "Mommy" => Color::from_rgba(224, 64, 251, 255),
        "Professor Gizmo" => Color::from_rgba(179, 136, 255, 255),
        "Bolt the Shopkeeper" => Color::from_rgba(255, 183, 77, 255),
        "???" => Color::from_rgba(206, 147, 216, 255),
        "B0RK.exe" => Color::from_rgba(118, 255, 3, 255),
        "Old Oak" => Color::from_rgba(165, 214, 167, 255),
        _ => Color::from_rgba(255, 213, 79, 255),
    }
}
