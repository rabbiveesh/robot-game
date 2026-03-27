# Challenge Lifecycle — Domain State Machine

## The Problem Class

Five bugs from playtesting share one root cause: the challenge lifecycle has no domain model.

| Bug | Root cause |
|-----|-----------|
| Dum Dum awarded on wrong answer | Reward logic is in 3 presentation functions, 2 have it backwards |
| "Hmm, not quite!" persists between challenges | Feedback state is a global, nobody resets it |
| TTS reads math symbols wrong | One string serves display + speech, no structured data |
| Voice answers bypassed the reducer | Adapter patches one code path, voice takes another |
| Stale voice state between challenges | Voice state lives on the global CHALLENGE object |

All five would be impossible if the challenge lifecycle were a domain state machine with a single reducer that produces all outputs (display text, speech text, feedback, reward, next phase).

## The Fix

Extract the challenge lifecycle into `src/domain/challenge/`. Pure state machine. No rendering, no globals, no browser dependencies. Tested alongside the learning domain.

### Challenge State

```js
function createChallengeState(challenge, context) {
  return Object.freeze({
    // Phase
    phase: 'presented',  // 'presented' | 'feedback' | 'teaching' | 'complete'

    // The challenge data
    challenge,           // from generateChallenge()
    context,             // { source: 'robot'|'npc'|'chest', npcName: '...' }
    attempts: 0,
    maxAttempts: 2,      // wrong twice → teaching mode
    correct: null,

    // Outputs for presentation (both display and speech, always in sync)
    question: {
      display: challenge.displayText,
      speech: challenge.speechText,
    },
    feedback: null,      // { display: string, speech: string } or null
    reward: null,        // { type: 'dum_dum', amount: number } or null

    // Voice state (scoped to this challenge, dies with it)
    voice: {
      listening: false,
      confirming: false,
      confirmNumber: null,
      retries: 0,
      lastResult: null,
      text: null,        // { display, speech }
    },
  });
}
```

### Challenge Reducer

```js
function challengeReducer(state, action) {
  switch (action.type) {
    case 'ANSWER_SUBMITTED': {
      const correct = action.answer === state.challenge.correctAnswer;
      const attempts = state.attempts + 1;

      if (correct) {
        return Object.freeze({
          ...state,
          phase: 'complete',
          correct: true,
          attempts,
          reward: { type: 'dum_dum', amount: 1 },
          feedback: {
            display: 'Amazing! You got it!',
            speech: 'Amazing! You got it!',
          },
          voice: resetVoice(),
        });
      }

      if (attempts >= state.maxAttempts) {
        return Object.freeze({
          ...state,
          phase: 'teaching',
          correct: false,
          attempts,
          reward: null,
          feedback: {
            display: "Let's figure it out together!",
            speech: "Let's figure it out together!",
          },
          voice: resetVoice(),
        });
      }

      return Object.freeze({
        ...state,
        phase: 'feedback',
        attempts,
        feedback: {
          display: 'Hmm, not quite! Try again!',
          speech: 'Hmm, not quite! Try again!',
        },
      });
    }

    case 'RETRY': {
      // Kid wants to try again after feedback
      return Object.freeze({
        ...state,
        phase: 'presented',
        feedback: null,
      });
    }

    case 'TEACHING_COMPLETE': {
      return Object.freeze({
        ...state,
        phase: 'complete',
      });
    }

    // Voice lifecycle actions
    case 'VOICE_LISTEN_START': {
      return Object.freeze({
        ...state,
        voice: { ...state.voice, listening: true, text: null },
      });
    }

    case 'VOICE_RESULT': {
      const { number, confidence } = action;
      if (number === null || confidence < 0.5) {
        return Object.freeze({
          ...state,
          voice: {
            ...state.voice,
            listening: false,
            retries: state.voice.retries + 1,
            text: { display: "Didn't catch that! Tap mic to try again.", speech: "I didn't catch that! Tap the microphone to try again." },
            lastResult: action,
          },
        });
      }
      if (confidence < 0.8) {
        return Object.freeze({
          ...state,
          voice: {
            ...state.voice,
            listening: false,
            confirming: true,
            confirmNumber: number,
            text: { display: `Did you say ${number}?`, speech: `Did you say ${number}?` },
            lastResult: action,
          },
        });
      }
      // High confidence — auto-submit (caller should dispatch ANSWER_SUBMITTED next)
      return Object.freeze({
        ...state,
        voice: {
          ...state.voice,
          listening: false,
          text: { display: `You said: ${number}!`, speech: `You said ${number}!` },
          lastResult: action,
        },
      });
    }

    case 'VOICE_CONFIRM': {
      if (action.confirmed) {
        // Caller should dispatch ANSWER_SUBMITTED next
        return Object.freeze({
          ...state,
          voice: { ...state.voice, confirming: false },
        });
      }
      return Object.freeze({
        ...state,
        voice: {
          ...state.voice,
          confirming: false,
          confirmNumber: null,
          retries: state.voice.retries + 1,
          text: { display: 'Okay! Tap mic to try again.', speech: 'Okay! Tap the microphone to try again.' },
        },
      });
    }

    case 'VOICE_ERROR': {
      const errorText = action.error === 'not-allowed'
        ? { display: 'Mic blocked', speech: 'Microphone is blocked. Use the buttons instead.' }
        : { display: "Didn't hear anything. Tap mic to try again!", speech: "I didn't hear anything. Tap the microphone to try again!" };
      return Object.freeze({
        ...state,
        voice: { ...state.voice, listening: false, text: errorText },
      });
    }

    default:
      return state;
  }
}

function resetVoice() {
  return Object.freeze({
    listening: false,
    confirming: false,
    confirmNumber: null,
    retries: 0,
    lastResult: null,
    text: null,
  });
}
```

### Challenge Generator Changes

The challenge generator produces BOTH display and speech text from structured data — no regex, no post-processing:

```js
// In generateNumbers(), for each band case:
// Instead of just:
question = `What is ${a} × ${b}?`;

// Produce:
displayText = `What is ${a} × ${b}?`;
speechText = `What is ${a} times ${b}?`;

// Operation symbol map:
const DISPLAY_OP = { '+': '+', '-': '-', '×': '×', '÷': '÷' };
const SPEECH_OP = { '+': 'plus', '-': 'minus', '×': 'times', '÷': 'divided by' };

// Standard questions:
displayText = `What is ${a} ${DISPLAY_OP[op]} ${b}?`;
speechText = `What is ${a} ${SPEECH_OP[op]} ${b}?`;

// Number bonds:
displayText = `What ${DISPLAY_OP[op]} ${b} = ${total}?`;
speechText = `What ${SPEECH_OP[op]} ${b} equals ${total}?`;
```

The `question` field on challenges becomes `{ display, speech }` instead of a flat string.

## What This Eliminates

| Bug class | How it's eliminated |
|-----------|-------------------|
| Reward on wrong answer | Reward is produced by the reducer, not by 3 different presentation functions. ONE code path, tested. |
| Stale feedback between challenges | Feedback is on the challenge state. New challenge = new state = null feedback. No globals to forget to reset. |
| TTS reading symbols | Speech text is produced alongside display text from structured data. TTS never sees symbols. |
| Voice state bleeding | Voice state is inside the challenge state. New challenge = reset voice. |
| Voice bypassing reducer | Voice actions go through the same challengeReducer. No separate code path. |
| Inconsistent behavior across interaction types | All interaction types create a ChallengeState and dispatch actions. The reducer is the single source of truth for "what happens next." |

## Integration with Existing Architecture

### Where it lives

```
src/domain/challenge/
  challenge-state.js     # createChallengeState, challengeReducer
  index.js               # re-exports
```

This is a new bounded context alongside `src/domain/learning/`. The challenge reducer handles the lifecycle; the learning reducer handles the profile. They communicate via events: when the challenge reaches `phase: 'complete'`, the application layer dispatches a `PUZZLE_ATTEMPTED` event to the learning reducer.

### How the adapter changes

The adapter currently monkey-patches `startChallenge`, `selectChallengeChoice`, and `handleVoiceInput`. With the challenge state machine:

```js
// Instead of monkey-patching individual functions:
let challengeState = null;

// When an NPC triggers a challenge:
function onChallengeStart(challenge, context) {
  challengeState = createChallengeState(challenge, context);
  // Presentation reads challengeState to render
}

// When the kid clicks an answer:
function onAnswer(answer) {
  challengeState = challengeReducer(challengeState, { type: 'ANSWER_SUBMITTED', answer });

  if (challengeState.phase === 'complete') {
    // Apply reward
    if (challengeState.reward) {
      DUM_DUMS += challengeState.reward.amount;
    }
    // Record to learning domain
    profileState = learnerReducer(profileState, {
      type: 'PUZZLE_ATTEMPTED',
      correct: challengeState.correct,
      ...
    });
  }
}

// When the kid uses voice:
function onVoiceListen() {
  challengeState = challengeReducer(challengeState, { type: 'VOICE_LISTEN_START' });
  // Start browser recognition...
}
function onVoiceResult(result) {
  challengeState = challengeReducer(challengeState, { type: 'VOICE_RESULT', ...result });
  if (challengeState.voice.lastResult?.confidence >= 0.8) {
    onAnswer(result.number); // auto-submit
  }
}
```

The monkey-patching of `selectChallengeChoice` and `handleVoiceInput` goes away. The presentation layer reads `challengeState` directly.

### Legacy rendering reads from challenge state

The existing `renderChallenge` in `dialogue.js` currently reads from the global `CHALLENGE` object. During migration, it reads from `challengeState` instead:

```js
function renderChallenge(ctx, canvasW, canvasH, time) {
  if (!challengeState) return;
  const cs = challengeState;

  // Question text (display version, not speech)
  drawText(cs.question.display, ...);

  // Feedback (if any)
  if (cs.feedback) drawText(cs.feedback.display, ...);

  // Voice UI
  if (cs.voice.listening) drawListeningAnimation(...);
  if (cs.voice.confirming) drawConfirmButtons(cs.voice.confirmNumber, ...);
  if (cs.voice.text) drawText(cs.voice.text.display, ...);

  // Celebration / teaching based on phase
  if (cs.phase === 'complete' && cs.correct) drawCelebration(...);
  if (cs.phase === 'teaching') drawTeaching(...);
}
```

TTS calls use the speech version:

```js
// When feedback changes:
if (cs.feedback?.speech) speakLine(context.speaker, cs.feedback.speech);

// When voice text changes:
if (cs.voice.text?.speech) speakLine('Sparky', cs.voice.text.speech);
```

The TTS layer NEVER transforms text. It speaks exactly what it's given. The separation is upstream, at the source.

### Interaction orchestration simplifies

The three interaction functions collapse to one pattern:

```js
async function triggerChallengeInteraction(source, npcName, playerName, time) {
  const challenge = generateChallenge(profileState, Math.random);
  const context = { source, npcName: npcName || 'Sparky' };

  // Optional AI intro
  let intro = null;
  if (API_KEY) {
    intro = await fetchDialogueForChallenge(context, challenge, playerName);
  }
  if (!intro) intro = defaultChallengeIntro(context, playerName);

  startDialogue([{ speaker: context.npcName, text: intro.display, speech: intro.speech }], () => {
    challengeState = createChallengeState(challenge, context);
    GAME.state = 'CHALLENGE';
    // Rendering takes over from here, reading challengeState
  });
}

// Robot:
triggerChallengeInteraction('robot', 'Sparky', playerName, time);
// NPC:
triggerChallengeInteraction('npc', npc.name, playerName, time);
// Chest:
triggerChallengeInteraction('chest', 'Sparky', playerName, time);
```

One function. One code path. One reducer. The Dum Dum bug literally cannot happen because the reward field is set by the reducer, which always gives rewards on correct and never on wrong.

## Dialogue System: display + speech Separation

While we're at it, the dialogue system should carry both display and speech on every line:

```js
// Current:
startDialogue([{ speaker: 'Sparky', text: 'WOW! 8 × 5 = 40!' }]);
// TTS has to regex-parse this

// After:
startDialogue([{
  speaker: 'Sparky',
  text: 'WOW! 8 × 5 = 40!',               // for canvas rendering
  speech: 'WOW! 8 times 5 equals 40!',     // for TTS
}]);
```

If `speech` is absent, TTS falls back to `text` (backward compatible for non-math dialogue). This is a one-field addition to the dialogue line schema, not a rewrite.

`speakLine` changes from:
```js
function speakLine(speaker, text) {
  const clean = text.replace(/.../g, '');
  speak(clean);
}
```
To:
```js
function speakLine(speaker, text, speech) {
  // Use speech if provided, else clean text as fallback
  const toSpeak = speech || text.replace(/[🤖🚀⭐🌟🍭📍#]/g, '').replace(/\*[^*]+\*/g, '');
  speak(toSpeak);
}
```

The regex stays ONLY as a fallback for legacy dialogue lines that don't have `speech`. All new dialogue lines should always have both.

## Tests

```
test/domain/challenge/
  challenge-state.test.js

Challenge lifecycle:
  - 'new challenge starts in presented phase with null feedback'
  - 'correct answer → complete phase with reward'
  - 'wrong answer (first) → feedback phase, no reward'
  - 'wrong answer (second) → teaching phase, no reward'
  - 'reward is ALWAYS null when incorrect'
  - 'reward is ALWAYS present when correct'
  - 'feedback resets on retry'
  - 'voice state resets on complete'
  - 'creating a new challenge state has no trace of previous challenge'

Voice lifecycle:
  - 'VOICE_LISTEN_START sets listening true'
  - 'VOICE_RESULT with null number → retry, not wrong answer'
  - 'VOICE_RESULT with confidence < 0.5 → retry'
  - 'VOICE_RESULT with confidence 0.5-0.8 → confirming'
  - 'VOICE_RESULT with confidence >= 0.8 → ready to submit'
  - 'VOICE_CONFIRM yes → ready to submit'
  - 'VOICE_CONFIRM no → retry'
  - 'VOICE_ERROR not-allowed → mic blocked text'

Display/speech separation:
  - 'question has both display and speech fields'
  - 'display contains × symbol, speech contains "times"'
  - 'display contains ÷ symbol, speech contains "divided by"'
  - 'feedback has both display and speech fields'
  - 'voice text has both display and speech fields'
```

## Implementation Plan

1. **Create `src/domain/challenge/`** — state machine, reducer, tests. Pure domain, no browser deps.
2. **Update challenge generator** — produce `{ displayText, speechText }` instead of `question` string. Use `DISPLAY_OP` / `SPEECH_OP` maps.
3. **Update adapter** — replace monkey-patching with challenge state management. One `onAnswer` function for both click and voice.
4. **Update dialogue system** — add `speech` field to line schema. `speakLine` uses `speech` if present.
5. **Update `renderChallenge`** — read from `challengeState` instead of global `CHALLENGE`. Display uses `.display`, TTS uses `.speech`.
6. **Delete old interaction functions** — replace `triggerRobotInteraction`, `triggerNPCChat`, `triggerChestInteraction` with unified `triggerChallengeInteraction`.
7. **Delete monkey-patches** — `selectChallengeChoice` and `handleVoiceInput` patches in adapter die. The challenge reducer handles everything.

Steps 1-2 are safe (pure domain, tested). Steps 3-7 touch the legacy code and should be done together as a single PR to avoid inconsistent states.

## This Is Presentation Migration Trigger #1

Per `docs/presentation-migration.md`, the interaction model forces migration of the challenge UI. This spec IS that migration for the challenge lifecycle. After this:
- `dialogue.js` loses ~200 lines of challenge state management
- `adapter.js` loses ~100 lines of monkey-patches
- The global `CHALLENGE` object becomes a read-only view that `renderChallenge` uses until we migrate rendering too
- All challenge rules live in `src/domain/challenge/`, tested

## Files Changed

```
NEW:
  src/domain/challenge/challenge-state.js
  src/domain/challenge/index.js
  test/domain/challenge/challenge-state.test.js

MODIFIED:
  src/domain/learning/challenge-generator.js  — displayText + speechText output
  adapter.js                                  — replace monkey-patches with challenge state
  dialogue.js                                 — speakLine accepts speech param, renderChallenge reads state,
                                                unified triggerChallengeInteraction, delete 3 old functions
```
