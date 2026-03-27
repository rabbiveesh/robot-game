# Challenge Simulator — CLI Tool

## Goal

A standalone CLI tool that simulates the challenge lifecycle domain in isolation. Exercises every action type (answer, retry, show-me, tell-me, voice) with configurable kid behavior. Composable with the existing learning simulator via pipes.

## Design: Two Tools That Compose

```
tools/simulate.js          — learning domain (intake, profile evolution, band progression)
tools/simulate-challenge.js — challenge domain (lifecycle, CRA hints, voice, retries)
```

Each tool works standalone. Together they simulate a full play session at both levels.

### Standalone usage

```bash
# Simulate a single challenge interaction
node tools/simulate-challenge.js --answer correct --cra abstract

# Simulate a kid who needs hints
node tools/simulate-challenge.js --answer wrong,show-me,wrong,tell-me --cra abstract

# Simulate voice input with low confidence
node tools/simulate-challenge.js --answer voice:0.6:correct --cra representational

# Simulate 20 random challenges with a kid profile
node tools/simulate-challenge.js --profile hesitant --count 20

# Just dump the state transitions for one challenge
node tools/simulate-challenge.js --answer wrong,retry,correct --trace
```

### Composed usage

The learning simulator can pipe challenge context into the challenge simulator:

```bash
# Learning sim outputs one line per challenge as JSON
node tools/simulate.js --profile gifted --output json | \
  node tools/simulate-challenge.js --stdin --profile confident

# Or: learning sim calls challenge sim internally
node tools/simulate.js --profile gifted --challenge-detail
```

The `--challenge-detail` flag on the learning simulator runs each challenge through the challenge simulator instead of just flipping a coin for correct/wrong. This gives the full combined output:

```
#14  add_carry  band:6  What is 28 + 15?
     abstract → SHOW_ME → representational
     answered 42 (wrong, attempt 1)
     answered 43 (correct, attempt 2, hint used)
     reward: 1 dum dum
     profile: band:6 sw:0.40 scaffolding:0.28 CRA:add→representational
```

## Challenge Simulator

### File: `tools/simulate-challenge.js`

Imports from `src/domain/challenge/` (pure, no browser). Exercises `createChallengeState` and `challengeReducer`.

### Input: Action Sequences

An action sequence describes what the simulated kid does:

```
correct              → ANSWER_SUBMITTED with correct answer
wrong                → ANSWER_SUBMITTED with wrong answer
show-me              → SHOW_ME
tell-me              → TELL_ME
retry                → RETRY
voice:0.9:correct    → VOICE_LISTEN_START → VOICE_RESULT(confidence=0.9) → ANSWER_SUBMITTED(correct)
voice:0.6:correct    → VOICE_LISTEN_START → VOICE_RESULT(confidence=0.6) → VOICE_CONFIRM(yes) → ANSWER_SUBMITTED(correct)
voice:0.3:wrong      → VOICE_LISTEN_START → VOICE_RESULT(confidence=0.3, retry)
voice:null            → VOICE_LISTEN_START → VOICE_RESULT(number=null, retry)
voice:error:no-speech → VOICE_LISTEN_START → VOICE_ERROR(no-speech)
```

### CLI Flags

```
--answer <sequence>    Comma-separated action sequence (e.g., wrong,show-me,correct)
--cra <stage>          Starting CRA stage: abstract, representational, concrete (default: abstract)
--answer-mode <mode>   Starting answer mode: choice, free_input, etc. (default: choice)
--interaction <type>   Interaction type: quiz, puzzle, shop (default: quiz)
--operation <op>       Math operation: add, sub, multiply, divide (default: add)
--band <n>             Band for number generation (default: 3)
--profile <name>       Simulated kid profile (see below)
--count <n>            Generate N random challenges with profile behavior (default: 1)
--trace                Show every state transition, not just summary
--seed <n>             RNG seed for deterministic output
--stdin                Read challenge context from stdin (JSON lines, for piping)
--output json          Output as JSON (for piping to learning simulator)
```

### Output: Trace Mode

```
$ node tools/simulate-challenge.js --answer wrong,show-me,wrong,tell-me --cra abstract --trace

Challenge: What is 28 + 15? (add_carry, band 6)
  State: presented (abstract, choice)

  Action: ANSWER_SUBMITTED (42, wrong)
  State: feedback (attempt 1/2)
  Feedback: "Hmm, not quite! Try again!"

  Action: SHOW_ME
  State: presented (representational, choice)  ← CRA dropped
  hintUsed: true, hintLevel: 1

  Action: ANSWER_SUBMITTED (42, wrong)
  State: teaching (attempt 2/2)
  Feedback: "Let's figure it out together!"

  Action: TELL_ME
  State: teaching (concrete)  ← CRA dropped to concrete
  toldMe: true
  Feedback: "The answer is 43!"

  Action: TEACHING_COMPLETE
  State: complete

  Result: incorrect, 2 attempts, hint used (level 1→concrete), told me, no reward
```

### Output: Summary Mode (default)

```
$ node tools/simulate-challenge.js --answer wrong,show-me,correct --cra abstract

  28 + 15 = ?  ✗→💡→✓  abstract→representational  reward: dum_dum  attempts: 2
```

### Output: JSON Mode (for piping)

```json
{
  "correct": true,
  "attempts": 2,
  "hintUsed": true,
  "hintLevel": 1,
  "toldMe": false,
  "finalCra": "representational",
  "reward": { "type": "dum_dum", "amount": 1 },
  "voiceUsed": false,
  "actions": ["ANSWER_SUBMITTED:wrong", "SHOW_ME", "ANSWER_SUBMITTED:correct"]
}
```

### Simulated Kid Profiles

```js
const CHALLENGE_PROFILES = {
  confident: {
    name: 'Confident kid',
    firstAttemptAccuracy: 0.85,  // chance of correct on first try
    retryAccuracy: 0.95,         // chance of correct after feedback
    usesShowMe: 0.05,            // rarely asks for hints
    usesTellMe: 0.01,            // almost never gives up
    usesVoice: 0.3,              // sometimes uses voice
    voiceConfidence: [0.7, 0.95], // high confidence range
  },

  hesitant: {
    name: 'Hesitant kid',
    firstAttemptAccuracy: 0.50,
    retryAccuracy: 0.70,
    usesShowMe: 0.40,            // frequently asks for help
    usesTellMe: 0.10,            // sometimes gives up
    usesVoice: 0.1,
    voiceConfidence: [0.4, 0.7],
  },

  explorer: {
    name: 'Explorer kid',
    firstAttemptAccuracy: 0.70,
    retryAccuracy: 0.80,
    usesShowMe: 0.60,            // loves pressing show-me to see the visuals
    usesTellMe: 0.02,
    usesVoice: 0.5,              // loves talking
    voiceConfidence: [0.6, 0.9],
  },

  frustrated: {
    name: 'Frustrated kid',
    firstAttemptAccuracy: 0.30,
    retryAccuracy: 0.40,
    usesShowMe: 0.20,
    usesTellMe: 0.35,            // gives up often
    usesVoice: 0.05,
    voiceConfidence: [0.3, 0.6],
  },
};
```

The profile drives a random action sequence per challenge:

```js
function simulateChallenge(challenge, profile, rng) {
  let state = createChallengeState(challenge, context);
  const actions = [];

  while (state.phase !== 'complete') {
    if (state.phase === 'presented') {
      // Maybe use show-me first
      if (rng() < profile.usesShowMe && state.renderHint.craStage !== 'concrete') {
        state = challengeReducer(state, { type: 'SHOW_ME' });
        actions.push('SHOW_ME');
        continue;
      }
      // Maybe use voice
      if (rng() < profile.usesVoice) {
        // ... voice simulation
        continue;
      }
      // Submit answer
      const acc = state.attempts === 0 ? profile.firstAttemptAccuracy : profile.retryAccuracy;
      const correct = rng() < acc;
      const answer = correct ? challenge.correctAnswer : challenge.correctAnswer + 1;
      state = challengeReducer(state, { type: 'ANSWER_SUBMITTED', answer });
      actions.push(correct ? 'correct' : 'wrong');
    }
    else if (state.phase === 'feedback') {
      // Maybe give up
      if (rng() < profile.usesTellMe) {
        state = challengeReducer(state, { type: 'TELL_ME' });
        actions.push('TELL_ME');
      } else {
        state = challengeReducer(state, { type: 'RETRY' });
        actions.push('RETRY');
      }
    }
    else if (state.phase === 'teaching') {
      state = challengeReducer(state, { type: 'TEACHING_COMPLETE' });
      actions.push('TEACHING_COMPLETE');
    }
  }

  return { state, actions };
}
```

## Composition Protocol

### Learning sim → Challenge sim

The learning simulator's `--output json` flag outputs one JSON line per challenge:

```json
{"band":6,"operation":"add","subSkill":"add_carry","craStage":"abstract","answerMode":"choice"}
```

The challenge simulator's `--stdin` flag reads these and simulates each:

```bash
node tools/simulate.js --profile gifted --output json | \
  node tools/simulate-challenge.js --stdin --profile hesitant --output json
```

Output is enriched JSON lines with challenge-level detail added:

```json
{"band":6,"operation":"add","correct":true,"attempts":2,"hintUsed":true,"hintLevel":1,"toldMe":false,"finalCra":"representational"}
```

### `--challenge-detail` on learning sim

For convenience, `--challenge-detail` flag on the existing simulator runs both domains internally without piping:

```bash
node tools/simulate.js --profile gifted --challenge-detail --challenge-profile hesitant
```

This uses the learning domain's simulated kid (gifted) for band/accuracy decisions and the challenge domain's simulated kid (hesitant) for interaction behavior (hints, voice, retries). The combined output shows both:

```
#14  add_carry  c:6→6  What is 28 + 15?
     abstract → SHOW_ME → representational
     ✗ 42 (attempt 1) → RETRY → ✓ 43 (attempt 2, hint used)
     reward: dum_dum  profile: band:6 sw:0.40 scaffolding:0.28
```

## Package.json

```json
"scripts": {
  "simulate": "node tools/simulate.js",
  "simulate:challenge": "node tools/simulate-challenge.js",
  "simulate:full": "node tools/simulate.js --challenge-detail"
}
```

## Tests

The challenge simulator itself doesn't need unit tests — it's a tool that exercises tested domain code. But the profile-driven simulation logic should have a few sanity checks:

```
tools/test/simulate-challenge.test.js (optional, nice-to-have)

  - 'confident profile completes most challenges on first try'
  - 'frustrated profile uses tell-me frequently'
  - 'explorer profile triggers show-me frequently'
  - 'voice simulation produces valid action sequences'
  - 'deterministic with seeded rng'
```

## Acceptance Criteria

1. `node tools/simulate-challenge.js --answer correct` runs and shows a completed challenge
2. `node tools/simulate-challenge.js --answer wrong,show-me,correct --trace` shows full state transitions
3. `node tools/simulate-challenge.js --profile hesitant --count 10` simulates 10 challenges with profile behavior
4. `node tools/simulate-challenge.js --answer voice:0.6:correct --trace` shows voice confidence → confirm → submit flow
5. Piping works: learning sim `--output json` → challenge sim `--stdin`
6. Combined mode: `--challenge-detail` on learning sim shows both domains
7. All outputs are deterministic with `--seed`
