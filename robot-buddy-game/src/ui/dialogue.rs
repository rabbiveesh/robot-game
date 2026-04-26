use macroquad::prelude::*;
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

    pub fn draw(&self) {
        if !self.active { return; }
        let Some(line) = self.lines.get(self.current_line) else { return };

        let sw = screen_width();
        let sh = screen_height();
        let box_h = 170.0;
        let box_y = sh - box_h - 10.0;
        let box_x = 10.0;
        let box_w = sw - 20.0;

        // Background
        draw_rectangle(box_x, box_y, box_w, box_h, Color::from_rgba(20, 20, 40, 230));

        // Border (colored by speaker)
        let border_color = speaker_color(&line.speaker);
        draw_rectangle_lines(box_x, box_y, box_w, box_h, 3.0, border_color);

        // Speaker name tab
        let name_w = line.speaker.len() as f32 * 13.0 + 30.0;
        draw_rectangle(box_x + 15.0, box_y - 18.0, name_w, 34.0, border_color);
        draw_text(&line.speaker, box_x + 27.0, box_y + 6.0, 26.0, Color::from_rgba(26, 26, 46, 255));

        // Text with typewriter effect
        let visible = &line.text[..self.char_index.min(line.text.len())];
        let max_chars = ((box_w - 40.0) / 15.0) as usize;
        let wrapped = word_wrap(visible, max_chars);
        for (i, text_line) in wrapped.iter().enumerate() {
            draw_text(text_line, box_x + 20.0, box_y + 52.0 + i as f32 * 32.0, 28.0, WHITE);
        }

        // "SPACE >" blink indicator
        if self.waiting_for_input {
            let blink = (get_time() * 6.0).sin() > 0.0;
            if blink {
                draw_text("SPACE >", box_x + box_w - 120.0, box_y + box_h - 18.0, 20.0,
                    Color::from_rgba(150, 150, 150, 255));
            }
        }
    }
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

fn word_wrap(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = vec![];
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.len() + word.len() + 1 > max_chars && !current.is_empty() {
            lines.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
