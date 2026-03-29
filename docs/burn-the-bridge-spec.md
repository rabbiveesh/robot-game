# Burn the Bridge — Kill Legacy Challenge Flow

## The Problem

Five bugs, one root cause: two systems manage the challenge lifecycle.

| System | Controls | Problem |
|--------|----------|---------|
| **Adapter** (new) | `challengeState`, `challengeReducer`, `_onChallengeAnswer`, `autoDismissChallenge` | Auto-dismisses on timeout, records events, applies rewards |
| **game.js + dialogue.js** (legacy) | `CHALLENGE` globals, `dismissChallenge()`, `dismissTeaching()`, Space key handler | Also fires `onComplete` callback, also manages transitions |

They race. Both fire `onComplete`. The sync bridge (`syncToLegacyChallenge`) writes state from new→old, but the Space handler in game.js reads old and acts on it independently. The QuizRenderer writes `_bounds` to frozen domain objects (silent failure). The adapter has a typo (`action.answer` instead of `answer`).

## The Fix

Remove ALL legacy challenge management from game.js and dialogue.js. The adapter owns the challenge lifecycle exclusively. No sync bridge needed because nothing reads the old globals.

### Step 1: Fix the typo (immediate, unblocks everything)

```js
// adapter.js line 127
// OLD: const correct = action.answer === ch.correctAnswer;
// NEW:
const correct = answer === ch.correctAnswer;
```

### Step 2: QuizRenderer uses its own bounds map, not frozen objects

```js
// quiz-renderer.js — store bounds externally
const _choiceBounds = [];

// In render():
_choiceBounds.length = 0;  // clear
choices.forEach((choice, i) => {
  const bx = ...;
  const by = ...;
  _choiceBounds[i] = { x: bx, y: by, w: btnW, h: btnH };
  // DON'T set choice._bounds — it's frozen
});

// In handleClick():
for (let i = 0; i < _choiceBounds.length; i++) {
  const b = _choiceBounds[i];
  if (b && mx >= b.x && mx <= b.x + b.w && my >= b.y && my <= b.y + b.h) {
    return { type: 'ANSWER_SUBMITTED', answer: Number(cs.challenge.choices[i].text) };
  }
}
```

### Step 3: game.js CHALLENGE state — dispatch through adapter, not legacy

Replace the entire CHALLENGE case in game.js's key handler:

```js
// OLD (game.js):
} else if (GAME.state === 'CHALLENGE') {
  if (CHALLENGE.showTeaching) {
    dismissTeaching(GAME.time);
  } else if (CHALLENGE.answered) {
    dismissChallenge();
    // ... state transition logic
  }
}

// NEW:
} else if (GAME.state === 'CHALLENGE') {
  // All challenge keyboard input goes through the renderer or adapter
  if (window._challengeState) {
    const cs = window._challengeState;
    if (cs.phase === 'teaching') {
      if (typeof window._onTeachingComplete === 'function') window._onTeachingComplete();
    }
    // answered/celebration: auto-dismiss handles it, Space is ignored
    // presented/feedback: number keys handled separately (below)
  }
}
```

Space during celebration does NOTHING — auto-dismiss handles the transition. No double-fire possible.

### Step 4: game.js click handler — route through QuizRenderer

```js
// OLD: directly check _bounds and call legacy functions
// NEW:
if (GAME.state === 'CHALLENGE' && window._activeRenderer && window._challengeState) {
  const action = window._activeRenderer.handleClick(mx, my, window._challengeState);
  if (action) {
    if (action.type === 'ANSWER_SUBMITTED') {
      window._onChallengeAnswer(action.answer, GAME.time, 'choice');
    } else if (action.type === 'SHOW_ME') {
      window._onShowMe();
    } else if (action.type === 'TELL_ME') {
      window._onTellMe();
    }
    return;
  }
}
```

One entry point for all challenge clicks. The renderer decides what was clicked. The adapter handles it. No legacy code involved.

### Step 5: game.js number key handler — route through QuizRenderer

```js
if (GAME.state === 'CHALLENGE' && window._activeRenderer && window._challengeState) {
  const action = window._activeRenderer.handleKey(e.key, window._challengeState);
  if (action) {
    if (action.type === 'ANSWER_SUBMITTED') {
      window._onChallengeAnswer(action.answer, GAME.time, 'choice');
    }
  }
}
```

### Step 6: Delete dead code

From dialogue.js, delete:
- `dismissChallenge()` — replaced by auto-dismiss in adapter
- `dismissTeaching()` — replaced by `_onTeachingComplete` in adapter
- `handleChallengeClick()` — replaced by QuizRenderer.handleClick
- `selectChallengeChoice()` — replaced by `_onChallengeAnswer`
- The `syncToLegacyChallenge` function in adapter — nothing reads from CHALLENGE globals anymore

From game.js, delete:
- The `_showMeBounds` / `_tellMeBounds` click checks — handled by QuizRenderer
- The `CHALLENGE.answered` / `CHALLENGE.showTeaching` checks — handled by state machine

### Step 7: Minimal CHALLENGE globals for renderChallenge legacy fallback

The old `renderChallenge` in dialogue.js is still the fallback when `_challengeState` is null. Keep `CHALLENGE.active` and `CHALLENGE.onComplete` for the intake fallback path (if it still exists). Everything else (`CHALLENGE.answered`, `showTeaching`, `wasCorrect`, etc.) is dead.

Actually — the intake was migrated to use the state machine. Is there any code path that doesn't go through `_startChallengeFromDomain`? If not, the legacy `renderChallenge` fallback is dead too and can be deleted.

## What This Eliminates

| Bug | Why it's gone |
|-----|--------------|
| `action` not defined | Fixed directly (typo) |
| Unclickable buttons | Bounds stored externally, not on frozen objects |
| Stuck after correct | No Space dismiss. Auto-dismiss is the only path. |
| Stuck after tell-me | No Space dismiss competing with auto-dismiss. |
| Double callback fire | `onComplete` fires from exactly one place (autoDismissChallenge) |
| Division viz not showing | Separate issue — consider showing visual by default at low CRA stages |

## Files Changed

```
adapter.js      — fix typo, remove syncToLegacyChallenge, simplify _onChallengeAnswer
game.js         — CHALLENGE key/click handlers dispatch through renderer + adapter
quiz-renderer.js — external bounds map, not frozen object mutation
dialogue.js     — delete dismissChallenge, dismissTeaching, handleChallengeClick,
                   selectChallengeChoice (4 dead functions)
```

## Tests

Run existing 235 tests — all should still pass (domain is untouched).

Manual test:
1. New game → intake 4 questions → all auto-advance, no freeze
2. Walk to NPC → challenge → click correct answer → celebration → auto-advance to dialogue
3. Click wrong → feedback → click correct → celebration → auto-advance
4. Click wrong twice → teaching → Space → auto-advance
5. Show-me → visual appears → click correct → celebration
6. Tell-me → teaching with answer → Space → auto-advance
7. Voice input → same flow
8. No double-dialogues, no freezes, no stuck states
