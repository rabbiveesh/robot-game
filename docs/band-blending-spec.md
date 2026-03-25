# Band Blending — Implementation Spec

## Problem

The current system treats bands as hard levels. A kid is AT band 6 and every problem is band 6. This creates:

1. **Cliffs**: band 5 (multiply x1,x2 — easy) to band 6 (+/- <50 — hard two-digit) is a sudden jump. Accuracy craters on promotion.
2. **Oscillation**: kid promotes to 6, fails, demotes to 5, aces it, promotes to 6, fails. Endlessly.
3. **Floor traps**: kid at band 1 with 65% accuracy can't get 3 in a row (27% probability), stays stuck forever.
4. **No interleaving**: research says mixing problem types builds retention. Drilling one band type doesn't.

## Solution

The band becomes a **center point** of a weighted distribution, not a hard level. Problem generation samples from a spread around the center.

```
Kid at band 6:

  band 4:  ░░                          5%   confidence builders
  band 5:  ░░░░░░░                    20%   reinforcement
  band 6:  ░░░░░░░░░░░░░░░░░          50%   working level
  band 7:  ░░░░░░░                    20%   stretch
  band 8:  ░░                          5%   exposure
```

## How It Works

### Distribution Generation

Given a center band and a spread width, produce a probability map:

```js
function bandDistribution(centerBand, spreadWidth) {
  // spreadWidth: 0.0 = tight (90% at center), 1.0 = wide (50% at center)
  // Returns: { [band]: probability } for bands 1-10
  //
  // At spreadWidth 0.0 (tight):
  //   center:  90%
  //   center±1: 5% each
  //
  // At spreadWidth 0.5 (normal):
  //   center:  50%
  //   center±1: 20% each
  //   center±2: 5% each
  //
  // At spreadWidth 1.0 (wide):
  //   center:  30%
  //   center±1: 20% each
  //   center±2: 10% each
  //   center±3: 5% each
  //
  // Clamp to valid bands 1-10. Redistribute overflow to nearest valid band.
}
```

### Challenge Generation Change

The challenge generator currently does:
```js
// OLD: always generate at the exact band
const { a, b, answer, question, op } = generateNumbers(profile.mathBand, operation, rng);
```

It becomes:
```js
// NEW: sample a band from the distribution, then generate at that band
const dist = bandDistribution(profile.mathBand, profile.spreadWidth);
const sampledBand = sampleFromDistribution(dist, rng);
const { a, b, answer, question, op } = generateNumbers(sampledBand, operation, rng);
```

The returned challenge includes both the center band and the sampled band:
```js
return Object.freeze({
  question,
  correctAnswer: answer,
  choices: ...,
  operation,
  centerBand: profile.mathBand,   // the kid's level
  sampledBand,                     // the actual difficulty of this problem
  numbers: Object.freeze({ a, b, op }),
});
```

### Event Change

`PUZZLE_ATTEMPTED` gets a new field:

```js
{
  type: 'PUZZLE_ATTEMPTED',
  correct: true,
  operation: 'add',
  band: 7,            // CHANGED: this is now the sampledBand (actual difficulty)
  centerBand: 6,      // NEW: the kid's current center band
  responseTimeMs: 2100,
  ...
}
```

The reducer uses both:
- `band` (sampled) tells us what difficulty they actually faced
- `centerBand` tells us their current level for streak/promotion logic

### Promotion and Demotion Changes

Promotion no longer means "N correct in a row at your band." It means **the distribution is generating above-center problems and the kid is handling them.**

```js
// NEW promotion logic in the reducer:

// Track accuracy at each difficulty tier relative to center
const atCenter = window.entries.filter(e => e.band === state.mathBand);
const aboveCenter = window.entries.filter(e => e.band > state.mathBand);
const belowCenter = window.entries.filter(e => e.band < state.mathBand);

// Promote when:
//   1. Accuracy at center band >= 75% (at least 4 attempts)
//   2. Accuracy at above-center >= 60% (at least 2 attempts)
// This means the kid has proven they can handle harder problems
// that the distribution has been feeding them.

// Demote when:
//   1. Accuracy at center band < 50% (at least 4 attempts)
//   2. OR frustration detected
// The kid is struggling at their own level.
```

This kills the streak requirement entirely for promotion. "3 in a row" becomes "sustained performance across a window" which is more robust and doesn't penalize a single careless error.

The old streak is still tracked for the UI (kids like seeing streaks) but it no longer drives promotion.

### Spread Width Dial

New field on the learner profile:

```js
spreadWidth: 0.5,  // 0 = tight (almost all at center), 1 = wide (lots of variety)
```

**Adaptation:**
- High accuracy + low frustration → widen spread (more variety, more stretch)
- Low accuracy or frustration → tighten spread (stay close to what you know)
- After promotion → temporarily tighten spread (let the kid adjust to the new center)
- After N problems at tight spread with good accuracy → widen back out

```js
// In the reducer, after PUZZLE_ATTEMPTED:

// Tighten on demotion or frustration
if (newBand < state.mathBand || event.type === 'FRUSTRATION_DETECTED') {
  newSpreadWidth = Math.max(0.1, state.spreadWidth - 0.15);
}

// Tighten briefly after promotion (let kid adjust)
if (newBand > state.mathBand) {
  newSpreadWidth = Math.max(0.2, state.spreadWidth - 0.1);
}

// Widen on sustained good performance (accuracy > 75% over last 10)
if (rollingAccuracy > 0.75 && state.spreadWidth < 0.8) {
  newSpreadWidth = Math.min(1.0, state.spreadWidth + 0.05);
}
```

### Interleaving

The spread naturally produces interleaving — a band 6 kid gets a mix of operations across bands 4-8. But we can enhance this by ensuring the generator doesn't produce the same band twice in a row:

```js
// If last problem was at band 7, prefer sampling a different band this time
// (nudge the distribution away from the last sampled band)
```

This gives the "spaced practice" effect from the education research.

## Profile Changes

```js
LearnerProfile {
  // EXISTING
  mathBand: 6,              // center of distribution (was: hard level)
  streak: 2,                // display only — Sparky celebrates streaks, no mechanical effect

  // NEW
  spreadWidth: 0.5,         // distribution width (0 = tight, 1 = wide)
  promoteThreshold: 0.75,   // accuracy at center band needed to promote
  stretchThreshold: 0.60,   // accuracy at above-center needed to promote

  // REMOVED
  // streakToPromote — replaced by accuracy-based promotion
}
```

Streak-based promotion is removed entirely. Band blending eliminates the problem streaks were solving — there's no cliff to overcome because the kid is already seeing stretch problems before promotion. Promotion is just "shift the center up" after the kid has proven sustained accuracy at and above their current center.

`streakToPromote` is removed from the profile. `streak` is kept as a display-only counter (kids like seeing streaks, Sparky can celebrate them) but it has zero effect on band progression.

The intake assessor no longer sets `streakToPromote`. It sets `promoteThreshold` and `stretchThreshold` instead — a confident fast kid gets lower thresholds (promotes sooner), a cautious kid gets higher ones (more evidence needed).

Before promotion, the system also varies stretch problems along CRA and answer mode axes. A kid at band 6 getting band 7 stretch problems starts with maximum support (concrete + multiple choice). As they succeed at band 7, the same problems shift to representational, then abstract, then free input. By the time they promote to band 7 as center, they've done band 7 at full difficulty. Promotion is a formality — the kid barely notices because nothing actually changed.

## Tests

### bandDistribution

```
- 'center band gets highest probability'
- 'probabilities sum to 1.0'
- 'tight spread (0.0) puts 90% at center'
- 'wide spread (1.0) puts 30% at center'
- 'clamps to valid bands 1-10'
- 'band 1 center redistributes below-floor probability upward'
- 'band 10 center redistributes above-ceiling probability downward'
```

### sampleFromDistribution

```
- 'with seeded rng, sampling is deterministic'
- 'over 1000 samples, frequencies approximate the distribution'
- 'never samples outside bands 1-10'
```

### Promotion (accuracy-based)

```
- 'promotes when center accuracy >= 75% and stretch accuracy >= 60%'
- 'does not promote when center accuracy is below threshold'
- 'does not promote with insufficient attempts (need at least 4 at center)'
- 'promotes even without a streak if accuracy is sustained'
- 'still promotes via streak for consistently perfect kids'
- 'demotes when center accuracy < 50%'
- 'does not demote with insufficient attempts'
```

### Spread width adaptation

```
- 'tightens on demotion'
- 'tightens on frustration'
- 'tightens briefly after promotion'
- 'widens on sustained good accuracy'
- 'does not widen above 1.0 or tighten below 0.1'
```

### Integration with simulator

After implementation, re-run all 4 profiles and verify:
- Gifted: no more oscillation at band 5/6 boundary. Smooth ascent.
- Struggling: doesn't get stuck at band 1 forever. Can promote via accuracy.
- 7yo: smoother band 7/8 transition. Less whiplash.
- 2e: same good behavior (high band, slow pace) but fewer frustration events.

## Files Changed

```
src/domain/learning/
  challenge-generator.js   — add bandDistribution(), sampleFromDistribution(),
                             use sampled band instead of exact band
  learner-profile.js       — add spreadWidth to profile, accuracy-based promotion
                             logic alongside streak-based, spread adaptation
  rolling-window.js        — add accuracyAtBand(window, band) and
                             accuracyAboveBand(window, band) helpers

test/domain/learning/
  challenge-generator.test.js  — distribution tests, sampling tests
  learner-profile.test.js      — accuracy-based promotion tests, spread tests
  rolling-window.test.js       — new helper tests

tools/simulate.js          — update to show sampled band vs center band in output
adapter.js                 — pass spreadWidth through, include centerBand in events
```

## Migration

The existing `mathBand` field keeps its meaning (center of distribution). Save data is backward compatible — old saves without `spreadWidth` default to 0.5. Events without `centerBand` use `band` as both (same as before blending, which is correct for historical events).
