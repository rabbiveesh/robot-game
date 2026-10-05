//! Per-frame input snapshot.
//!
//! In production, `FrameInput::capture()` reads from macroquad once per frame.
//! In tests, build one with the `with_*` builders. UI and game code only
//! read from this struct — they never touch macroquad input directly.

use std::collections::HashSet;
use macroquad::prelude::{
    KeyCode, MouseButton,
    get_char_pressed, is_key_down, is_key_pressed,
    is_mouse_button_down, is_mouse_button_pressed, is_mouse_button_released,
    mouse_position,
};

/// Every key the game cares about. Capture polls macroquad for each.
const TRACKED_KEYS: &[KeyCode] = &[
    KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right,
    KeyCode::W, KeyCode::A, KeyCode::S, KeyCode::D,
    KeyCode::Space, KeyCode::Enter,
    KeyCode::Backspace, KeyCode::Tab, KeyCode::Escape,
    KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4,
    KeyCode::P, KeyCode::T, KeyCode::E,
    KeyCode::GraveAccent, // backtick — toggles the dev debug/export overlay
];

#[derive(Default, Clone)]
pub struct FrameInput {
    keys_pressed: HashSet<KeyCode>,
    keys_down: HashSet<KeyCode>,
    pub mouse_pos: (f32, f32),
    /// Left button (or finger) went down this frame.
    pub mouse_clicked: bool,
    /// Left button (or finger) is held — true on the click frame too. Drags
    /// read this; plain taps only need `mouse_clicked`.
    pub mouse_down: bool,
    /// Left button (or finger) came up this frame — where a drag drops.
    pub mouse_released: bool,
    pub chars_typed: Vec<char>,
    /// Wall clock, Unix seconds. Only for stamping records (the attempt log);
    /// nothing in play reads it. 0 in tests unless set.
    pub now: f64,
}

#[allow(dead_code)] // builders are for the upcoming test harness
impl FrameInput {
    /// Empty frame — no keys, no clicks. Default starting point for tests.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Snapshot all input from macroquad for the current frame.
    pub fn capture() -> Self {
        let mut input = FrameInput::default();
        for &k in TRACKED_KEYS {
            if is_key_pressed(k) { input.keys_pressed.insert(k); }
            if is_key_down(k) { input.keys_down.insert(k); }
        }
        input.mouse_pos = mouse_position();
        input.mouse_clicked = is_mouse_button_pressed(MouseButton::Left);
        input.mouse_down = is_mouse_button_down(MouseButton::Left);
        input.mouse_released = is_mouse_button_released(MouseButton::Left);
        input.now = macroquad::miniquad::date::now();
        while let Some(c) = get_char_pressed() {
            input.chars_typed.push(c);
        }
        input
    }

    pub fn pressed(&self, k: KeyCode) -> bool {
        self.keys_pressed.contains(&k)
    }

    pub fn down(&self, k: KeyCode) -> bool {
        self.keys_down.contains(&k)
    }

    // ─── Test builders ──────────────────────────────────────

    pub fn with_key_pressed(mut self, k: KeyCode) -> Self {
        self.keys_pressed.insert(k);
        self
    }

    pub fn with_key_down(mut self, k: KeyCode) -> Self {
        self.keys_down.insert(k);
        self
    }

    pub fn with_mouse_at(mut self, x: f32, y: f32) -> Self {
        self.mouse_pos = (x, y);
        self
    }

    pub fn with_mouse_click(mut self, x: f32, y: f32) -> Self {
        self.mouse_pos = (x, y);
        self.mouse_clicked = true;
        self.mouse_down = true;
        self
    }

    /// Button held with the pointer at (x, y) — the middle frames of a drag.
    pub fn with_mouse_held(mut self, x: f32, y: f32) -> Self {
        self.mouse_pos = (x, y);
        self.mouse_down = true;
        self
    }

    /// Button let go at (x, y) — the last frame of a drag.
    pub fn with_mouse_release(mut self, x: f32, y: f32) -> Self {
        self.mouse_pos = (x, y);
        self.mouse_released = true;
        self
    }

    pub fn with_now(mut self, unix_secs: f64) -> Self {
        self.now = unix_secs;
        self
    }

    pub fn with_char(mut self, c: char) -> Self {
        self.chars_typed.push(c);
        self
    }
}
