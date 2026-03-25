# MVP: Adaptive Engine — Implementation Spec

## Goal

Replace the current flat adaptive system with the reducer-based Learning domain. The existing game stays playable. A parent can watch the profile evolve via a debug overlay.

**What we're validating**: Does the adaptive system correctly identify what a kid needs and adjust accordingly? We watch real kids play and see if the dials move in sensible ways.

## What Changes

| Component | Before | After |
|-----------|--------|-------|
| Skill tracking | `SKILL.math.band`, `SKILL.math.streak` (global mutable) | `LearnerProfile` (immutable, reducer) |
| Challenge generation | `generateMathChallenge()` reads global `SKILL` | `ChallengeGenerator.generate(profile, rng)` pure function |
| Difficulty adjustment | Streak of 3 up, streak of -2 down (hardcoded) | Configurable via profile dials (`streakToPromote`, `wrongsBeforeTeach`) |
| Frustration handling | None | `FrustrationDetector` analyzes rolling window |
| Game start | Directly into gameplay | Intake quiz (4 math questions) → profile initialization |
| Parent visibility | None | Debug overlay (press P) showing profile state |

## What Does NOT Change

- Map, tiles, sprites, movement, camera — untouched
- NPC dialogue, TTS, Claude API — untouched
- Save slot UI on title screen — untouched (but save data schema adds profile)
- Secret areas — untouched
- Existing game files stay in place — new code lives in `src/`

## Deliverables

### 1. Project Scaffolding

Set up the build tooling alongside the existing game:

```
package.json          # vitest, rollup, rollup-plugin-node-resolve
vitest.config.js
rollup.config.js
src/
  domain/
    learning/
      index.js
      learner-profile.js
      challenge-generator.js
      intake-assessor.js
      frustration-detector.js
      rolling-window.js
      operation-stats.js
test/
  domain/
    learning/
      learner-profile.test.js
      challenge-generator.test.js
      intake-assessor.test.js
      frustration-detector.test.js
```

The existing flat JS files (sprites.js, world.js, etc.) continue to work via `<script>` tags. The new `src/` code is ES modules tested with vitest and (for now) loaded into the game via a thin adapter.

### 2. Learning Domain Implementation

All code in `src/domain/learning/`. All functions are pure. All state is immutable (`Object.freeze`). All randomness is injected.

#### learner-profile.js

The state shape and the root reducer.

```js
export function createProfile(overrides = {}) {
  return Object.freeze({
    mathBand: 1,
    streak: 0,
    pace: 0.5,
    scaffolding: 0.5,
    challengeFreq: 0.5,
    streakToPromote: 3,
    wrongsBeforeTeach: 2,
    hintVisibility: 0.5,
    textSpeed: 0.035,
    framingStyle: 0.5,
    representationStyle: 0.5,
    craStages: {
      add: 'concrete',
      sub: 'concrete',
      multiply: 'concrete',
      divide: 'concrete',
      number_bond: 'concrete',
    },
    intakeCompleted: false,
    ...overrides,
  });
}

export function learnerReducer(state, event) {
  switch (event.type) {
    case 'PUZZLE_ATTEMPTED': ...
    case 'TEACHING_RETRY': ...
    case 'BEHAVIOR': ...
    case 'FRUSTRATION_DETECTED': ...
    case 'INTAKE_COMPLETED': ...
    default: return state;
  }
}
```

**Key behaviors to implement and test:**

- `PUZZLE_ATTEMPTED` + correct: increment streak, promote band if streak >= streakToPromote
- `PUZZLE_ATTEMPTED` + wrong: decrement streak, demote band if streak <= -2
- `PUZZLE_ATTEMPTED`: push to rolling window (max 20), track operation stats
- `BEHAVIOR` + `text_skipped`: nudge pace up, textSpeed down
- `BEHAVIOR` + `rapid_clicking`: reduce wrongsBeforeTeach (mashing = frustration)
- `FRUSTRATION_DETECTED` + high: drop band, reduce wrongsBeforeTeach to 1, slow pace
- `INTAKE_COMPLETED`: set all dials from intake results
- Boredom detection: right-right-wrong(fast)-right pattern should NOT count the fast wrong as a real failure

#### rolling-window.js

```js
export function createWindow(entries = [], maxSize = 20) {
  return Object.freeze({ entries: Object.freeze(entries.slice(-maxSize)), maxSize });
}

export function pushEntry(window, entry) {
  const entries = [...window.entries, entry].slice(-window.maxSize);
  return createWindow(entries, window.maxSize);
}

export function accuracy(window) { ... }
export function avgResponseTime(window) { ... }
export function consecutiveWrong(window) { ... }
export function operationAccuracy(window, operation) { ... }
```

#### operation-stats.js

```js
export function createOperationStats() {
  return Object.freeze({
    add: { correct: 0, attempts: 0 },
    sub: { correct: 0, attempts: 0 },
    multiply: { correct: 0, attempts: 0 },
    divide: { correct: 0, attempts: 0 },
    number_bond: { correct: 0, attempts: 0 },
  });
}

export function recordOperation(stats, operation, correct) {
  // Returns new frozen stats object
}
```

#### challenge-generator.js

Pure function. Takes profile + rng, returns a challenge. Does NOT modify state.

```js
export function generateChallenge(profile, rng) {
  // 1. Pick operation (weighted: 60% strengths, 40% growth areas)
  // 2. Pick numbers (scaled to mathBand)
  // 3. Generate wrong answers (spread scales with answer magnitude)
  // 4. Return: { question, correctAnswer, choices, operation, band, numbers: {a, b, op} }
}
```

Reuses the math generation logic from the current `dialogue.js` but restructured as a pure function with injected rng.

**Bands 1-10 (same as current):**
1. Add within 5
2. +/- within 10
3. +/- within 15 + number bonds
4. +/- within 20 + number bonds
5. Multiply x1, x2
6. +/- within 50
7. +/- within 100
8. Multiply x1-5
9. Multiply x1-12
10. Division (inverse of x1-12)

#### intake-assessor.js

Runs the intake logic. Stateless.

```js
export function generateIntakeQuestion(currentBand, questionIndex, rng) {
  // Returns a challenge at the given band
}

export function processIntakeResults(answers) {
  // answers: [{ band, correct, responseTimeMs, skippedText }]
  // Returns: { mathBand, pace, scaffolding, streakToPromote, ... }
  // Logic:
  //   - Start at band 3
  //   - Correct → next question band +2
  //   - Wrong → next question band -1
  //   - Final band = last correct (min 1)
  //   - Fast responses (< 3s avg) → higher pace
  //   - Slow responses (> 8s avg) → lower pace, more scaffolding
  //   - Skipped text → faster textSpeed
}
```

#### frustration-detector.js

Analyzes rolling window + behavioral signals.

```js
export function detectFrustration(window, recentBehaviors = []) {
  // Returns: { level: 'none'|'mild'|'high', recommendation: string }
  //
  // Signals:
  //   HIGH: 3+ consecutive wrong on same band
  //   HIGH: rapid_clicking behavior in last 3 events
  //   HIGH: accuracy < 40% in rolling window
  //   MILD: long idle (>15s) after wrong answer
  //   MILD: chose easier path twice in a row
  //   NONE: otherwise
  //
  // Recommendations:
  //   'continue' — all good
  //   'encourage' — say something nice, same difficulty
  //   'drop_band' — too hard, go down
  //   'switch_to_chat' — take a break from challenges
  //   'offer_easier_path' — give them an out
}
```

### 3. Tests

Every domain function gets tested. The test file structure mirrors src:

```
test/domain/learning/
  learner-profile.test.js     # Reducer transitions
  challenge-generator.test.js # Output correctness, band scaling, operation weighting
  intake-assessor.test.js     # Placement accuracy, dial calibration
  frustration-detector.test.js # Signal detection, recommendation logic
  rolling-window.test.js      # Window operations, accuracy calc
```

**Key test scenarios:**

```js
// Learner profile
- 'promotes band after N correct in a row (configurable N)'
- 'demotes band after 2 wrong in a row'
- 'does not demote below band 1'
- 'does not promote above band 10'
- 'boredom pattern: fast wrong between corrects is not a real failure'
- 'text_skipped behavior increases pace and textSpeed'
- 'frustration event drops band and reduces wrongsBeforeTeach'
- 'state is frozen — original state unchanged after reduction'

// Challenge generator
- 'generates addition for band 1'
- 'generates add/sub for band 2'
- 'generates multiplication for band 9'
- 'generates division for band 10'
- 'wrong answers are close to correct answer (within spread)'
- 'wrong answers scale spread for larger numbers'
- 'with seeded rng, output is deterministic'
- 'weights toward strength operations (60/40 split)'

// Intake assessor
- 'places kid in correct band after all-correct intake'
- 'places kid in band 1 after all-wrong intake'
- 'fast responder gets higher pace dial'
- 'slow responder gets lower pace and more scaffolding'
- 'text skipper gets faster textSpeed'

// Frustration detector
- 'detects high frustration after 3 consecutive wrong'
- 'detects high frustration on rapid clicking'
- 'detects mild frustration on long idle after wrong'
- 'returns none when accuracy is healthy'
- 'recommends drop_band on high frustration'
- 'recommends encourage on mild frustration'
```

### 4. Game Integration (Adapter Layer)

A thin adapter in the existing game wires the new domain into the old system. This is NOT in `src/` — it lives alongside the legacy files as a bridge.

**`adapter.js`** — loaded via `<script>` after the legacy files and the bundled domain:

```js
// Replace the global SKILL tracking with the new profile
let profileState = createProfile();
let eventLog = [];

// Monkey-patch the existing challenge generator
const _oldGenerateMath = generateMathChallenge;
generateMathChallenge = function() {
  const seededRng = Math.random; // production rng
  return generateChallenge(profileState, seededRng);
};

// Hook into the existing selectChallengeChoice to record events
const _oldSelect = selectChallengeChoice;
selectChallengeChoice = function(index, time) {
  const startTime = CHALLENGE._shownAt || time;
  _oldSelect(index, time);

  const event = {
    type: 'PUZZLE_ATTEMPTED',
    correct: CHALLENGE.choices[index]?.correct,
    operation: CHALLENGE.teachingData?.op || 'add',
    band: profileState.mathBand,
    responseTimeMs: (time - startTime) * 1000,
    attemptNumber: CHALLENGE.attempts,
  };
  profileState = learnerReducer(profileState, event);
  eventLog.push(event);
};

// ... similar patches for teaching mode, behavior signals, etc.
```

This is intentionally ugly — it's a bridge, not architecture. It will be deleted when we do the full migration.

### 5. Intake Quiz Integration

Hook into `initGame` — after the welcome dialogue, run the intake sequence before regular play begins.

The intake uses the existing challenge UI (dialogue box + choice buttons). It's 4 questions with Sparky framing it as "calibration." After the 4th question, `processIntakeResults` produces the initial profile and play begins.

### 6. Parent Debug Overlay

Press `P` during gameplay to toggle a semi-transparent overlay showing:

```
── Learner Profile ──────────────
Band: 4 (+/- <20)     Streak: 2/3
Pace: 0.62   Scaffolding: 0.45
Frustration: none
Rolling accuracy: 78% (18/20)
──────────────────────────────────
add:     85% (17/20)  CRA: representational
sub:     60% (6/10)   CRA: concrete
multiply: --          CRA: concrete
──────────────────────────────────
Last 5 events:
  ✓ add  band:4  1.2s
  ✓ add  band:4  0.9s
  ✗ sub  band:4  6.1s
  ✓ sub  band:4  3.4s  (retry after teaching)
  ✓ add  band:4  1.1s
```

This is rendered as canvas text over the game. Simple, ugly, functional. It's for us, not for the kid.

## Acceptance Criteria

The MVP is done when:

1. `npm test` passes — all domain tests green
2. New game starts with intake quiz (4 questions)
3. Profile adapts during play — band goes up on streaks, down on failures
4. Frustration detection triggers — after 3 wrong in a row, system backs off
5. Parent overlay (P key) shows live profile state
6. Existing game features still work — movement, NPCs, save/load, TTS, secret areas
7. Save data includes the new profile (backward compatible — old saves still load with default profile)

## Planned Evolution: Signal Interpreter

After playtesting, we'll insert a Signal Interpreter between raw UI input and the reducer. This layer adjusts for confounders (mouse skill, misclicks, distraction, parent help). For now, the adapter constructs events directly from UI state — **keep all event construction in the adapter, in one place**, so inserting the interpreter later is a clean cut.

See `docs/architecture-spec.md` for full Signal Interpreter design.

## Non-Goals for This MVP

- Signal Interpreter / confounder adjustment (next iteration — need playtesting data first)
- Quest system rewrite (future)
- Story-embedded puzzles (future)
- CRA visual representations in teaching mode (future — current dot system stays)
- Phonics removal (future — leave it in, it doesn't break anything)
- Full project layout migration (future — just src/domain/learning/ for now)
- Parent dashboard UI (future — debug overlay is sufficient)
- Event bus (future — direct calls for now)
