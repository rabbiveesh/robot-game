//! Per-frame input snapshot.
//!
//! In production, `FrameInput::capture()` reads from macroquad once per frame.
//! In tests, build one with the `with_*` builders. UI and game code only
//! read from this struct — they never touch macroquad input directly.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use macroquad::prelude::{
    KeyCode, MouseButton,
    get_char_pressed, is_key_down, is_key_pressed,
    is_mouse_button_down, is_mouse_button_pressed, mouse_position,
};

// Edge detection state for the left mouse button. Why we maintain this in
// addition to macroquad's `is_mouse_button_pressed`: on native (macOS in
// particular) the `pressed` flag occasionally misses a click — the press
// event lands during `next_frame().await` but the flag is cleared before our
// next `capture()` samples it. Tracking the previous frame's
// `is_mouse_button_down` lets us synthesize our own "rising edge" signal and
// OR it with macroquad's, so a click only goes missing if BOTH signals miss
// it. Single-threaded callers only.
static PREV_MOUSE_DOWN: AtomicBool = AtomicBool::new(false);

/// Every key the game cares about. Capture polls macroquad for each.
const TRACKED_KEYS: &[KeyCode] = &[
    KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right,
    KeyCode::W, KeyCode::A, KeyCode::S, KeyCode::D,
    KeyCode::Space, KeyCode::Enter,
    KeyCode::Backspace, KeyCode::Tab, KeyCode::Escape,
    KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4,
    KeyCode::P, KeyCode::T, KeyCode::E,
];

#[derive(Default, Clone)]
pub struct FrameInput {
    keys_pressed: HashSet<KeyCode>,
    keys_down: HashSet<KeyCode>,
    pub mouse_pos: (f32, f32),
    pub mouse_clicked: bool,
    pub chars_typed: Vec<char>,
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
        let down = is_mouse_button_down(MouseButton::Left);
        let prev = PREV_MOUSE_DOWN.swap(down, Ordering::Relaxed);
        let edge_pressed = down && !prev;
        input.mouse_clicked = edge_pressed || is_mouse_button_pressed(MouseButton::Left);
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
        self
    }

    pub fn with_char(mut self, c: char) -> Self {
        self.chars_typed.push(c);
        self
    }
}
