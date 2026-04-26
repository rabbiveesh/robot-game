# Native Speech Spec

TTS and speech recognition for native (non-WASM) builds. Currently these are browser-only via SpeechSynthesis/SpeechRecognition web APIs. This spec covers adding native platform support so the desktop build isn't silent.

Schedule: post-migration. Low urgency — the game is primarily played in-browser. But the native build is great for dev and local playtesting, and a talking Sparky is much better than a silent one.

## Current State

### TTS (Text-to-Speech)

- **WASM**: Works. `extern "C" { fn tts_speak(...) }` calls a miniquad plugin that invokes `window.speechSynthesis`. Per-speaker pitch/rate config in `audio/tts.rs`.
- **Native**: No-op. `platform_speak` is an empty function behind `#[cfg(not(target_arch = "wasm32"))]`.

### Speech Recognition (Voice Input)

- **WASM**: Specced in `docs/voice-input-impl-spec.md`. Domain support exists (`ChallengeAction::VoiceResult`, confidence handling). Browser implementation was in legacy JS `speech-recognition.js`. Not yet ported to macroquad.
- **Native**: Nothing.

## TTS: Native Implementation

### macOS: NSSpeechSynthesizer / AVSpeechSynthesizer

macOS has built-in speech synthesis. Two options:

**Option A: `say` command** (simplest, good enough for dev)

```rust
#[cfg(not(target_arch = "wasm32"))]
fn platform_speak(text: &str, pitch: u32, rate: u32) {
    let rate_wpm = (rate as f32 / 100.0 * 175.0) as u32; // 175 WPM default
    std::process::Command::new("say")
        .arg("-r").arg(rate_wpm.to_string())
        .arg(text)
        .spawn()
        .ok();
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_cancel() {
    std::process::Command::new("killall").arg("say").spawn().ok();
}
```

Limitations: `say` doesn't support pitch directly (would need `-v` voice selection). Blocking workaround: `spawn()` runs async, but overlapping calls stack up. The `cancel()` via `killall say` is crude but works for dev.

**Option B: `tts` crate** (cross-platform, production-quality)

The `tts` crate (https://crates.io/crates/tts) wraps platform speech APIs:
- macOS: AVSpeechSynthesizer
- Windows: SAPI / OneCore
- Linux: speech-dispatcher

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tts = "0.26"
```

```rust
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use once_cell::sync::Lazy;

#[cfg(not(target_arch = "wasm32"))]
static TTS_ENGINE: Lazy<Mutex<tts::Tts>> = Lazy::new(|| {
    Mutex::new(tts::Tts::default().expect("Failed to init TTS"))
});

#[cfg(not(target_arch = "wasm32"))]
fn platform_speak(text: &str, pitch: u32, rate: u32) {
    if let Ok(mut engine) = TTS_ENGINE.lock() {
        let _ = engine.set_rate((rate as f32 / 100.0 - 1.0) * 0.5); // normalize
        let _ = engine.set_pitch((pitch as f32 / 100.0 - 1.0) * 0.5);
        let _ = engine.speak(text, true); // true = interrupt previous
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_cancel() {
    if let Ok(mut engine) = TTS_ENGINE.lock() {
        let _ = engine.stop();
    }
}
```

**Recommendation**: Start with Option A (`say` command) for immediate dev feedback. Switch to Option B (`tts` crate) if/when we want Windows/Linux support or need pitch control.

## Speech Recognition: Native Implementation

Native speech recognition is harder than TTS. Options:

### macOS: SFSpeechRecognizer

Apple's Speech framework provides on-device recognition. Requires:
- Privacy permission (microphone + speech recognition)
- Objective-C/Swift bridging via `objc` crate or a helper binary

Too heavy for the current scope. Defer.

### Cross-platform: Whisper via `whisper-rs`

OpenAI's Whisper model runs locally via `whisper-rs` (bindings to whisper.cpp). Accurate, works offline, no API key.

Considerations:
- Model download: ~75MB for `tiny` model, ~500MB for `base`
- First-time setup: download model on first use
- Latency: `tiny` model transcribes in ~200ms on M1
- Accuracy: good for single numbers/short phrases, which is our use case

This is the right long-term answer for native speech recognition but is a significant integration. Defer until the game is feature-complete.

### Cloud: Whisper API or Google Cloud Speech

Simple HTTP POST with audio data. Requires API key and internet. Not ideal for a kids' game running locally. Avoid unless we have a specific reason.

**Recommendation for speech recognition**: Defer native implementation entirely. The domain support is already there (VoiceResult, confidence handling). When we build it, Whisper via `whisper-rs` is the right path for native. Browser SpeechRecognition covers the WASM case.

## Architecture

The current `audio/tts.rs` pattern is correct — `#[cfg]` gates on `target_arch` with identical public API. The native implementation just changes the `platform_*` functions. No changes to calling code.

For speech recognition, the same pattern works:

```
audio/
  tts.rs               # speak(), cancel() — platform-gated
  speech_recognition.rs # listen(), is_available() — platform-gated (future)
  mod.rs
```

The domain's `ChallengeAction::VoiceResult { number, confidence }` doesn't care where the recognition came from — browser API, Whisper, or a mock in tests.

## Priority

1. **TTS via `say` command** — trivial, immediate value for native dev/playtesting
2. **TTS via `tts` crate** — when targeting Windows/Linux or needing pitch control
3. **Speech recognition via browser API** — port the existing spec to macroquad WASM
4. **Speech recognition via Whisper** — long-term, when the game is feature-complete
