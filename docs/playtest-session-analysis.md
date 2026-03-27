# Playtest Session Analysis — Bugs & Fixes

**Player:** 7yo playtester
**Session:** 2026-03-26, ~23 minutes, 21 problems across 2 sessions
**Result:** Band 10, 94% division accuracy, system has nothing left for her

## What Happened

The playtester aced the intake, promoted quickly to band 10, and spent the rest of her time doing division problems. She's 94% on division (including div_hard) but the system never tested her on the things she actually needs practice on — multiplication tables (only 1 attempt) and carrying/borrowing (zero attempts). She's at the ceiling doing the same operation on repeat.

The adaptive system correctly identified she's strong. It incorrectly concluded she's done learning.

## Bug 1: Rolling Window Not Persisted

### Problem

`rollingWindow: { entries: [], maxSize: 20 }` in the saved profile. After load, the reducer has zero evidence. It needs 4+ attempts before promotion/demotion logic can fire. All of the previous session's learning is lost.

### Root Cause

The adapter's `gatherSaveData` serializes profile dials but the `createProfile` overrides path in `loadFromSlot` creates a fresh window. The window entries from `profileState.rollingWindow` aren't included in the serialized `learnerProfile` object.

### Fix

In `adapter.js`, the `gatherSaveData` monkey-patch must include rolling window entries:

```js
data.learnerProfile = {
  ...existingFields,
  rollingWindowEntries: profileState.rollingWindow.entries,  // ADD THIS
};
```

And in `loadFromSlot`, restore them:

```js
profileState = createProfile({
  ...data.learnerProfile,
  rollingWindow: createWindow(data.learnerProfile.rollingWindowEntries || []),
});
```

### Tests

```
- 'save and load preserves rolling window entries'
- 'load with missing rollingWindowEntries creates empty window'
- 'loaded window respects maxSize (truncates if old save had more)'
```

## Bug 2: Response Time Not Capped

### Problem

First 4 events have response times of 94-148 seconds. The playtester wasn't thinking for 2 minutes — she walked around, explored, came back. `challengeShownAt` is set when the challenge UI opens and never resets. Any analysis that uses response time (pace adaptation, frustration detection based on slow responses, intake speed assessment) is corrupted by these outliers.

### Fix

Cap response time at a sane maximum. Any `responseTimeMs` above the cap is replaced with `null` (meaning "we don't know how long they actually thought").

```js
const RESPONSE_TIME_CAP_MS = 30000; // 30 seconds

// In the adapter, when constructing events:
const rawResponseTime = performance.now() - challengeShownAt;
const responseTimeMs = rawResponseTime > RESPONSE_TIME_CAP_MS ? null : rawResponseTime;
```

The reducer, frustration detector, and rolling window helpers already handle `null` response times (they filter them out). So this is just a one-line change in the adapter's event construction.

### Also

Reset `challengeShownAt` when a challenge is dismissed without answering (kid walks away, dialogue closes, etc.). Currently it persists across interactions.

### Tests

```
- 'response time > 30s is recorded as null'
- 'response time <= 30s is recorded as-is'
- 'avgResponseTime ignores null entries'  (already exists)
```

## Bug 3: Scaffolding and Pace Don't Adapt During Play

### Problem

The playtester's scaffolding is 0.7 (high) and pace is 0.9 — set by intake and never updated. She's 94% accurate at band 10 but the system still thinks she needs heavy scaffolding because her intake response times were moderate.

The `PUZZLE_ATTEMPTED` case in the reducer adjusts `spreadWidth` on sustained good performance but never touches `scaffolding` or `pace` (except via `BEHAVIOR` events like text_skipped and `FRUSTRATION_DETECTED`).

### Fix

Add ongoing adaptation of scaffolding and pace in the `PUZZLE_ATTEMPTED` handler, based on rolling window performance:

```js
// After band/spread logic, before return:

// Nudge scaffolding down on sustained high accuracy (kid doesn't need help)
if (newWindow.entries.length >= 10) {
  const acc = accuracy(newWindow);
  if (acc > 0.85 && state.scaffolding > 0.1) {
    newScaffolding = Math.max(0, state.scaffolding - 0.03);
  } else if (acc < 0.5 && state.scaffolding < 0.9) {
    newScaffolding = Math.min(1, state.scaffolding + 0.05);
  }
}

// Nudge pace up on fast correct answers, down on slow ones
if (event.responseTimeMs != null && event.correct) {
  if (event.responseTimeMs < 3000 && state.pace < 1.0) {
    newPace = Math.min(1, state.pace + 0.02);
  } else if (event.responseTimeMs > 10000 && state.pace > 0) {
    newPace = Math.max(0, state.pace - 0.02);
  }
}
```

Small nudges (0.02-0.05) so it takes 10-20 problems to shift meaningfully. The intake sets the starting point; play adjusts from there.

### Tests

```
- 'scaffolding decreases on sustained high accuracy (>85% over 10+)'
- 'scaffolding increases on sustained low accuracy (<50% over 10+)'
- 'scaffolding does not go below 0 or above 1'
- 'pace increases on fast correct answers (<3s)'
- 'pace decreases on slow correct answers (>10s)'
- 'pace does not adjust on null responseTimeMs'
```

## Bug 4: Band 10 Is a Monotony Trap

### Problem

`BAND_OPERATIONS[10] = ['divide']`. A kid at band 10 gets only division. The playtester did 17 division problems across 2 sessions. With spread 0.25, she occasionally gets band 9 multiplication, but the spread is tight because it was tightened after promotion and hasn't widened enough.

She's 94% on division but has 0 attempts at carrying, borrowing, number bonds at high numbers, or multi-step problems. The system never tested whether she can do 78-39 (sub_borrow) or 47+28 (add_carry) because the band structure sends her straight to division and keeps her there.

### Fix: Multi-Operation High Bands

Bands 9 and 10 should mix operations, not specialize. A kid at band 10 should be getting hard problems from ALL operations, not just division.

```js
const BAND_OPERATIONS = {
  1: ['add'],
  2: ['add', 'sub'],
  3: ['add', 'sub', 'number_bond'],
  4: ['add', 'sub', 'number_bond'],
  5: ['multiply'],
  6: ['add', 'sub'],
  7: ['add', 'sub'],
  8: ['multiply'],
  9: ['add', 'sub', 'multiply'],           // CHANGED: was just multiply
  10: ['add', 'sub', 'multiply', 'divide'], // CHANGED: was just divide
};
```

At band 9, the kid gets a mix of hard add/sub (within 100, with carrying/borrowing) plus multiplication (1-12). At band 10, they get everything including division. This matches real math education — a kid doing division should also maintain fluency at addition, subtraction, and multiplication.

The sub-skill weighting (60/40 strength/growth) ensures the mix is personalized — a kid strong at addition but weak at division gets more division, but still gets some addition to maintain fluency.

### Also: Spread should widen faster at the ceiling

When a kid is at band 10 (or whatever the max is) with high accuracy, the spread should widen more aggressively. There's no higher band to promote to, so the only way to add variety is a wider spread. Currently spread widens by 0.05 per PUZZLE_ATTEMPTED when accuracy > 75%. At the ceiling, make it 0.1.

```js
// In the reducer:
if (newBand === 10 && newBand === state.mathBand) {
  // At ceiling — widen faster to maintain variety
  if (rollingAcc > 0.75 && newSpreadWidth < 1.0) {
    newSpreadWidth = Math.min(1.0, newSpreadWidth + 0.1);
  }
}
```

### What This Means for The playtester

With these changes, at band 10 she'd get:
- Division (her strength — still ~25% of problems for maintenance)
- Multiplication 1-12 (band 9-10 level — she only had 1 attempt, needs practice)
- Addition within 100 with carrying (band 7 level via spread — she's had 0 carrying attempts)
- Subtraction with borrowing (band 7 level via spread — she's had 0 borrowing attempts)

The system would discover which of these she actually struggles with and adapt. Right now it can't discover anything because it only asks her division.

### Tests

```
- 'band 10 generates all four operations'
- 'band 9 generates add, sub, and multiply'
- 'spread widens faster at band 10 (0.1 instead of 0.05)'
```

## Bug 5: First 4 Events Missing Sub-Skill and Features

### Problem

The first 4 events in session 1 have `subSkill: null` and `features: null`. These were recorded before the taxonomy PR was deployed. Not a bug per se — backward compatibility works. But these events provide zero analytical value for sub-skill tracking or feature discovery.

### Fix

No code change needed. This is a data migration issue that self-heals — all new events have sub-skills and features. The old null events will eventually age out of the rolling window (max 20 entries). Document this as a known data quality issue in older session logs.

## Bug 6: Feature Extraction `maxDigit` Is Wrong for Multi-Digit Numbers

### Problem

Event at line 410: `"maxDigit": 14` for the problem 144 ÷ 12. The feature extractor does `Math.floor(a / 10)` which gives 14 for a=144. The `maxDigit` is supposed to capture the largest single digit in the problem, not the tens value of a 3-digit number.

### Fix

```js
// OLD: gets tens value, not digit
maxDigit: Math.max(onesA, onesB, a > 9 ? tensA : 0, b > 9 ? tensB : 0),

// NEW: extract all individual digits and take the max
maxDigit: Math.max(...String(a).split('').map(Number), ...String(b).split('').map(Number)),
```

### Tests

```
- 'maxDigit for 144 ÷ 12 is 4 (not 14)'
- 'maxDigit for 7 × 8 is 8'
- 'maxDigit for 23 + 14 is 4'
```

## Implementation Priority

1. **Rolling window persistence** — bug, immediate, data loss on every save/load
2. **Response time cap** — bug, immediate, corrupts all time-based analysis
3. **Band 9/10 multi-operation** — The playtester is stuck, she needs this now
4. **Scaffolding/pace adaptation** — dials are stale after intake
5. **Faster spread widening at ceiling** — quality of life at top band
6. **maxDigit fix** — feature data accuracy, low urgency

## Files Changed

```
adapter.js                              — rolling window persistence, response time cap
src/domain/learning/learner-profile.js  — scaffolding/pace adaptation in PUZZLE_ATTEMPTED,
                                          faster spread at ceiling
src/domain/learning/challenge-generator.js — BAND_OPERATIONS for bands 9-10, maxDigit fix

test/domain/learning/learner-profile.test.js  — scaffolding/pace tests, ceiling spread tests
test/domain/learning/challenge-generator.test.js — multi-op band tests, maxDigit tests
```
