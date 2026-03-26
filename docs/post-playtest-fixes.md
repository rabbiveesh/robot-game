# Post-Playtest Fixes — Implementation Spec

Three issues from the first real-kid playtest session.

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

### Acceptance

- V key toggles voice debug panel (only when P overlay is visible)
- Panel shows interim results updating live as the kid speaks
- Panel shows final result, confidence, parsed number, expected answer, match
- Console logs full voice event on every recognition result
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
