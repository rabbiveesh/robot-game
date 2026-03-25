# CLI Intake Simulator — Implementation Spec

## Goal

A node script that simulates a kid going through the intake quiz and subsequent challenges, printing the profile evolution to stdout. Used by designers/parents to understand what the adaptive system does without needing a real kid or a browser.

## Usage

```bash
# Simulate a gifted 4-year-old (fast, mostly correct, gets bored)
node tools/simulate.js --profile gifted

# Simulate a struggling kid (slow, mostly wrong, needs patience)
node tools/simulate.js --profile struggling

# Simulate a 7-year-old (fast, correct up to band 7, then struggles)
node tools/simulate.js --profile seven-year-old

# Custom: specify response patterns directly
node tools/simulate.js --intake correct,correct,wrong,correct --speed fast --questions 30

# Just intake, no follow-up questions
node tools/simulate.js --profile gifted --intake-only
```

## Output

Colored terminal output showing each event and the resulting profile state:

```
═══ INTAKE (Sparky's Calibration) ═══

 Q1  band:3  7 + 5 = ?   ✓  1.8s  → next band: 5
 Q2  band:5  2 × 8 = ?   ✓  2.1s  → next band: 7
 Q3  band:7  63 + 28 = ? ✗  8.4s  → next band: 6
 Q4  band:6  42 - 17 = ? ✓  3.2s  → next band: 8

 Intake result:
   Placed at band: 6 (+/- <50)
   Pace: 0.60   Scaffolding: 0.45
   Streak to promote: 3
   Text speed: 0.035

═══ PLAY SESSION (30 challenges) ═══

  #1  add   band:6  23 + 14 = 37  ✓  2.1s  streak:1   pace:0.60
  #2  sub   band:6  41 - 18 = 23  ✓  3.4s  streak:2   pace:0.60
  #3  add   band:6  35 + 12 = 47  ✓  1.9s  streak:3   pace:0.60  ⬆ PROMOTED → band:7
  #4  sub   band:7  82 - 35 = 47  ✗  7.2s  streak:-1  pace:0.58
  #5  add   band:7  54 + 38 = 92  ✗  9.1s  streak:-2  pace:0.55  ⬇ DEMOTED → band:6
  ...
  #18 add   band:6  29 + 16 = 45  ✗  0.4s  [BOREDOM — not penalized]  streak:2
  ...
  #22 sub   band:6  38 - 19 = ?   ✗  8s
  #23 sub   band:6  31 - 14 = ?   ✗  9s
  #24 sub   band:6  26 - 11 = ?   ✗  11s   😰 FRUSTRATION: high → drop_band
  ...

═══ FINAL PROFILE ═══

  Band: 5 (x1 x2)        Play time: 30 questions
  Pace: 0.48              Scaffolding: 0.52
  Frustration events: 1
  Accuracy: 67% (20/30)

  Operation breakdown:
    add:    82% (14/17)   strength
    sub:    46% (6/13)    growth area
    mult:   --
    div:    --
```

## Simulated Kid Profiles

Each profile defines how the simulated kid "behaves" — their accuracy at each band, response speed, and quirks.

```js
const PROFILES = {
  gifted: {
    name: 'Gifted 4yo',
    // Probability of correct answer at each band
    accuracy: { 1: 0.99, 2: 0.95, 3: 0.90, 4: 0.85, 5: 0.75, 6: 0.50, 7: 0.30, 8: 0.20, 9: 0.10, 10: 0.05 },
    // Response time range [min, max] in ms
    speed: { fast: [800, 2500], normal: [1500, 4000] },
    // Chance of boredom wrong (fast wrong on easy questions)
    boredomChance: 0.15,
    skipsText: true,
  },

  struggling: {
    name: 'Struggling 5yo',
    accuracy: { 1: 0.80, 2: 0.60, 3: 0.40, 4: 0.20, 5: 0.10, 6: 0.05, 7: 0.02, 8: 0.01, 9: 0.01, 10: 0.01 },
    speed: { fast: [4000, 8000], normal: [6000, 12000] },
    boredomChance: 0,
    skipsText: false,
  },

  'seven-year-old': {
    name: 'Typical 7yo',
    accuracy: { 1: 0.99, 2: 0.98, 3: 0.95, 4: 0.92, 5: 0.85, 6: 0.80, 7: 0.70, 8: 0.55, 9: 0.35, 10: 0.20 },
    speed: { fast: [1000, 3000], normal: [2000, 5000] },
    boredomChance: 0.10,
    skipsText: true,
  },

  '2e': {
    name: '2e kid (high reasoning, slow processing)',
    accuracy: { 1: 0.95, 2: 0.95, 3: 0.90, 4: 0.88, 5: 0.80, 6: 0.75, 7: 0.65, 8: 0.50, 9: 0.30, 10: 0.15 },
    speed: { fast: [5000, 9000], normal: [7000, 15000] },
    boredomChance: 0.05,
    skipsText: false,  // slow reader but deep thinker
  },
};
```

## Implementation

### File: `tools/simulate.js`

Node script. Imports directly from `src/domain/learning/` (ES modules). No browser deps, no adapter, no canvas.

```js
import { createProfile, learnerReducer } from '../src/domain/learning/index.js';
import { generateChallenge } from '../src/domain/learning/challenge-generator.js';
import { generateIntakeQuestion, processIntakeResults, nextIntakeBand } from '../src/domain/learning/intake-assessor.js';
import { detectFrustration } from '../src/domain/learning/frustration-detector.js';
import { accuracy } from '../src/domain/learning/rolling-window.js';
```

### Flow

1. Parse CLI args (profile name, question count, flags)
2. Create a simulated kid from the profile
3. Run intake: 4 questions, simulate answers based on profile accuracy at each band
4. Process intake → get initial profile
5. Run N challenges, simulating answers:
   - Generate challenge from current profile
   - Simulate answer: correct with probability `profile.accuracy[band]`
   - Simulate response time: random within speed range
   - Simulate boredom: if correct streak > 3 and easy band, chance of fast wrong
   - Feed event into reducer
   - Run frustration detection
   - Print the line
6. Print final profile summary

### Seeded RNG

Use a seeded PRNG so simulations are reproducible. Default seed from profile name, overridable with `--seed`.

```bash
node tools/simulate.js --profile gifted --seed 42
# Same output every time
```

### Add to package.json

```json
"scripts": {
  "simulate": "node tools/simulate.js"
}
```

Then: `npm run simulate -- --profile gifted`

## Acceptance Criteria

1. All 4 built-in profiles produce plausible output
2. Gifted profile promotes quickly, hits a ceiling, system speeds up pace
3. Struggling profile stays at low bands, system slows down, frustration triggers band drops
4. 2e profile: high bands but slow pace dial — system correctly separates reasoning from speed
5. `--intake-only` shows just the placement
6. `--seed` makes output deterministic
7. Zero browser dependencies — runs in pure Node
