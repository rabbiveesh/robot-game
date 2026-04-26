/// Global game settings — read by TTS and dialogue, edited by the settings overlay.
/// Single-threaded macroquad; atomics keep access lock-free and thread-safe anyway.
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

static TTS_ENABLED: AtomicBool = AtomicBool::new(true);
static TEXT_SPEED: AtomicU8 = AtomicU8::new(1); // 0 = slow, 1 = normal, 2 = fast

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextSpeed {
    Slow,
    Normal,
    Fast,
}

impl TextSpeed {
    fn from_raw(v: u8) -> Self {
        match v {
            0 => TextSpeed::Slow,
            2 => TextSpeed::Fast,
            _ => TextSpeed::Normal,
        }
    }
    fn to_raw(self) -> u8 {
        match self {
            TextSpeed::Slow => 0,
            TextSpeed::Normal => 1,
            TextSpeed::Fast => 2,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            TextSpeed::Slow => "Slow",
            TextSpeed::Normal => "Normal",
            TextSpeed::Fast => "Fast",
        }
    }
}

pub fn tts_enabled() -> bool {
    TTS_ENABLED.load(Ordering::Relaxed)
}

pub fn set_tts_enabled(v: bool) {
    TTS_ENABLED.store(v, Ordering::Relaxed);
}

pub fn toggle_tts() {
    let cur = tts_enabled();
    set_tts_enabled(!cur);
}

pub fn text_speed() -> TextSpeed {
    TextSpeed::from_raw(TEXT_SPEED.load(Ordering::Relaxed))
}

pub fn set_text_speed(s: TextSpeed) {
    TEXT_SPEED.store(s.to_raw(), Ordering::Relaxed);
}

/// Seconds per character for the dialogue typewriter.
pub fn char_speed_seconds() -> f32 {
    match text_speed() {
        TextSpeed::Slow => 0.06,
        TextSpeed::Normal => 0.03,
        TextSpeed::Fast => 0.012,
    }
}
