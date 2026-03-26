# Per-Sub-Skill Band Tracking — Design Spec (FUTURE)

**Status:** Not ready for implementation. Depends on real playtesting data from the operation taxonomy to validate the need.

## Problem

The current system has one band center for all math. A kid at band 6 gets band 6 problems for addition, subtraction, carrying, borrowing — everything. But the simulator shows a gifted kid at 89% on `sub_no_borrow` and 20% on `sub_borrow` at the same band. The system can't express "this kid needs band 6 for non-borrow subtraction and band 3 for borrowing simultaneously."

The single band is too coarse. The kid gets dragged down by their weakest sub-skill, frustration cascades fire, and they end up at band 4 doing `add_single` problems they could do in their sleep — because `sub_borrow` cratered their overall accuracy.

## Observed in Simulator

Gifted profile (seed 42, post-taxonomy):
```
sub_no_borrow: 89% (8/9)    ← should be at band 6-7
sub_borrow:    20% (1/5)    ← should be at band 3-4
```

System result: kid ends at band 4 for everything. The 89% strength sub-skill is being punished by the 20% growth area sharing the same band.

## Proposed Solution

Replace the single `mathBand` with per-sub-skill band tracking. Each sub-skill has its own center band, spread width, and promotion/demotion thresholds.

```js
LearnerProfile {
  // REMOVE (or keep as a display-only "overall level")
  mathBand: 6,

  // ADD
  subSkillBands: {
    add_single:      { center: 4, spread: 0.3 },
    add_no_carry:    { center: 6, spread: 0.5 },
    add_carry:       { center: 4, spread: 0.3 },
    add_carry_tens:  { center: 3, spread: 0.2 },
    sub_single:      { center: 4, spread: 0.3 },
    sub_no_borrow:   { center: 7, spread: 0.5 },
    sub_borrow:      { center: 3, spread: 0.2 },
    sub_borrow_tens: { center: 2, spread: 0.1 },
    mul_trivial:     { center: 5, spread: 0.5 },
    mul_easy:        { center: 5, spread: 0.3 },
    mul_hard:        { center: 5, spread: 0.2 },
    div_easy:        { center: 5, spread: 0.2 },
    div_hard:        { center: 5, spread: 0.1 },
    bond_small:      { center: 4, spread: 0.3 },
    bond_large:      { center: 3, spread: 0.2 },
  },
}
```

### Challenge Generation

Currently: sample band from distribution → pick operation → pick sub-skill → generate numbers.

With per-sub-skill bands: pick operation → pick sub-skill (60/40 weighting) → sample band from THAT sub-skill's distribution → generate numbers.

The sub-skill's own band determines the number range. A kid might get `sub_no_borrow` at band 7 (79 - 45) followed by `sub_borrow` at band 3 (13 - 7). Both are subtraction. The numbers are appropriate for what the kid can actually do with each sub-skill.

### Promotion and Demotion

Each sub-skill promotes/demotes independently based on its own accuracy window. `sub_no_borrow` promoting to band 7 doesn't affect `sub_borrow` at band 3.

The rolling window would need to be filterable by sub-skill, or each sub-skill maintains its own mini-window. The `accuracyAtBand` and `accuracyAboveBand` functions would take a sub-skill filter parameter.

### Display Band

For the UI (Sparky celebrating, area unlock, parent dashboard), compute a display band from the sub-skill bands — weighted average, or median, or the band of the most-practiced sub-skill. This replaces `mathBand` for display purposes but has no mechanical effect.

### Interaction with Band Blending

Band blending still works — each sub-skill has its own center and spread. The distribution is per-sub-skill. This is actually simpler than the current system because the band and the sub-skill are aligned — no more "band 6 subtraction that might or might not involve borrowing."

## Complexity Concern

This is significantly more state. 15 sub-skills × (center + spread) = 30 values instead of 2 (mathBand + spreadWidth). The promotion logic runs per-sub-skill. The rolling window needs sub-skill filtering.

Is it worth it? Depends on what we see in real playtesting. If real kids show the same pattern as the simulator (strong at one sub-skill, weak at another within the same band), yes. If most real kids are roughly uniform within a band, the single-band system with sub-skill weighting (what we have now) is sufficient.

## Decision Criteria

Implement per-sub-skill bands when:
1. Real playtesting shows a kid stuck because one sub-skill is dragging down their overall band (like the simulator's gifted profile)
2. The parent dashboard shows sub-skill accuracy gaps > 30% within the same band, for multiple kids
3. The feature discovery system (from the taxonomy spec) confirms that sub-skill is the dominant predictor of errors, not just a feature among many

## What We Have Now That Enables This Later

- Sub-skill classification on every challenge (ships with taxonomy PR)
- Feature vectors in every event (ships with taxonomy PR)
- Per-sub-skill operation stats (ships with taxonomy PR)
- Rolling window entries include `subSkill` field
- `accuracyAtBand` and `accuracyAboveBand` can be extended with a sub-skill filter

The data foundation is in place. The migration would be: replace `mathBand` + `spreadWidth` with `subSkillBands`, update the challenge generator to read per-sub-skill bands, update the reducer to promote/demote per-sub-skill. The event schema doesn't change.
