# Post-Playtest Fixes — Implementation Spec

**Status: MOSTLY DONE.** Items 1-8, 10 shipped. Item 9 (integration tests) deferred — domain tests are cargo test now. Remaining items superseded by the challenge lifecycle refactor and burn-the-bridge.

Issues from real-kid playtesting sessions.

## 1. Voice Input Debug Mode

Voice recognition failed in practice. We need to see what's happening without a kid present.

### What to build

A debug panel that shows the raw speech recognition output in real time. Toggle with `V` key (only visible when parent overlay `P` is also active — don't show debug chrome to kids).

When the mic is active, render below the parent overlay:

```
── Voice Debug ──────────────────
Status: listening...
Interim: "thir..."
Final: "thirty"
Confidence: 0.62
Parsed: 30
Expected: 13
Match: NO
Hesitation: 1.2s
Fillers: no
Self-corrected: no
──────────────────────────────────
```

Also log every voice event to the browser console with full detail:

```js
console.log('[Voice]', {
  raw: event.results,
  transcript: 'thirty',
  confidence: 0.62,
  parsed: 30,
  expected: CHALLENGE.correctAnswer,
  interimHistory: ['thir', 'thirt', 'thirty'],
});
```

### Files to change

- `dialogue.js` — add `CHALLENGE._voiceDebug` object that accumulates state during listening. Render it when both `P` and `V` debug flags are active.
- `game.js` — add `V` key handler (only toggles when `debugOverlayVisible` is already true)

### Pre-flight check

The first time the mic button is tapped, before starting recognition, run a diagnostic and show results in the debug panel:

```js
// Check 1: Is the API available?
const hasAPI = !!(window.SpeechRecognition || window.webkitSpeechRecognition);

// Check 2: Is the page on HTTPS or localhost? (file:// won't work)
const isSecureContext = window.isSecureContext;

// Check 3: Is mic permission already granted/denied?
const permState = await navigator.permissions.query({ name: 'microphone' });
// permState.state = 'granted' | 'denied' | 'prompt'
```

Show in debug panel:
```
── Voice Pre-flight ─────────────
API available: YES
Secure context: NO (file://) ← THIS IS YOUR PROBLEM
Mic permission: prompt
─────────────────────────────────
```

If `isSecureContext` is false, show a visible warning on the mic button itself: "Mic needs HTTPS" — so even without the debug panel, the parent knows why it's not working.

If `permState === 'denied'`, show: "Mic blocked — check browser settings"

### Acceptance

- V key toggles voice debug panel (only when P overlay is visible)
- Pre-flight diagnostic runs on first mic tap, shows API/HTTPS/permission status
- If not secure context, mic button shows "Needs HTTPS" instead of "Say it"
- If permission denied, mic button shows "Mic blocked"
- Panel shows interim results updating live as the kid speaks
- Panel shows final result, confidence, parsed number, expected answer, match
- Console logs full voice event on every recognition result
- Console logs pre-flight diagnostic on first mic tap
- Panel clears when mic stops listening

## 2. Session Export

We need to pull event logs out of the browser so we can analyze play sessions offline.

### What to build

Two export mechanisms:

**A) Console command.** Type this in the browser dev console:

```js
ADAPTIVE.exportSession()
// → downloads a JSON file: robot-buddy-session-2026-03-26T02-14.json
```

The JSON contains:
```json
{
  "exportDate": "2026-03-26T02:14:00Z",
  "playerName": "Miko",
  "profile": { /* full learnerProfile snapshot */ },
  "currentSession": [ /* event log */ ],
  "previousSessions": [ /* last 5 session logs */ ],
  "operationStats": { /* full sub-skill breakdown */ },
  "metadata": {
    "gameVersion": "0.1.0",
    "totalPlayTime": 1234,
    "dumDums": 7,
    "mapId": "overworld"
  }
}
```

**B) Parent overlay button.** When P overlay is active, show a small "Export" button in the corner. Click → triggers the same download.

### Implementation

In `adapter.js`:

```js
window.ADAPTIVE.exportSession = function() {
  const data = {
    exportDate: new Date().toISOString(),
    playerName: GAME.playerName,
    profile: { ...profileState },
    currentSession: eventLog,
    previousSessions: previousSessionLogs,
    operationStats: { ...profileState.operationStats },
    metadata: {
      gameVersion: '0.1.0',
      totalPlayTime: GAME.time,
      dumDums: DUM_DUMS,
      mapId: MAP.id,
    },
  };

  const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `robot-buddy-session-${new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)}.json`;
  a.click();
  URL.revokeObjectURL(url);
};
```

The parent overlay renders a small button:
```
[Export Session]
```
Positioned bottom-left of the debug overlay panel. Calls `ADAPTIVE.exportSession()` on click.

### Files to change

- `adapter.js` — add `exportSession` to `window.ADAPTIVE`, add export button bounds and click handler in the debug overlay render function

### Acceptance

- `ADAPTIVE.exportSession()` in console downloads a JSON file
- Export button visible in parent overlay (P key), triggers same download
- JSON contains full profile, event log, previous sessions, operation stats, metadata
- File is valid JSON, human-readable (pretty-printed)

## 3. Wrong Message Clips Mic Button

The "Hmm, not quite!" feedback text overlaps the mic button when a wrong answer is given.

### The problem

The mic button is positioned at `btnY + btnH + 15`. The wrong/encouragement text is at `btnY + btnH + 45`. The voice feedback text (`CHALLENGE._voiceText`) is at `micBtnY + micBtnH + 25`. These three things stack on top of each other.

### The fix

Move the feedback text ABOVE the choices (between the question and the buttons), not below them. The mic button area below the choices should only contain mic-related UI (the button, the "Say it" label, the confirmation buttons, the voice feedback text).

Layout should be:
```
┌──────────────────────────────────────────┐
│  [Question text]                          │
│                                           │
│  [Feedback: "Hmm, not quite!"]           │  ← moved up
│                                           │
│  [ 12 ]    [ 13 ]    [ 14 ]             │
│                                           │
│          [🎤 Say it]                      │
│     [voice feedback text]                 │
│     [Did you say 30?  Yes / No]          │
└──────────────────────────────────────────┘
```

### Files to change

- `dialogue.js` — in `renderChallenge()`, move the `CHALLENGE.attempts > 0` feedback text above the choice buttons. Adjust y-coordinates. The mic button and voice text stay where they are.

### Acceptance

- Wrong answer feedback ("Hmm, not quite!") appears between question and choices, not overlapping the mic
- Mic button, voice feedback, and confirmation buttons have clear space below the choices
- Works for all three states: first wrong, second wrong, voice wrong

## 4. In-Game Settings Access

### Problem

If you start a game without an API key (or want to change TTS/AI provider settings), you have to delete your save and start over. There's no way to edit settings from inside the game.

### Fix

Add a settings button to the pause/menu state. Press `ESC` or a gear icon to open an overlay with the same settings that appear on the new game screen:
- AI provider picker (Anthropic / Gemini)
- API key input
- Voice provider (Browser TTS / ElevenLabs)
- ElevenLabs API key
- TTS toggle

The overlay pauses the game (`GAME.state = 'SETTINGS'`). Settings save to the current save slot immediately. Closing returns to `PLAYING`.

### Files to change

- `index.html` — extract settings form HTML into a reusable template or render in canvas
- `game.js` — add `SETTINGS` state, ESC key handler, settings overlay rendering
- `dialogue.js` or `adapter.js` — apply changed settings live (swap API key, toggle TTS)

### Acceptance

- ESC opens settings overlay during gameplay
- Can change API key, AI provider, voice provider, TTS toggle
- Changes apply immediately (no restart needed)
- Settings persist to save slot
- ESC or close button returns to game

## 5. Start Screen Scrolling

### Problem

The new game form (name, gender, level pickers, settings accordion) is taller than the viewport on smaller screens. No scroll, so the bottom is cut off — can't reach the start button.

### Fix

Make the title screen container scrollable. Add `overflow-y: auto; max-height: 100vh;` to the `#titleScreen` div. The canvas area should not scroll — only the title screen form.

### Files to change

- `index.html` — add CSS for scrollable title screen container

### Acceptance

- Title screen scrolls on small viewports
- Start button is always reachable
- Canvas game area does not scroll during gameplay

## 6. "Hmm, not quite!" Persists Between Challenges

### Problem

The wrong-answer feedback text ("Hmm, not quite! Try again!") from one challenge is still visible when the next challenge appears. The `CHALLENGE.attempts` counter or feedback state isn't being reset when a new challenge starts.

### Fix

In `startChallenge()`, explicitly reset all feedback state:

```js
CHALLENGE.attempts = 0;
CHALLENGE._voiceText = '';
CHALLENGE._voiceRetries = 0;
CHALLENGE._voiceListening = false;
CHALLENGE._voiceConfirming = false;
CHALLENGE._lastVoiceResult = null;
CHALLENGE._micLabel = null;
```

### Files to change

- `dialogue.js` — in `startChallenge()`, reset all challenge feedback state

### Acceptance

- New challenge always starts clean with no leftover feedback text
- No leftover voice state from previous challenge
- Works across NPC → NPC transitions and chest → NPC transitions

**Note:** The challenge lifecycle refactor (`docs/challenge-lifecycle-spec.md`) eliminates this bug class — each challenge creates a new immutable state object. No globals to forget to reset.

## 7. TTS Should Sync With Typewriter, Not After

### Problem

TTS currently speaks the line AFTER the typewriter finishes displaying the full text. This was intentional (to avoid spoiling the text before the kid reads it). But this isn't a reading game — a 4-year-old can't read. The text and speech should arrive simultaneously so the kid hears Sparky "talking" while the text appears, like subtitles.

### Fix

Revert the TTS timing to speak at the START of each dialogue line (when the typewriter begins), not at the end. The typewriter becomes a visual accompaniment to the speech, not the primary content delivery.

In `startDialogue()`:
```js
// Speak the first line immediately
if (lines.length > 0) {
  speakLine(lines[0].speaker, lines[0].text);
}
```

In `advanceDialogue()` (moving to next line):
```js
// Speak the new line immediately
const line = DIALOGUE.lines[DIALOGUE.currentLine];
speakLine(line.speaker, line.text);
```

Remove the `speakLine` call from `updateDialogue()` (the typewriter completion handler) and from the space-to-skip path in `advanceDialogue()`.

When the user skips the typewriter (space), the text jumps to full but speech continues playing — don't restart it.

### Files to change

- `dialogue.js` — move `speakLine` calls to line start, not line end

### Acceptance

- TTS begins speaking when the typewriter starts, not when it finishes
- Skipping the typewriter (space) shows full text but doesn't restart speech
- Advancing to the next line cancels current speech and starts the new line's speech
- Works for all speakers (Sparky, Mommy, Gizmo, etc.)

## 8. Dum Dum Awarded on Wrong Answer

### Problem

In `dialogue.js`, `triggerRobotInteraction` (line 1376) and `triggerNPCChat` (line 1419) call `awardDumDum(time)` in the WRONG answer branch. The original design framed it as "Sparky got confused, give him a Dum Dum as consolation" — cute story, terrible incentive structure. Getting questions wrong is more rewarding than getting them right (wrong = Dum Dum, correct = just praise).

Chest interactions are correct (Dum Dum only on correct answer).

### Fix

Move `awardDumDum(time)` to the `correct` branch in both `triggerRobotInteraction` and `triggerNPCChat`. Update dialogue text to match:

```js
// triggerRobotInteraction — correct branch:
if (correct) {
  awardDumDum(time);
  startDialogue([{
    speaker: 'Sparky',
    text: `WOW ${playerName}! You are SO SMART! Here, have a Dum Dum! You earned it!`,
  }]);
} else {
  startDialogue([{
    speaker: 'Sparky',
    text: `Hmm, that's okay boss! We'll figure it out together next time!`,
  }]);
}

// triggerNPCChat — same pattern:
if (correct) {
  awardDumDum(time);
  startDialogue([{
    speaker: npc.name,
    text: `Incredible, ${playerName}! You earned a Dum Dum!`,
  }]);
} else {
  startDialogue([
    { speaker: npc.name, text: `Oh no! Let's keep practicing!` },
    { speaker: 'Sparky', text: `Don't worry boss, we'll get it next time!` },
  ]);
}
```

### Files to change

- `dialogue.js` — swap `awardDumDum` from wrong to correct branch in both `triggerRobotInteraction` and `triggerNPCChat`

### Acceptance

- Correct answer → Dum Dum awarded
- Wrong answer → no Dum Dum, encouragement only
- Chest behavior unchanged (already correct)
- Dum Dum counter in HUD only increases on correct answers

**Note:** The challenge lifecycle refactor (`docs/challenge-lifecycle-spec.md`) eliminates this bug class entirely — reward logic moves to a single domain reducer where it's impossible to get backwards. The quick fix above is for shipping before the refactor lands.

## 9. Integration / E2E Tests

### Problem

We have 147 domain tests but zero tests for the game itself. The Dum Dum bug above wouldn't be caught by any existing test. The challenge state persistence bug, the voice routing bug, and the layout clip were all found by manual playtesting. We need at least basic integration tests that exercise the actual game code.

### What to test

Two tiers:

**Tier 1: Headless integration tests (vitest, no browser)**

These test the adapter and game logic by simulating the global environment:

```js
// Mock the globals that dialogue.js / game.js / adapter.js expect
globalThis.GAME = { state: 'PLAYING', canvas: null, ctx: null, canvasW: 960, canvasH: 640 };
globalThis.SKILL = { math: { band: 1, streak: 0 } };
globalThis.CHALLENGE = { active: false, choices: [], attempts: 0 };
globalThis.DUM_DUMS = 0;
// ... etc
```

Test scenarios:
```
Adapter integration:
  - 'generateMathChallenge returns challenge with subSkill and features'
  - 'selectChallengeChoice records event through learnerReducer'
  - 'voice answer via _submitVoiceAnswer records event through learnerReducer'
  - 'save and load round-trips profile state including rolling window'
  - 'response time > 30s is capped to null'
  - 'intake quiz runs and sets profile from results'

Reward logic:
  - 'correct answer on robot interaction awards Dum Dum'
  - 'wrong answer on robot interaction does NOT award Dum Dum'
  - 'correct answer on NPC interaction awards Dum Dum'
  - 'wrong answer on NPC interaction does NOT award Dum Dum'
  - 'correct answer on chest awards Dum Dum'
  - 'wrong answer on chest does NOT award Dum Dum'

Challenge state:
  - 'new challenge resets attempts, voiceText, and feedback state'
  - 'challenge feedback does not persist across interactions'
```

**Tier 2: Playwright E2E tests (browser, low priority)**

Smoke tests that the game actually loads and basic interactions work:

```
  - 'game loads and shows title screen'
  - 'can enter name and start new game'
  - 'player can move with arrow keys'
  - 'interacting with NPC shows dialogue'
  - 'challenge appears with 3 choices'
  - 'correct answer shows celebration'
  - 'P key toggles parent overlay'
  - 'export session downloads a JSON file'
```

These need a dev server running (`npx serve .`) and a real browser. Lower priority than headless integration tests but valuable for catching rendering bugs.

### Files to create

```
test/integration/
  adapter.test.js         — adapter + domain integration
  reward-logic.test.js    — Dum Dum award correctness
  challenge-state.test.js — state reset between challenges

test/integration/setup.js — mock globals (GAME, SKILL, CHALLENGE, etc.)

test/e2e/                 — Playwright tests (future)
  smoke.spec.js
```

### Setup helper

The integration tests need to load the legacy global-based code. Create a setup file that:
1. Sets up the global mocks
2. Imports the relevant functions from dialogue.js (may need minor refactoring to make them importable)
3. Imports the adapter's monkey-patched versions

This is inherently messy because the legacy code is global-mutable. The integration tests will be messier than the domain tests. That's expected — the point is catching bugs like the Dum Dum reward inversion, not writing clean test code.

### Acceptance

- `npm test` runs both domain and integration tests
- Dum Dum reward logic is tested for all 3 interaction types (robot, NPC, chest)
- Challenge state reset is tested
- Adapter round-trip (generate → answer → record event → save → load) is tested

## 10. TTS Reads Math Symbols Instead of Words

### Problem

TTS reads "What is 8 × 5?" as "what is 8 [silence or garbled] 5". Root cause: one string serves display AND speech. The TTS layer shouldn't have to parse math symbols — the source should produce separate display and speech text.

### Fix

**Superseded by `docs/challenge-lifecycle-spec.md`.** The challenge lifecycle refactor produces `{ displayText, speechText }` from structured data at the source. The TTS layer receives speech-ready text and never sees symbols. This eliminates the entire class of display/speech divergence bugs, not just math symbols.
