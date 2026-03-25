# Voice Input — Quick Implementation Spec

## Goal

Add "Say it" as an answer mode. Kid taps a mic button, says a number, it gets recognized and submitted. Ship fast, test with real kids.

## Scope

- Mic button on challenge screen
- Number recognition (spoken word → integer)
- Confidence handling (confirm if uncertain, retry if unintelligible)
- Signal extraction (hesitation, self-correction, response time)
- Works in Chrome/Edge/Safari. Graceful degradation elsewhere.

NOT in scope: voice commands ("show me", "hey Sparky"), conversational mode, raw audio analysis.

## File

```
src/infrastructure/speech-recognition.js
```

Zero domain dependencies. Exports functions the adapter/presentation can call. Returns parsed results as plain objects.

## API

```js
// Check if voice input is available
export function isVoiceAvailable() {
  return !!(window.SpeechRecognition || window.webkitSpeechRecognition);
}

// Start listening. Returns a promise that resolves with the result.
// Rejects on error or timeout.
export function listenForNumber(options = {}) {
  // options.timeoutMs: max listen time (default 10000)
  // options.lang: language (default 'en-US')
  //
  // Returns: {
  //   transcript: 'thirteen',
  //   number: 13,              // parsed, or null if unparseable
  //   confidence: 0.92,
  //   alternatives: [{ transcript: 'thirty', number: 30, confidence: 0.04 }],
  //   hesitationMs: 1200,      // time from mic on to first speech
  //   totalMs: 3400,           // time from mic on to final result
  //   selfCorrected: false,    // did interim results change direction
  //   hadFillerWords: false,   // "um", "uh", "like", "I think"
  //   raw: event.results,      // full API result for debugging
  // }
}

// Stop listening early
export function stopListening()

// Parse a transcript string into a number
// Exported separately for unit testing
export function parseSpokenNumber(transcript) {
  // Returns number or null
}
```

## Number Parser

Must handle:

```
"thirteen"           → 13
"13"                 → 13
"twenty three"       → 23
"twenty-three"       → 23
"one hundred"        → 100
"a hundred"          → 100
"one hundred and 44" → 144    (for division answers at band 10)
"umm thirteen"       → 13    (strip fillers)
"I think it's thirteen" → 13 (extract the number)
"thirteen no twelve" → 12    (last number wins — self-correction)
"firteen"            → null  (can't parse — ask again, not wrong)
```

The parser is the most important thing to get right. It's pure function, fully unit testable.

```js
// Core word-to-number map
const ONES = { zero: 0, one: 1, two: 2, ..., nineteen: 19 };
const TENS = { twenty: 20, thirty: 30, ..., ninety: 90 };
const MAGNITUDE = { hundred: 100 };
const FILLERS = ['um', 'uh', 'umm', 'erm', 'like', 'i think', "it's", 'its', 'is it', 'maybe'];

function parseSpokenNumber(transcript) {
  let text = transcript.toLowerCase().trim();

  // Strip filler words
  for (const f of FILLERS) {
    text = text.replace(new RegExp(`\\b${f}\\b`, 'g'), '');
  }
  text = text.replace(/[?.!,]/g, '').trim();

  // If multiple numbers present, take the last one (self-correction)
  // "twelve no thirteen" → extract [12, 13] → return 13

  // Try parsing as digit string first
  const digitMatch = text.match(/\d+/g);
  if (digitMatch) {
    return parseInt(digitMatch[digitMatch.length - 1]);
  }

  // Parse word numbers
  // ... (full implementation needed)

  return null;
}
```

## UX

### Challenge Screen

When voice is available, show a mic button alongside the answer choices:

```
┌──────────────────────────────────────────┐
│  Sparky: How many bolts do I have?       │
│                                          │
│    [ 12 ]    [ 13 ]    [ 14 ]           │
│                                          │
│    💡 Show me!    🤷 Tell me!    🎤 Say it│
└──────────────────────────────────────────┘
```

Kid taps 🎤:
1. Button pulses/animates to show "listening"
2. After recognition: if confidence > 0.8, show the recognized number briefly ("You said: 13!") then submit
3. If confidence 0.5-0.8: "Did you say thirteen?" with Yes/No buttons
4. If confidence < 0.5 or unparseable: "I didn't catch that! Try again?" with 🎤 retry button
5. If timeout (10s, no speech): "I didn't hear anything. Want to try again or pick an answer?"

### Visual Feedback During Listening

The mic button should animate while listening — pulsing rings, color change, or Sparky's antenna spinning. The kid needs to know the game is listening.

If `interimResults` is enabled, show the partial transcript live:
```
🎤 "thir..."
🎤 "thirteen" ✓
```

### Permissions

Browser will prompt for microphone permission on first use. This should happen ONLY when the kid taps the 🎤 button, never automatically. If permission denied, hide the mic button and fall back to click/type.

Show a Sparky dialogue before the first mic use: "Ooh! You want to TALK to me? Press the microphone and say the number! Beep boop!"

## Signal Extraction

Every voice answer produces a `PUZZLE_ATTEMPTED` event with extra fields:

```js
{
  type: 'PUZZLE_ATTEMPTED',
  correct: true,
  operation: 'add',
  band: 6,
  responseTimeMs: 3400,       // total time including speech
  answerMode: 'voice',        // NEW
  voiceConfidence: 0.92,      // NEW
  voiceHesitationMs: 1200,    // NEW — time before first speech
  voiceSelfCorrected: false,  // NEW — interim results changed
  voiceHadFillers: false,     // NEW — "um", "uh" detected
  voiceRetries: 0,            // NEW — how many "say it again" prompts
}
```

The reducer records these. The parent dashboard can show voice-specific stats. Future use: if a kid always hesitates on subtraction but answers addition instantly by voice, that's a CRA/difficulty signal specific to the operation.

## Tests

```
test/infrastructure/speech-recognition.test.js

parseSpokenNumber:
  - 'parses single digit words (one through nine)'
  - 'parses teen words (ten through nineteen)'
  - 'parses tens (twenty, thirty, ... ninety)'
  - 'parses compound (twenty three → 23)'
  - 'parses hyphenated (twenty-three → 23)'
  - 'parses hundreds (one hundred → 100)'
  - 'parses hundred and (one hundred and forty four → 144)'
  - 'parses digit strings ("13" → 13)'
  - 'strips filler words (umm thirteen → 13)'
  - 'strips phrases (I think its thirteen → 13)'
  - 'handles self-correction — takes last number (twelve no thirteen → 13)'
  - 'returns null for unparseable input'
  - 'returns null for empty string'
  - 'handles "a hundred" as 100'
  - 'handles mixed (twenty 3 → 23)' // kids might mix words and digits
```

Note: we can't unit test the actual SpeechRecognition API (it needs a browser + mic). But the parser and signal extraction logic are pure functions that we test thoroughly. The browser integration is a thin wrapper.

## Adapter Integration

In `adapter.js`, when voice mode is selected:

```js
// When challenge is shown and kid taps mic:
async function handleVoiceAnswer(time) {
  const result = await listenForNumber({ timeoutMs: 10000 });

  if (result.number === null) {
    // Couldn't parse — ask again, not a wrong answer
    showRetryPrompt();
    return;
  }

  if (result.confidence < 0.8) {
    // Confirm first
    const confirmed = await showConfirmation(result.number);
    if (!confirmed) {
      showRetryPrompt();
      return;
    }
  }

  // Submit as answer
  const correct = result.number === CHALLENGE.correctAnswer;
  const event = {
    type: 'PUZZLE_ATTEMPTED',
    correct,
    operation: mapOpToOperation(CHALLENGE.teachingData?.op),
    band: profileState.mathBand,
    responseTimeMs: result.totalMs,
    answerMode: 'voice',
    voiceConfidence: result.confidence,
    voiceHesitationMs: result.hesitationMs,
    voiceSelfCorrected: result.selfCorrected,
    voiceHadFillers: result.hadFillerWords,
    voiceRetries: 0,
    attemptNumber: CHALLENGE.attempts + 1,
    timestamp: Date.now(),
  };

  profileState = learnerReducer(profileState, event);
  eventLog.push(event);

  // Show result in game
  if (correct) {
    showCorrectFeedback();
  } else {
    showWrongFeedback(result.number);
  }
}
```

## Acceptance Criteria

1. 🎤 button appears on challenge screen when browser supports speech recognition
2. Tapping mic starts listening with visual feedback
3. Saying a number 1-144 is correctly recognized and submitted
4. Low confidence triggers confirmation ("Did you say 13?")
5. Unparseable speech triggers retry, not a wrong answer
6. Timeout after 10s prompts retry or fallback to clicking
7. Permission prompt only on first mic tap, never automatic
8. Works in Chrome. Hidden in Firefox. Tested in Safari if available.
9. All parser tests pass
10. Events include voice-specific signal fields

## Presentation Migration

**See `docs/presentation-migration.md` for migration trigger and plan.**
