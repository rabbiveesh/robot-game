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

    // Rendering hint (tells the presentation which renderer to use)
    renderHint: context.renderHint || {
      craStage: 'abstract',          // overridden by learner profile per operation
      answerMode: 'choice',          // overridden by learner profile
      interactionType: 'quiz',       // 'quiz' | 'puzzle' | 'shop' | 'drag' | 'number_line'
    },

    // Scaffold tracking (show-me / tell-me)
    hintUsed: false,
    hintLevel: 0,        // how many times show-me was pressed
    toldMe: false,       // did the kid press tell-me

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

    // Scaffold actions (show-me / tell-me)
    case 'SHOW_ME': {
      const currentCra = state.renderHint.craStage;
      const lowerCra = currentCra === 'abstract' ? 'representational'
                     : currentCra === 'representational' ? 'concrete'
                     : 'concrete';
      if (lowerCra === currentCra) return state; // already at bottom
      return Object.freeze({
        ...state,
        renderHint: Object.freeze({ ...state.renderHint, craStage: lowerCra }),
        hintUsed: true,
        hintLevel: (state.hintLevel || 0) + 1,
      });
    }

    case 'TELL_ME': {
      // Sparky shows the answer with a full concrete walkthrough
      // Phase goes to teaching (same as 2 wrong answers) but correct stays null
      // — this is not a wrong answer, it's a request for help
      return Object.freeze({
        ...state,
        phase: 'teaching',
        toldMe: true,
        reward: null,
        feedback: {
          display: `The answer is ${state.challenge.correctAnswer}!`,
          speech: `The answer is ${state.challenge.correctAnswer}!`,
        },
        renderHint: Object.freeze({ ...state.renderHint, craStage: 'concrete' }),
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

Scaffold (show-me / tell-me):
  - 'SHOW_ME drops CRA from abstract to representational'
  - 'SHOW_ME drops CRA from representational to concrete'
  - 'SHOW_ME at concrete returns state unchanged'
  - 'SHOW_ME sets hintUsed true and increments hintLevel'
  - 'TELL_ME sets phase to teaching with concrete CRA'
  - 'TELL_ME sets toldMe true and reward null'
  - 'TELL_ME feedback contains the correct answer'

Render hint:
  - 'new challenge state includes renderHint from context'
  - 'default renderHint is abstract/choice/quiz'
  - 'SHOW_ME updates renderHint.craStage'
  - 'quest challenge can override interactionType to puzzle'

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

Steps 1-7 are done. The remaining work is CRA renderers + the adaptive feedback loop.

## CRA Adaptive Feedback Loop

The challenge lifecycle captures hint/CRA signals (`hintUsed`, `hintLevel`, `toldMe`, `craLevelShown`) in `PUZZLE_ATTEMPTED` events. The learner reducer must consume these to advance CRA stages per operation and adjust scaffolding. Currently the reducer has a TODO placeholder at `TEACHING_RETRY` and ignores all hint fields.

### What the learner reducer should consume from PUZZLE_ATTEMPTED

```js
case 'PUZZLE_ATTEMPTED': {
  // ... existing band/streak/spread/scaffolding/pace logic ...

  // CRA stage progression (NEW)
  const op = event.operation;
  let newCraStages = state.craStages;

  if (event.correct && !event.hintUsed && !event.toldMe) {
    // Correct without any help at the current CRA stage
    // Track consecutive no-hint successes per operation
    // After 3 no-hint correct at the SAME CRA stage, promote CRA
    //   concrete → representational → abstract
    const noHintSuccesses = countConsecutiveNoHintCorrect(newWindow, op, state.craStages[op]);
    if (noHintSuccesses >= 3 && state.craStages[op] !== 'abstract') {
      const nextCra = state.craStages[op] === 'concrete' ? 'representational' : 'abstract';
      newCraStages = Object.freeze({ ...state.craStages, [op]: nextCra });
    }
  }

  if (event.hintUsed && event.correct) {
    // Correct but needed a hint — the CRA stage they dropped TO is where they succeed
    // If they're marked as 'abstract' for this operation but needed show-me to
    // 'representational' to get it right, demote to 'representational'
    const shownStage = event.craLevelShown;
    if (shownStage && CRA_ORDER[shownStage] < CRA_ORDER[state.craStages[op]]) {
      newCraStages = Object.freeze({ ...state.craStages, [op]: shownStage });
    }
  }

  if (event.toldMe) {
    // Gave up entirely — don't change CRA stage, but note it.
    // Repeated tell-me on the same operation should demote CRA to concrete.
    const tellMeCount = countRecentTellMe(newWindow, op);
    if (tellMeCount >= 2 && state.craStages[op] !== 'concrete') {
      newCraStages = Object.freeze({ ...state.craStages, [op]: 'concrete' });
    }
  }
}

const CRA_ORDER = { concrete: 0, representational: 1, abstract: 2 };
```

### Helper: countConsecutiveNoHintCorrect

```js
function countConsecutiveNoHintCorrect(window, operation, craStage) {
  let count = 0;
  for (let i = window.entries.length - 1; i >= 0; i--) {
    const e = window.entries[i];
    if (e.operation !== operation) continue;
    if (!e.correct || e.hintUsed || e.toldMe) break;
    if (e.craLevelShown && e.craLevelShown !== craStage) break;
    count++;
  }
  return count;
}
```

### Helper: countRecentTellMe

```js
function countRecentTellMe(window, operation) {
  return window.entries.filter(e =>
    e.operation === operation && e.toldMe
  ).length;
}
```

### How CRA stages feed into challenge creation

When the adapter creates a challenge, it sets the `renderHint.craStage` from the learner profile:

```js
// In _startChallengeFromDomain or the trigger functions:
const operation = challenge.operation;
const craStage = profileState.craStages[operation] || 'concrete';
const context = {
  source: 'robot',
  npcName: 'Sparky',
  renderHint: {
    craStage,                     // from learner profile for THIS operation
    answerMode: 'choice',         // from profile answerMode dial
    interactionType: 'quiz',
  },
};
```

This closes the loop:
1. Challenge starts at the kid's CRA stage for this operation
2. Kid uses show-me → CRA drops → they succeed at a lower level
3. Event records `craLevelShown` and `hintUsed`
4. Learner reducer reads these → adjusts `craStages[operation]`
5. Next challenge for this operation starts at the updated CRA stage

### The TEACHING_RETRY case becomes unnecessary

The old TODO at `TEACHING_RETRY` in the learner reducer was the placeholder for CRA tracking. With the new approach, CRA tracking happens inside `PUZZLE_ATTEMPTED` using `hintUsed`, `hintLevel`, `toldMe`, and `craLevelShown`. The `TEACHING_RETRY` case can be deleted — it served no purpose and the signals it was meant to capture now flow through the standard event fields.

### What the CRA renderer PR should include

Domain changes:
- Replace the `TEACHING_RETRY` TODO with CRA stage logic in `PUZZLE_ATTEMPTED` handler
- Add `countConsecutiveNoHintCorrect` and `countRecentTellMe` helpers
- Add `hintUsed`, `toldMe`, `craLevelShown` to rolling window entries (they're in events but not stored in the window)
- Tests: CRA promotion after 3 no-hint correct, CRA demotion on hint-assisted correct, CRA demotion on repeated tell-me

Adapter changes:
- Pass `profileState.craStages[operation]` into the challenge context renderHint
- Include `hintUsed`, `toldMe`, `craLevelShown` in events (already done for new fields, but verify window entries include them)

Presentation changes:
- QuizRenderer extraction (from `renderChallenge`)
- CRA concrete renderer (dots/stars alongside the question)
- CRA representational renderer (number line or tens/ones blocks)
- Show-me button → dispatches `SHOW_ME` to challenge reducer → application swaps renderer
- Tell-me button → dispatches `TELL_ME` → shows answer at concrete level

### Tests for CRA feedback loop

```
Learner reducer — CRA progression:
  - 'CRA promotes from concrete to representational after 3 no-hint correct'
  - 'CRA promotes from representational to abstract after 3 no-hint correct'
  - 'CRA does not promote above abstract'
  - 'CRA demotes when hint was needed and succeeded at lower level'
  - 'CRA demotes to concrete after 2 tell-me events for same operation'
  - 'CRA stages are tracked per-operation independently'
  - 'mixed operations: CRA for add can be abstract while sub is concrete'
  - 'hint-assisted correct at same CRA level does not demote'
  - 'no-hint streak resets on a wrong answer'
  - 'no-hint streak resets on a hint-used answer'
```

## Pluggable Renderers

The challenge state machine handles the lifecycle (phases, rewards, feedback, voice). The RENDERING is a separate concern — different challenge types look completely different but share the same lifecycle.

### The Renderer Interface

Every challenge renderer implements the same interface:

```js
ChallengeRenderer {
  // Render the current state to canvas
  render(ctx, challengeState, canvasW, canvasH, time): void

  // Handle click/tap at (x, y) — returns an action to dispatch, or null
  handleClick(x, y, challengeState): Action | null

  // Handle key press — returns an action to dispatch, or null
  handleKey(key, challengeState): Action | null

  // Cleanup (stop animations, release resources)
  dispose(): void
}
```

The renderer reads `challengeState` and draws. It never mutates state — it returns Actions that the application layer dispatches to the reducer. The lifecycle is the same for every renderer; only the visuals change.

### Renderer Types

```
src/presentation/renderers/
  quiz-renderer.js           # Current: multiple choice buttons (what we have now)
  cra-concrete-renderer.js   # Dots/stars/objects to count and group
  cra-repres-renderer.js     # Number line, tens/ones blocks, bar models
  cra-abstract-renderer.js   # Just the numbers (same as quiz but no choices — free input)
  drag-renderer.js           # Drag objects into groups (future: concrete answer mode)
  number-line-renderer.js    # Tap to jump on number line (future: representational answer mode)
  shop-renderer.js           # Purchase UI with embedded math (future: economy phase 2)
  puzzle-renderer.js         # Door codes, bridge weights, etc. (future: quest system)
```

### How the Application Layer Picks a Renderer

The challenge state includes `renderHint` — a suggestion from the domain about how to render based on CRA stage and answer mode. The application layer uses it to select a renderer:

```js
// On the challenge state (set by createChallengeState):
renderHint: {
  craStage: 'representational',   // from learner profile for this operation
  answerMode: 'choice',           // from learner profile
  interactionType: 'quiz',        // 'quiz' | 'puzzle' | 'shop' | 'drag' | 'number_line'
}
```

The application layer maps the hint to a renderer:

```js
function selectRenderer(renderHint) {
  // CRA stage determines the visual layer
  // answerMode determines the input mechanism
  // interactionType overrides both for special cases (shop, quest puzzle)

  if (renderHint.interactionType === 'shop') return new ShopRenderer();
  if (renderHint.interactionType === 'puzzle') return new PuzzleRenderer();

  switch (renderHint.craStage) {
    case 'concrete': return new CraConcreteRenderer(renderHint.answerMode);
    case 'representational': return new CraRepresRenderer(renderHint.answerMode);
    case 'abstract':
    default: return new QuizRenderer(renderHint.answerMode);
  }
}
```

### For Now: One Renderer

The initial implementation has ONE renderer: `QuizRenderer` — the existing `renderChallenge` logic extracted into the renderer interface. All challenges use it regardless of CRA stage or answer mode. This is the same UX the kids have now, just properly architectured.

Then we add renderers incrementally:
1. `QuizRenderer` (this PR — extract existing code)
2. `CraConcreteRenderer` (next: dots alongside the question, show-me drops to this)
3. `CraRepresRenderer` (next: number line / tens blocks alongside the question)
4. `DragRenderer` (future: drag objects to build the answer)
5. `ShopRenderer` (future: economy phase 2)
6. `PuzzleRenderer` (future: quest system door codes, bridge weights)

Each renderer is a separate file. Adding a new one never touches the lifecycle state machine or other renderers.

### How Show-Me Works With Renderers

"Show me!" doesn't swap the renderer mid-challenge. It dispatches a `SHOW_ME` action to the lifecycle reducer:

```js
case 'SHOW_ME': {
  const currentCra = state.renderHint.craStage;
  const lowerCra = currentCra === 'abstract' ? 'representational'
                 : currentCra === 'representational' ? 'concrete'
                 : 'concrete'; // already at bottom
  return Object.freeze({
    ...state,
    renderHint: { ...state.renderHint, craStage: lowerCra },
    hintUsed: true,
    hintLevel: (state.hintLevel || 0) + 1,
  });
}
```

The application layer sees `renderHint.craStage` changed, picks a new renderer, and re-renders. The transition can be animated (the old renderer fades out, the new one fades in). The state machine doesn't know about rendering — it just tracks which CRA level was requested.

### How Quest Puzzles Will Work

A quest step that says "the door code is ? + 6 = 13" creates a challenge with:

```js
createChallengeState(challenge, {
  source: 'quest',
  questId: 'crystal_caves_3',
  npcName: 'Door',
  renderHint: {
    interactionType: 'puzzle',  // overrides CRA-based selection
    craStage: profile.craStages.number_bond,
    answerMode: 'free_input',   // door codes are typed, not multiple choice
  },
});
```

The `PuzzleRenderer` draws a door with a keypad. The lifecycle is identical — `ANSWER_SUBMITTED`, correct → `complete`, wrong → `feedback`. Same reducer, different visuals.

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
