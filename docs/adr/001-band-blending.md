# ADR-001: Band Blending — Distribution-Based Difficulty

**Status:** Accepted and implemented
**Date:** 2026-03-26
**Deciders:** Veesh, Claude (designer)

## Context

The adaptive engine originally used hard band levels — a kid at band 6 got only band 6 problems. Promotion required N correct answers in a row at the same band. This caused three problems observed via CLI simulator QA:

1. **Cliffs.** Band 5 (multiply x1,x2) to band 6 (+/- <50) is a sudden jump in difficulty. Accuracy cratered on promotion.
2. **Oscillation.** Kid promotes to 6, fails, demotes to 5, aces it, promotes to 6, fails. Endlessly. Simulator showed the gifted profile bouncing between bands 5-6 for 30 straight questions.
3. **Floor traps.** A struggling kid at band 1 with 65% accuracy needed 3 in a row to promote (0.65^3 = 27% chance). Effectively stuck forever.

Additionally, education research on spaced practice and interleaving showed that mixing problem types across difficulty levels builds better retention than drilling one level.

## Decision

Replace hard band levels with a weighted probability distribution around a center band. The band number becomes the center of a bell-curve-like spread, not a wall.

### Key design choices

**Band is a center, not a level.** `mathBand: 6` means "generate problems weighted toward band 6, with some band 5 reinforcement and some band 7 stretch." The kid sees a mix at all times.

**Spread width is a dial.** `spreadWidth: 0.5` controls how wide the distribution is. Tight (0.0) = 90% at center (almost the old behavior). Wide (1.0) = 30% at center with significant spread. The system tightens on frustration/demotion and widens on sustained good performance.

**Accuracy-based promotion replaces streaks.** Promote when: center accuracy >= 75% (4+ attempts) AND stretch accuracy >= 60% (2+ attempts). Demote when: center accuracy < 50% (4+ attempts). This evaluates sustained performance across a window, not a fragile streak that one careless error breaks.

**Streak is display-only.** Kids like seeing streaks, Sparky celebrates them, but streaks have zero mechanical effect on band progression. `streakToPromote` is removed from the profile.

**Stretch problems ramp through CRA and answer mode before promotion.** A kid at band 6 getting band 7 stretch problems starts with maximum support (concrete + multiple choice). As they succeed, the same band 7 problems shift to representational, then abstract. By promotion time, they've done band 7 at full difficulty. Promotion is invisible.

## Consequences

### Positive
- Cliffs eliminated — kid is already doing stretch problems before promotion
- Oscillation eliminated — promotion only fires after proven sustained accuracy at AND above center
- Floor traps eliminated — 65% accuracy kid can promote because the accuracy threshold (75% at center) is achievable without a perfect streak
- Natural interleaving — distribution mixes difficulty levels automatically
- Backward compatible — old saves without `spreadWidth` default to 0.5, old events without `centerBand` use `band` as both

### Negative
- More complex promotion logic — `shouldPromote()` and `shouldDemote()` replace a simple streak counter
- Distribution math adds a layer of indirection in challenge generation
- After a band change, `accuracyAtBand(newCenter)` returns almost no data because prior entries were at the old center — this is intentional (forces fresh evidence) but looks like a bug without the documentation comment

### Risks
- The intake assessor sets `promoteThreshold` and `stretchThreshold` based on response speed. With band blending, a low promote threshold (0.65) combined with wide spread could promote too aggressively. To be monitored via simulator.
- The spread width widen/tighten rates (0.05 up, 0.1-0.15 down) are tuned by intuition, not data. May need adjustment after real playtesting.

## Alternatives Considered

**Smoother band transitions (half-bands).** Adding bands 5.5, 6.5 etc. that mix operations between adjacent levels. Rejected: doubles the band count without solving the promotion mechanics problem.

**Longer rolling window.** Keeping hard bands but using a 50-entry window instead of 20 for more stable accuracy measurement. Rejected: doesn't solve the cliff problem — a bigger window just delays the oscillation.

**Keep streaks alongside accuracy-based promotion.** Promote on EITHER streak OR accuracy, whichever first. Initially specced this way, then rejected: band blending eliminates the problem streaks solve (cliffs), and keeping streaks adds complexity for zero benefit. Gifted kids promote via high accuracy just as fast.

## Implementation

Implemented in commit `84b0717`. 96 tests passing. Key files:

- `src/domain/learning/challenge-generator.js` — `bandDistribution()`, `sampleFromDistribution()`, challenge generation samples from distribution
- `src/domain/learning/learner-profile.js` — `shouldPromote()`, `shouldDemote()`, spread width adaptation, `streakToPromote` removed
- `src/domain/learning/rolling-window.js` — `accuracyAtBand()`, `accuracyAboveBand()`
- `adapter.js` — passes `centerBand` and `spreadWidth` through events and save data
