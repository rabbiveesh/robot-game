# GenAI & Voice — Design Spec

## Problem

Current GenAI usage is naive — one API call per NPC interaction, 1-3 second latency. A 4-year-old will mash space or walk away before the response arrives. Voice output (TTS) exists but voice INPUT doesn't, which is the most natural interface for a kid who can't type.

## Part 1: Low-Latency AI Dialogue

### The Latency Budget

A kid's attention span after pressing Space to talk:
- 0-500ms: acceptable, feels responsive
- 500ms-1.5s: noticeable, kid might press space again
- 1.5s+: kid has walked away, mashed buttons, or talked to a sibling

We need AI dialogue to feel instant. That means it's already generated before the kid asks for it.

### Architecture: Pre-Generation Pipeline

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Generator    │────▶│  Cache       │────▶│  Consumer    │
│  (background) │     │  (per-context)│     │  (instant)   │
└──────────────┘     └──────────────┘     └──────────────┘
     ▲                                          │
     │            refill when low                │
     └──────────────────────────────────────────┘
```

### Cache Structure

Dialogue is cached by context type. Each context type has a pool of pre-generated lines.

```js
DialogueCache {
  pools: {
    'sparky_greeting':     [],  // "Hi boss!" variants
    'sparky_joke':         [],  // random jokes
    'sparky_challenge_intro_math': [],  // "BEEP BOOP! My sensors detect a puzzle!"
    'sparky_correct':      [],  // celebration lines
    'sparky_wrong':        [],  // encouragement lines
    'sparky_frustration':  [],  // "Take your time, boss!"
    'npc_mommy_chat':      [],  // Mommy dialogue
    'npc_gizmo_chat':      [],  // Professor Gizmo dialogue
    'npc_gizmo_challenge': [],  // Gizmo introducing a challenge
    'area_dream':          [],  // dream world ambient lines
    'area_doghouse':       [],  // glitch world lines
  },
  poolSize: 5,    // target lines per pool
  minBefore: 2,   // refill when pool drops below this
}
```

### Generation Strategy

**On session start (after intake):**
- Fire off batch requests for all pools in parallel
- Each request generates 5 lines for that context
- Use a single batch API call where possible (Gemini supports batch, Claude doesn't but we can fire concurrent requests)
- Total: ~12 pools × 5 lines = 60 lines, probably 3-5 API calls with batching
- This runs in the background while the welcome dialogue plays

**During play:**
- When a line is consumed from a pool, check if pool is below `minBefore`
- If so, queue a background refill (non-blocking)
- Consumer always gets a line instantly from the pool
- If pool is empty (shouldn't happen but safety), fall back to hardcoded lines

**Context-aware generation:**
The prompt for each pool includes the kid's name, current area, Dum Dum count, and recent events. Pools are regenerated when context changes significantly (enter new area, major event).

### Batch Prompt Design

Instead of one API call per line, generate multiple lines in one call:

```
System: You are Sparky the robot. Generate 5 different greeting lines for {playerName}.
Each line should be 1-2 sentences. Be silly and fun. Number them 1-5.

Response:
1. BEEP BOOP! Hi {playerName}! I polished my antenna just for you!
2. *bzzt* Hey boss! Did you know robots can dream? I dreamed about DUM DUMS!
3. ...
```

Parse the numbered list, add to pool. One API call, 5 lines. 5x more efficient.

### Provider-Specific Optimization

| Provider | Strategy |
|----------|----------|
| **Anthropic Claude** | Fire parallel requests (one per pool). Use Haiku for speed. ~500ms per request. |
| **Google Gemini** | Use batch API. Single request with multiple prompts. Free tier: 15 RPM, so batch is critical. |
| **None (no API key)** | Hardcoded fallback lines only. Already works. |

### Fallback Chain

```
1. Check cache pool → instant
2. Pool empty? Check stale pool (lines from previous session, still valid) → instant
3. Stale empty? Use hardcoded fallback → instant
4. Queue background refill for next time
```

The kid NEVER waits for an API call. Ever.

### Cache Persistence

Cache pools are stored in sessionStorage (not localStorage — they're ephemeral):
- Survives page refresh within a session
- Cleared on new session (lines might be stale)
- ~20KB total for 60 lines, well within limits

### Domain Events for Cache

```js
DialogueCacheRefill {
  type: 'DIALOGUE_CACHE_REFILL',
  pool: 'sparky_greeting',
  linesGenerated: 5,
  provider: 'gemini',
  latencyMs: 800,
}

DialogueCacheMiss {
  type: 'DIALOGUE_CACHE_MISS',
  pool: 'sparky_joke',
  fallbackUsed: true,
}
```

These feed into the parent dashboard — "AI dialogue is working, cache hit rate is 94%."

## Part 2: Voice Input

### Why This Matters

A 4-year-old can say "thirteen" but can't type "13" or reliably click a small button. Voice input is the most natural answer mode for young kids. It's also the richest signal — how a kid says an answer (confident vs hesitant, fast vs slow, asking "is it thirteen?" vs stating "thirteen!") reveals things clicking a button can't.

### Web Speech API

The browser has built-in speech recognition:

```js
const recognition = new (window.SpeechRecognition || window.webkitSpeechRecognition)();
recognition.lang = 'en-US';
recognition.continuous = false;
recognition.interimResults = true;

recognition.onresult = (event) => {
  const transcript = event.results[0][0].transcript;
  const confidence = event.results[0][0].confidence;
  // "thirteen" → 13
};
```

**Browser support:** Chrome (desktop + Android), Edge, Safari (iOS 14.5+). Firefox: no. Good enough for a family game.

### Voice as Answer Mode

Voice input becomes a new position on the answer mode axis:

```
choice → eliminate → free input → voice → drag/build
```

Wait — actually voice isn't "harder" than free input. It's a parallel mode, not a higher rung. A kid who can say "thirteen" might not be able to type it, and vice versa. Voice should be available alongside other modes, not instead of them.

Better model:

```
Answer method (independent axis):
  ├── click (choices, eliminate)
  ├── type (keyboard number input)
  ├── voice (speak the answer)
  └── build (drag objects)
```

The self-selection model from the interaction spec applies: "How do you want to answer? Pick it / Type it / Say it / Count it out"

### Number Recognition

The hard part: mapping spoken words to numbers.

```js
function parseSpokenNumber(transcript) {
  const clean = transcript.toLowerCase().trim();

  // Direct number words
  const words = {
    'zero': 0, 'one': 1, 'two': 2, 'three': 3, 'four': 4,
    'five': 5, 'six': 6, 'seven': 7, 'eight': 8, 'nine': 9,
    'ten': 10, 'eleven': 11, 'twelve': 12, 'thirteen': 13,
    'fourteen': 14, 'fifteen': 15, 'sixteen': 16, 'seventeen': 17,
    'eighteen': 18, 'nineteen': 19, 'twenty': 20,
    'thirty': 30, 'forty': 40, 'fifty': 50, 'sixty': 60,
    'seventy': 70, 'eighty': 80, 'ninety': 90, 'hundred': 100,
  };

  // Try exact match first
  if (words[clean] !== undefined) return words[clean];

  // Try compound: "twenty three" → 23
  // Try digit string: "one three" → 13 or "1 3" → 13
  // Try numeric: "13" → 13
  // ...

  return null; // couldn't parse
}
```

**Handling kid speech patterns:**
- "ummm... thirteen?" → strip filler words, strip question mark tone
- "I think it's thirteen" → extract the number
- "thirteen! No wait, twelve!" → use the last number spoken
- Unintelligible → "I didn't catch that, can you say it again?" (not counted as wrong)

### Voice Command Recognition

Beyond numbers, voice can handle commands:

| Kid says | Action |
|----------|--------|
| "Show me" / "Help" / "I don't know" | Trigger show-me hint |
| "Tell me" / "What is it" | Trigger tell-me |
| "Talk to Sparky" / "Hey Sparky" | Initiate interaction |
| A number | Submit as answer |

### Confidence Threshold

The SpeechRecognition API returns a confidence score (0-1). We use it:

```
confidence > 0.8:  Accept the answer, submit it
confidence 0.5-0.8: "Did you say thirteen?" — confirm before submitting
confidence < 0.5:  "I didn't catch that, say it again?"
```

This prevents misrecognition from counting as wrong answers. The kid is never penalized for unclear speech.

### Signal Extraction from Voice

```js
VoiceAnswerEvent {
  type: 'VOICE_ANSWER',
  transcript: 'thirteen',
  parsedNumber: 13,
  confidence: 0.92,
  hesitation: false,       // did they pause or say "um" before the number?
  selfCorrected: false,    // did they change their answer mid-speech?
  responseTimeMs: 3200,    // time from mic activation to final answer
  correct: true,
}
```

| Signal | What it indicates |
|--------|------------------|
| High confidence, fast, no hesitation | Fluent at this level |
| Hesitation then correct | Thinking it through — right level of challenge |
| Self-correction (wrong → right) | Good metacognition — caught their own error |
| Self-correction (right → wrong) | Second-guessing — might be anxious |
| Low confidence from recognition | Kid's speech unclear — not a math signal, don't adapt |

### Conversational Mode (Future)

The ultimate vision: kid talks to Sparky naturally, Sparky responds with AI dialogue + AI voice, and math happens in the conversation.

```
Kid: "Hey Sparky, what should we do?"
Sparky (AI): "BEEP BOOP! I found a treasure chest! But it has a lock!
             It says we need to figure out 8 plus 5!"
Kid: "Thirteen!"
Sparky (AI): "YESSS! You're so smart! *chest opens* There's a Dum Dum inside!"
```

This requires:
1. Speech recognition (this spec)
2. AI dialogue generation (Part 1 of this spec)
3. AI voice (ElevenLabs, already implemented)
4. Orchestration: recognize → generate response → speak response

Latency budget for conversational mode:
- Recognition: ~500ms
- AI generation: ~0 (pre-cached for expected responses)
- AI voice synthesis: ~500ms (ElevenLabs turbo)
- Total: ~1 second. Acceptable if we fill the gap with Sparky "thinking" animation.

The trick: pre-generate responses for EXPECTED answers. If the question is 8+5, we pre-generate responses for "correct (13)", "common wrong answers (12, 14)", and "I don't know." When the kid speaks, we match to the closest pre-generated response and play it instantly.

```js
PreGeneratedResponses {
  correct: "YESSS! 13! You're a genius, boss!",
  wrong_close: "Hmm, almost! That's really close though!",
  wrong_far: "BZZZT! My circuits say that's not quite right...",
  dont_know: "No worries boss! Want me to show you?",
  unclear: "Beep boop? My microphone is fuzzy! Say that again?",
}
```

5 pre-generated responses per question. Generated in the background while the question is being presented (we know the question before the kid answers, so we have 3-10 seconds to generate while they think).

## Part 3: Implementation Plan

### Phase 1: Cache pipeline (no new UX, invisible to kid)

- DialogueCache module in infrastructure layer
- Batch prompt generation (5 lines per call)
- Background refill on pool depletion
- Fallback chain (cache → stale → hardcoded)
- Cache in sessionStorage
- Wire into existing dialogue system (replace current single-call prefetch)

### Phase 2: Voice answer mode

- SpeechRecognition integration
- Number parser (spoken word → integer)
- Confidence thresholds with confirmation
- "Say it" as a self-selected answer mode option
- Voice signal extraction events
- Microphone permission handling (only ask when kid selects voice mode)

### Phase 3: Voice commands

- Recognize "show me" / "tell me" / "help" alongside numbers
- "Hey Sparky" as interaction trigger (optional, for hands-free play)

### Phase 4: Conversational mode

- Pre-generate expected response set per question
- Orchestrate: listen → match → play pre-generated response
- Sparky "thinking" animation to fill latency gaps
- Full speech-to-speech loop with AI dialogue + AI voice

## Architecture

### Where it lives

```
src/
  infrastructure/
    dialogue-cache.js      # Cache pipeline, batch generation, pool management
    speech-recognition.js  # Voice input, number parsing, confidence handling
    claude-dialogue.js     # (existing) Anthropic API
    gemini-dialogue.js     # (existing) Gemini API
    speech-service.js      # (existing) TTS output
    elevenlabs-service.js  # (existing) ElevenLabs output
```

The cache and speech recognition are infrastructure — they don't know about the learning domain. The application layer orchestrates: "kid spoke '13'" → creates a `PUZZLE_ATTEMPTED` event with `answerMode: 'voice'` → reducer processes it the same as a click.

### Domain impact

Minimal. The reducer already handles `PUZZLE_ATTEMPTED` events. Voice just adds new fields:

```js
{
  type: 'PUZZLE_ATTEMPTED',
  correct: true,
  // ... existing fields ...
  answerMode: 'voice',         // NEW
  voiceConfidence: 0.92,       // NEW
  voiceHesitation: false,      // NEW
  voiceSelfCorrected: false,   // NEW
}
```

The reducer can optionally use these for profile adaptation (e.g., a kid who's confident in voice mode but hesitant in click mode → their voice CRA stage might be different from their click CRA stage). But this is a future refinement — for now, `answerMode: 'voice'` is just recorded.

## Open Questions

- **Privacy:** Speech recognition in Chrome sends audio to Google's servers for processing. Should we disclose this? Is it a dealbreaker for some parents? Safari does on-device recognition — should we prefer Safari's API when available?
- **Accents and kid speech:** Recognition accuracy drops significantly for young children. Should we offer a "voice calibration" where the kid says numbers 1-20 and we learn their pronunciation patterns?
- **Background noise:** Kids play in noisy environments. Should the mic have a noise gate? Or just rely on the confidence threshold?
- **ElevenLabs rate limits:** Free tier is 10,000 chars/month. At ~50 chars per line and ~20 lines per session, that's 1,000 chars/session = 10 sessions/month. Probably enough for testing but not for daily use. Should we track char usage and fall back to browser TTS before hitting the limit?
- **Gemini rate limits:** Free tier is 15 RPM. Batch generation helps (one call = 5 lines) but a session-start burst of 12 pools could hit the limit. Should we stagger the initial generation?
