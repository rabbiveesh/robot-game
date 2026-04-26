# GenAI Dialogue Spec

Status: not implemented. `robot-buddy-game/src/net/` is empty. This spec defines the design for low-latency AI dialogue when the feature is built.

For voice input/output, see `voice-input-impl-spec.md` and `native-speech-spec.md`.

## Problem

Naive AI dialogue is one API call per NPC interaction — 1–3 second latency. A 4-year-old will mash space or walk away before the response arrives. We need AI dialogue to feel instant, which means it has to already be generated before the kid asks for it.

### Latency budget

| Window           | Feel                                   |
|------------------|----------------------------------------|
| 0–500 ms         | Acceptable, feels responsive           |
| 500 ms – 1.5 s   | Noticeable, kid might re-press space   |
| > 1.5 s          | Lost — kid mashed buttons or wandered  |

## Architecture: pre-generation pipeline

```
┌──────────────┐     ┌────────────────┐     ┌──────────────┐
│  Generator   │────▶│  Cache         │────▶│  Consumer    │
│  (background)│     │  (per context) │     │  (instant)   │
└──────┬───────┘     └────────────────┘     └──────────────┘
       ▲                                            │
       └────── refill when low ─────────────────────┘
```

The kid never waits for an API call. The consumer always pulls from a pool of pre-generated lines. The generator runs in the background, refilling pools when they drop below a threshold.

## Cache structure

Dialogue is keyed by context type. Each context has a small pool of variants.

| Pool                          | Purpose                                  |
|-------------------------------|------------------------------------------|
| `sparky_greeting`             | "Hi boss!" variants                      |
| `sparky_joke`                 | Random jokes                             |
| `sparky_challenge_intro_math` | "BEEP BOOP! My sensors detect a puzzle!" |
| `sparky_correct`              | Celebration lines                        |
| `sparky_wrong`                | Encouragement lines                      |
| `sparky_frustration`          | "Take your time, boss!"                  |
| `npc_mommy_chat`              | Mommy dialogue                           |
| `npc_gizmo_chat`              | Professor Gizmo dialogue                 |
| `npc_gizmo_challenge`         | Gizmo introducing a challenge            |
| `area_dream`                  | Dream world ambient lines                |
| `area_doghouse`               | Glitch world lines                       |

Tunables: pool target = 5 lines, refill trigger = below 2.

## Generation strategy

**Session start (after intake):** fire batch requests for every pool in parallel. Each request generates 5 lines for that context. ~12 pools × 5 lines ≈ 60 lines, in 3–5 API calls with batching. Runs in the background while the welcome dialogue plays.

**During play:** each consumer pull checks the pool depth. If below the refill trigger, queue a background refill. Consumer always gets a line instantly. If a pool is unexpectedly empty, drop straight to the hardcoded fallback.

**Context-aware generation:** the prompt for each pool includes the kid's name, current area, Dum Dum count, and recent events. Pools are regenerated when context shifts significantly (entering a new area, major event).

## Batch prompts

One API call → many lines. Don't generate one at a time.

```
System: You are Sparky the robot. Generate 5 different greeting lines for {playerName}.
Each line should be 1-2 sentences. Be silly and fun. Number them 1-5.

Response:
1. BEEP BOOP! Hi {playerName}! I polished my antenna just for you!
2. *bzzt* Hey boss! Did you know robots can dream? I dreamed about DUM DUMS!
...
```

Parse the numbered list, push into the pool. Five-for-one per call.

## Provider profile

| Provider           | Strategy                                                          |
|--------------------|-------------------------------------------------------------------|
| Anthropic Claude   | Parallel requests (one per pool). Use Haiku for speed (~500 ms).  |
| Google Gemini      | Batch API. Free tier 15 RPM — batching is critical.               |
| None (no key)      | Hardcoded fallback only. Already works; default at startup.       |

## Fallback chain

```
1. Cache pool                     → instant
2. Stale pool from prior session  → instant
3. Hardcoded fallback line        → instant
4. Queue background refill for next time
```

The kid is never blocked.

## Cache persistence

Pools persist in `localStorage` under a session-scoped key (browser) — survives page refresh within a session, cleared on new session because lines might reference stale context (Dum Dum counts, area names). ~20 KB total for 60 lines.

## Telemetry

Cache events fed into the parent dashboard:

```rust
DialogueCacheRefill {
    pool: String,            // "sparky_greeting"
    lines_generated: u32,    // 5
    provider: String,        // "gemini"
    latency_ms: u32,         // 800
}

DialogueCacheMiss {
    pool: String,            // "sparky_joke"
    fallback_used: bool,     // true
}
```

## Conversational mode (future)

Speech-in → AI → speech-out, with math living inside the conversation.

```
Kid: "Hey Sparky, what should we do?"
Sparky: "BEEP BOOP! I found a treasure chest! It says we need to figure out 8 plus 5!"
Kid: "Thirteen!"
Sparky: "YESSS! You're so smart! *chest opens* There's a Dum Dum inside!"
```

Latency budget for the loop:

| Step                  | Approx |
|-----------------------|--------|
| Recognition           | 500 ms |
| AI generation         | 0      | (pre-cached for expected responses) |
| AI voice synthesis    | 500 ms | (ElevenLabs turbo)                   |
| **Total**             | ~1 s   | (acceptable with a "thinking" animation) |

The trick: pre-generate responses for the *expected* answers to a known question. If the question is `8 + 5`, pre-generate responses for: correct (`13`), common wrong answers (`12`, `14`), `"I don't know"`, and unclear speech. When the kid speaks, match to the closest pre-generated response and play it instantly.

```rust
struct PreGeneratedResponses {
    correct: String,           // "YESSS! 13! You're a genius, boss!"
    wrong_close: String,       // "Hmm, almost! That's really close though!"
    wrong_far: String,         // "BZZZT! My circuits say that's not quite right..."
    dont_know: String,         // "No worries boss! Want me to show you?"
    unclear: String,           // "Beep boop? My microphone is fuzzy! Say that again?"
}
```

5 pre-generated responses per question. The pre-generation runs while the kid is reading/thinking — there are 3–10 seconds before they answer, plenty of headroom for one batch call.

## Implementation order

1. **Cache pipeline** — invisible to the kid, replaces today's hardcoded fallbacks behind the scenes.
2. **Voice answer mode** — see `voice-input-impl-spec.md`.
3. **Voice commands** — recognize `"show me"`/`"tell me"`/`"help"` alongside numbers.
4. **Conversational mode** — the full speech-to-speech loop with pre-generated response sets.

## Where it lives

```
robot-buddy-game/src/net/
  ai_dialogue.rs    # provider client, batch prompt assembly, response parsing
  cache.rs          # pool storage, refill triggers, fallback chain
```

Browser HTTP via `fetch` exposed through a miniquad plugin (same pattern as the localStorage and TTS plugins in `index.html`). Native HTTP via `reqwest` behind `#[cfg(not(target_arch = "wasm32"))]`.

## Open questions

- **Privacy.** Speech recognition in Chrome sends audio to Google. Disclose? Prefer Safari's on-device API when available?
- **Kid speech accuracy.** Recognition drops sharply for young children. Worth a "voice calibration" where the kid says digits 1–20 and we learn their pronunciation?
- **Background noise.** Noise gate, or rely on the confidence threshold?
- **ElevenLabs limits.** Free tier 10k chars/month ≈ 10 sessions. Track usage and fall back to browser TTS before the cap?
- **Gemini rate limits.** Free tier 15 RPM. Batching helps, but a startup burst of 12 pools might trip the limit. Stagger?
