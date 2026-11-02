//! Per-frame input snapshot.
//!
//! In production, `crate::app` fills a `FrameInput` from Bevy's input
//! resources once per frame. In tests, build one with the `with_*` builders.
//! UI and game code only read from this struct — they never touch Bevy input
//! directly.

use std::collections::HashSet;

use crate::ui::rect::UiRect;
use crate::ui::{ArtId, UiAction};

/// The keys the game reads. Names follow the physical key (`Key1` is the `1`
/// key on the top row).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Up, Down, Left, Right,
    W, A, S, D, E, P, T,
    Space, Enter, Backspace, Tab, Escape,
    LeftShift, RightShift,
    Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    /// Backtick: toggles the dev debug/export overlay.
    GraveAccent,
}

impl KeyCode {
    /// The game key for a Bevy key, if the game reads it. Numpad digits count
    /// as digits and Numpad Enter as Enter.
    pub fn from_bevy(k: bevy::input::keyboard::KeyCode) -> Option<KeyCode> {
        use bevy::input::keyboard::KeyCode as B;
        Some(match k {
            B::ArrowUp => KeyCode::Up,
            B::ArrowDown => KeyCode::Down,
            B::ArrowLeft => KeyCode::Left,
            B::ArrowRight => KeyCode::Right,
            B::KeyW => KeyCode::W,
            B::KeyA => KeyCode::A,
            B::KeyS => KeyCode::S,
            B::KeyD => KeyCode::D,
            B::KeyE => KeyCode::E,
            B::KeyP => KeyCode::P,
            B::KeyT => KeyCode::T,
            B::Space => KeyCode::Space,
            B::Enter | B::NumpadEnter => KeyCode::Enter,
            B::Backspace => KeyCode::Backspace,
            B::Tab => KeyCode::Tab,
            B::Escape => KeyCode::Escape,
            B::ShiftLeft => KeyCode::LeftShift,
            B::ShiftRight => KeyCode::RightShift,
            B::Digit0 | B::Numpad0 => KeyCode::Key0,
            B::Digit1 | B::Numpad1 => KeyCode::Key1,
            B::Digit2 | B::Numpad2 => KeyCode::Key2,
            B::Digit3 | B::Numpad3 => KeyCode::Key3,
            B::Digit4 | B::Numpad4 => KeyCode::Key4,
            B::Digit5 | B::Numpad5 => KeyCode::Key5,
            B::Digit6 | B::Numpad6 => KeyCode::Key6,
            B::Digit7 | B::Numpad7 => KeyCode::Key7,
            B::Digit8 | B::Numpad8 => KeyCode::Key8,
            B::Digit9 | B::Numpad9 => KeyCode::Key9,
            B::Backquote => KeyCode::GraveAccent,
            _ => return None,
        })
    }
}

#[derive(Default, Clone)]
pub struct FrameInput {
    keys_pressed: HashSet<KeyCode>,
    keys_down: HashSet<KeyCode>,
    pub mouse_pos: (f32, f32),
    /// A mouse press or a finger landing this frame, at `mouse_pos`.
    pub mouse_clicked: bool,
    /// Left button (or finger) is held — true on the click frame too. Drags
    /// read this; plain taps only need `mouse_clicked`.
    pub mouse_down: bool,
    /// Left button (or finger) came up this frame — where a drag drops.
    pub mouse_released: bool,
    pub chars_typed: Vec<char>,
    /// The panel button tapped this frame, if any. Bevy UI hit-tests the
    /// nodes it drew; the game only ever sees which button it was.
    pub ui_press: Option<UiAction>,
    /// True when this frame's click landed on a panel (button or not), so
    /// the map doesn't also take it as click-to-walk.
    pub click_on_ui: bool,
    /// Panel art slots on screen last frame, where they sat (logical pixels).
    pub art: Vec<(ArtId, UiRect)>,
    /// Wall clock, Unix seconds. Only for stamping records (the attempt log);
    /// nothing in play reads it. 0 in tests unless set.
    pub now: f64,
}

impl FrameInput {
    /// Empty frame — no keys, no clicks. Default starting point for tests.
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn pressed(&self, k: KeyCode) -> bool {
        self.keys_pressed.contains(&k)
    }

    pub fn down(&self, k: KeyCode) -> bool {
        self.keys_down.contains(&k)
    }

    // ─── Builders (capture + tests) ─────────────────────────

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

    /// A tap on a panel button.
    pub fn with_ui_press(mut self, action: UiAction) -> Self {
        self.ui_press = Some(action);
        self.mouse_clicked = true;
        self.click_on_ui = true;
        self
    }

    /// The size of panel art slot `id`, if it was on screen.
    pub fn art_size(&self, id: ArtId) -> Option<(f32, f32)> {
        self.art_rect(id).map(|r| (r.w, r.h))
    }

    /// Where panel art slot `id` sat on screen last frame — for art the kid
    /// drags things around in.
    pub fn art_rect(&self, id: ArtId) -> Option<UiRect> {
        self.art.iter().find(|(a, _)| *a == id).map(|&(_, r)| r)
    }

    /// Art slot `id` laid out at `rect` (tests; the app fills these itself).
    pub fn with_art(mut self, id: ArtId, rect: UiRect) -> Self {
        self.art.retain(|(a, _)| *a != id);
        self.art.push((id, rect));
        self
    }

    pub fn with_char(mut self, c: char) -> Self {
        self.chars_typed.push(c);
        self
    }
}
