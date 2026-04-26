# Voice Input Spec

Add "Say it" as an answer mode. Kid taps a mic button, says a number, it gets recognized and submitted.

Status: not implemented in macroquad. Domain support exists (`ChallengeAction::VoiceResult { number, confidence }`). For platform implementation see `native-speech-spec.md`.

## Scope

- Mic button on the challenge screen.
- Number recognition: spoken word/digits → integer 0–144 (covers division answers at band 10).
- Confidence handling: confirm if uncertain, retry if unintelligible.
- Signal extraction: hesitation, self-correction, response time.

NOT in scope: voice commands ("show me", "hey Sparky"), conversational mode, raw audio analysis.

## Number parser contract

The parser is a pure function `&str → Option<u32>`. Must handle:

| Input                        | Output |
|------------------------------|--------|
| `"thirteen"`                 | 13     |
| `"13"`                       | 13     |
| `"twenty three"`             | 23     |
| `"twenty-three"`             | 23     |
| `"one hundred"`              | 100    |
| `"a hundred"`                | 100    |
| `"one hundred and 44"`       | 144    |
| `"umm thirteen"`             | 13     |
| `"I think it's thirteen"`    | 13     |
| `"thirteen no twelve"`       | 12     (last number wins — self-correction) |
| `"firteen"`                  | None   (can't parse — ask again, not wrong)  |
| `""`                         | None   |

Filler stripping: `um`, `uh`, `umm`, `erm`, `like`, `i think`, `it's`, `its`, `is it`, `maybe`.

## Recognition result shape

```rust
pub struct VoiceResult {
    pub transcript: String,
    pub number: Option<u32>,
    pub confidence: f32,
    pub hesitation_ms: u32,    // mic on → first speech
    pub total_ms: u32,         // mic on → final result
    pub self_corrected: bool,  // interim transcript changed direction
    pub had_filler_words: bool,
}
```

## UX

```
┌──────────────────────────────────────────┐
│  Sparky: How many bolts do I have?       │
│                                          │
│    [ 12 ]    [ 13 ]    [ 14 ]            │
│                                          │
│    💡 Show me!    🤷 Tell me!    🎤 Say it│
└──────────────────────────────────────────┘
```

1. Tap 🎤 → button pulses to show "listening".
2. After recognition:
   - confidence > 0.8 → show recognized number ("You said: 13!") then submit.
   - 0.5–0.8 → "Did you say thirteen?" with Yes/No.
   - < 0.5 or unparseable → "I didn't catch that! Try again?" with retry button.
3. Timeout (10s, no speech) → "I didn't hear anything. Try again or pick an answer?"
4. Permission prompt only on first mic tap. If denied, hide the mic button.

If interim results are available, show partial transcript live: `🎤 "thir..." → 🎤 "thirteen" ✓`.

Sparky introduces voice on first availability: "Ooh! You want to TALK to me? Press the microphone and say the number! Beep boop!"

## Event signal

When the kid answers via voice, the resulting `PUZZLE_ATTEMPTED` event carries:

```rust
answer_mode: AnswerMode::Voice,
voice_confidence: f32,
voice_hesitation_ms: u32,
voice_self_corrected: bool,
voice_had_fillers: bool,
voice_retries: u32,
```

The reducer records these. Future use: per-operation hesitation patterns are a CRA/difficulty signal.

## Acceptance criteria

1. 🎤 button appears when the platform supports speech recognition.
2. Tapping mic starts listening with visual feedback.
3. Numbers 0–144 are correctly recognized and submitted.
4. Low confidence triggers confirmation, not auto-submit.
5. Unparseable speech triggers retry, never a wrong-answer event.
6. 10s timeout prompts retry or fallback to clicking.
7. Permission prompt only on first mic tap.
8. Parser contract test cases above all pass.
9. Events include voice signal fields.
