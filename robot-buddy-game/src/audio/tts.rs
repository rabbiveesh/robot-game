/// Text-to-speech — browser SpeechSynthesis on WASM, no-op on native.

/// Per-speaker voice config: pitch and rate as 0-200 scale (100 = normal).
struct VoiceConfig {
    pitch: u32,
    rate: u32,
}

fn voice_for_speaker(speaker: &str) -> VoiceConfig {
    match speaker {
        "Sparky"              => VoiceConfig { pitch: 160, rate: 110 },
        "Mommy"               => VoiceConfig { pitch: 120, rate: 95 },
        "Professor Gizmo"     => VoiceConfig { pitch: 80, rate: 85 },
        "Bolt the Shopkeeper" => VoiceConfig { pitch: 100, rate: 105 },
        "???"                 => VoiceConfig { pitch: 70, rate: 70 },
        "B0RK.exe"            => VoiceConfig { pitch: 180, rate: 130 },
        "Old Oak"             => VoiceConfig { pitch: 60, rate: 70 },
        _                     => VoiceConfig { pitch: 100, rate: 100 },
    }
}

/// Clean text for speech: convert math symbols, strip emoji/markdown.
fn clean_for_speech(text: &str) -> String {
    text.replace('\u{00d7}', "times")
        .replace('\u{00f7}', "divided by")
        .replace('+', " plus ")
        .replace(" - ", " minus ")
        .replace('\u{2212}', "minus")
        .replace(|c: char| "\u{1F916}\u{1F680}\u{2B50}\u{1F31F}\u{1F36D}\u{1F4CD}#".contains(c), "")
}

/// Speak a line of dialogue. No-op on native (no browser speech API).
pub fn speak(speaker: &str, text: &str) {
    let clean = clean_for_speech(text);
    if clean.trim().is_empty() { return; }
    let voice = voice_for_speaker(speaker);
    platform_speak(&clean, voice.pitch, voice.rate);
}

pub fn cancel() {
    platform_cancel();
}

// ─── PLATFORM ───────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn tts_speak(text_ptr: *const u8, text_len: usize, pitch100: u32, rate100: u32);
    fn tts_cancel();
}

#[cfg(target_arch = "wasm32")]
fn platform_speak(text: &str, pitch: u32, rate: u32) {
    unsafe { tts_speak(text.as_ptr(), text.len(), pitch, rate); }
}

#[cfg(target_arch = "wasm32")]
fn platform_cancel() {
    unsafe { tts_cancel(); }
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_speak(_text: &str, _pitch: u32, _rate: u32) {
    // No-op on native — TTS is browser-only for now
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_cancel() {}
