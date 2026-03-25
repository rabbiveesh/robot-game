# Operation Taxonomy — Implementation Spec

## Problem

The current operation tracking treats `add` and `sub` as monoliths. But 23+14 (no carry) and 28+15 (carry) are different cognitive skills. A kid strong at one may completely fail at the other. The simulator showed the gifted profile cratering at band 6 because the band mixes trivial non-carry problems with hard carry problems — accuracy reads as ~50% when it's actually "90% non-carry, 20% carry." The adaptive system can't distinguish these because it only sees `operation: 'add'`.

Same problem for subtraction with borrowing, and multiplication has its own sub-skills (times-1, times-2 are trivially different from times-7, times-8).

## The Taxonomy

Operations split into sub-skills that map to real cognitive milestones:

### Addition
| Sub-skill | Example | When it appears | What's hard about it |
|-----------|---------|----------------|---------------------|
| `add_single` | 3 + 4 | Bands 1-2 | Basic number facts |
| `add_no_carry` | 23 + 14 | Bands 3-6 | Place value, but no regrouping |
| `add_carry` | 28 + 15 | Bands 4-7 | Ones sum > 9, must regroup to tens. Completely different mental operation. |
| `add_carry_tens` | 85 + 47 | Bands 7+ | Carry propagates across tens column too |

### Subtraction
| Sub-skill | Example | When it appears | What's hard about it |
|-----------|---------|----------------|---------------------|
| `sub_single` | 8 - 3 | Bands 1-2 | Basic number facts |
| `sub_no_borrow` | 47 - 23 | Bands 3-6 | Place value, no regrouping |
| `sub_borrow` | 42 - 17 | Bands 4-7 | Ones digit insufficient, must borrow from tens. The #1 stumbling block in elementary math. |
| `sub_borrow_tens` | 103 - 47 | Bands 7+ | Borrow propagates |

### Multiplication
| Sub-skill | Example | When it appears | What's hard about it |
|-----------|---------|----------------|---------------------|
| `mul_trivial` | 1 × 7, 2 × 5 | Band 5 | Times-1 is identity, times-2 is doubling. Most kids get these. |
| `mul_easy` | 3 × 4, 5 × 6 | Band 8 | Small tables. 5s have a pattern. |
| `mul_hard` | 7 × 8, 6 × 9 | Band 9 | The notorious ones. No easy pattern. Kids memorize or derive. |

### Division
| Sub-skill | Example | When it appears | What's hard about it |
|-----------|---------|----------------|---------------------|
| `div_easy` | 12 ÷ 3, 20 ÷ 5 | Band 10 | Inverse of easy multiplication |
| `div_hard` | 56 ÷ 7, 72 ÷ 9 | Band 10 | Inverse of hard multiplication |

### Number bonds
| Sub-skill | Example | When it appears | What's hard about it |
|-----------|---------|----------------|---------------------|
| `bond_small` | ? + 3 = 7 | Band 3 | Missing addend, small numbers |
| `bond_large` | ? + 8 = 15 | Band 4 | Missing addend, may require carrying logic |

## How It Works

### Challenge Generator

The `generateNumbers` function already picks numbers for each band. The change: tag each generated problem with its sub-skill based on the numbers chosen.

```js
function classifyAddition(a, b) {
  if (a < 10 && b < 10) return 'add_single';
  const onesSum = (a % 10) + (b % 10);
  if (onesSum < 10) return 'add_no_carry';
  // Check if tens carry propagates
  const tensSum = Math.floor(a / 10) + Math.floor(b / 10) + (onesSum >= 10 ? 1 : 0);
  if (tensSum >= 10) return 'add_carry_tens';
  return 'add_carry';
}

function classifySubtraction(a, b) {
  if (a < 10 && b < 10) return 'sub_single';
  const onesA = a % 10;
  const onesB = b % 10;
  if (onesA >= onesB) return 'sub_no_borrow';
  const tensA = Math.floor(a / 10);
  const tensB = Math.floor(b / 10);
  if (tensA - 1 < tensB) return 'sub_borrow_tens';
  return 'sub_borrow';
}

function classifyMultiplication(a, b) {
  const smaller = Math.min(a, b);
  const larger = Math.max(a, b);
  if (smaller <= 2) return 'mul_trivial';
  if (smaller <= 5 && larger <= 6) return 'mul_easy';
  return 'mul_hard';
}

function classifyDivision(dividend, divisor) {
  // Classify by the multiplication it inverts
  const answer = dividend / divisor;
  return classifyMultiplication(divisor, answer).replace('mul_', 'div_').replace('trivial', 'easy');
}
```

The sub-skill is returned alongside the challenge and included in the event:

```js
{
  type: 'PUZZLE_ATTEMPTED',
  correct: true,
  operation: 'add',           // coarse — still used for CRA tracking, backward compat
  subSkill: 'add_carry',      // fine — used for operation stats, challenge weighting
  band: 6,
  centerBand: 6,
  ...
}
```

### Operation Stats

Expand from 5 coarse operations to the fine-grained sub-skills:

```js
function createOperationStats() {
  return Object.freeze({
    // Coarse (kept for backward compat and CRA tracking)
    add: Object.freeze({ correct: 0, attempts: 0 }),
    sub: Object.freeze({ correct: 0, attempts: 0 }),
    multiply: Object.freeze({ correct: 0, attempts: 0 }),
    divide: Object.freeze({ correct: 0, attempts: 0 }),
    number_bond: Object.freeze({ correct: 0, attempts: 0 }),

    // Fine-grained (new — drives challenge weighting)
    add_single: Object.freeze({ correct: 0, attempts: 0 }),
    add_no_carry: Object.freeze({ correct: 0, attempts: 0 }),
    add_carry: Object.freeze({ correct: 0, attempts: 0 }),
    add_carry_tens: Object.freeze({ correct: 0, attempts: 0 }),
    sub_single: Object.freeze({ correct: 0, attempts: 0 }),
    sub_no_borrow: Object.freeze({ correct: 0, attempts: 0 }),
    sub_borrow: Object.freeze({ correct: 0, attempts: 0 }),
    sub_borrow_tens: Object.freeze({ correct: 0, attempts: 0 }),
    mul_trivial: Object.freeze({ correct: 0, attempts: 0 }),
    mul_easy: Object.freeze({ correct: 0, attempts: 0 }),
    mul_hard: Object.freeze({ correct: 0, attempts: 0 }),
    div_easy: Object.freeze({ correct: 0, attempts: 0 }),
    div_hard: Object.freeze({ correct: 0, attempts: 0 }),
    bond_small: Object.freeze({ correct: 0, attempts: 0 }),
    bond_large: Object.freeze({ correct: 0, attempts: 0 }),
  });
}

function recordOperation(stats, operation, subSkill, correct) {
  // Record BOTH coarse and fine-grained
  let updated = stats;
  if (stats[operation]) {
    updated = { ...updated, [operation]: freeze({ correct: stats[operation].correct + (correct ? 1 : 0), attempts: stats[operation].attempts + 1 }) };
  }
  if (subSkill && stats[subSkill]) {
    updated = { ...updated, [subSkill]: freeze({ correct: stats[subSkill].correct + (correct ? 1 : 0), attempts: stats[subSkill].attempts + 1 }) };
  }
  return Object.freeze(updated);
}
```

### Challenge Weighting

The 60/40 strength/growth weighting in `pickOperation` currently operates on coarse operations (`add`, `sub`, `multiply`). With sub-skills, we add a second weighting pass:

```
Step 1: Pick coarse operation (existing 60/40 weighting)
        → e.g., 'add'

Step 2: Pick sub-skill within that operation (NEW 60/40 weighting)
        → e.g., if add_no_carry is strength and add_carry is growth,
          60% chance of add_no_carry, 40% chance of add_carry

Step 3: Generate numbers that produce the chosen sub-skill
        → if sub-skill is add_carry, ensure (a % 10) + (b % 10) >= 10
```

Step 3 is the key change to `generateNumbers` — instead of random numbers within a range, we constrain the numbers to produce the target sub-skill.

```js
function generateAddition(band, targetSubSkill, rng) {
  // Band determines the number range
  const maxSum = [0, 5, 10, 15, 20, 20, 50, 100, 100, 100, 100][band];

  if (targetSubSkill === 'add_carry') {
    // Ensure ones digits sum to >= 10
    let a, b;
    do {
      a = Math.floor(rng() * (maxSum / 2)) + 5;
      b = Math.floor(rng() * (maxSum - a - 1)) + 1;
    } while ((a % 10) + (b % 10) < 10 || a + b > maxSum);
    return { a, b, answer: a + b };
  }

  if (targetSubSkill === 'add_no_carry') {
    // Ensure ones digits sum to < 10
    let a, b;
    do {
      a = Math.floor(rng() * (maxSum / 2)) + 2;
      b = Math.floor(rng() * (maxSum - a - 1)) + 1;
    } while ((a % 10) + (b % 10) >= 10 || a + b > maxSum);
    return { a, b, answer: a + b };
  }

  // Default: any addition within range
  // ...
}
```

### Which Sub-Skills Appear at Which Bands

| Band | Available sub-skills |
|------|---------------------|
| 1 | `add_single` |
| 2 | `add_single`, `sub_single` |
| 3 | `add_single`, `add_no_carry`, `sub_single`, `sub_no_borrow`, `bond_small` |
| 4 | `add_no_carry`, `add_carry`, `sub_no_borrow`, `sub_borrow`, `bond_small`, `bond_large` |
| 5 | `mul_trivial` |
| 6 | `add_no_carry`, `add_carry`, `sub_no_borrow`, `sub_borrow` |
| 7 | `add_carry`, `add_carry_tens`, `sub_borrow`, `sub_borrow_tens` |
| 8 | `mul_trivial`, `mul_easy` |
| 9 | `mul_easy`, `mul_hard` |
| 10 | `div_easy`, `div_hard` |

Note: band blending means a kid at band 6 might get band 4 problems (which include `add_carry`) or band 7 problems (which include `add_carry_tens`). The sub-skill weighting operates AFTER band sampling — first sample the band, then pick the sub-skill within that band.

### Interaction with Band Blending

Band blending and sub-skills are orthogonal and compose cleanly:

```
1. Sample band from distribution (band blending)     → e.g., band 6
2. Pick coarse operation (60/40 strength/growth)      → e.g., 'sub'
3. Pick sub-skill within operation (60/40 weighting)  → e.g., 'sub_borrow'
4. Generate numbers that produce the sub-skill        → e.g., 42 - 17
```

Each step narrows the space. The adaptive system has 60/40 weighting at both the operation and sub-skill levels, so a kid weak at borrowing gets more borrowing practice (but not TOO much — still 60% strength).

## Parent Dashboard Implications

The debug overlay (and future parent dashboard) can show sub-skill breakdowns:

```
Addition:
  Single digit:  95% (19/20)    ████████████████████
  No carry:      82% (9/11)     ████████████████░░░░
  With carry:    33% (2/6)      ██████░░░░░░░░░░░░░░  ← growth area
  Carry tens:    --

Subtraction:
  Single digit:  90% (9/10)     ██████████████████░░
  No borrow:     78% (7/9)      ████████████████░░░░
  With borrow:   20% (1/5)      ████░░░░░░░░░░░░░░░░  ← growth area
  Borrow tens:   --
```

This tells a parent exactly what to work on — not "my kid is bad at subtraction" but "my kid is fine at subtraction except when borrowing is involved."

## Event Schema Change

```js
PuzzleAttempted {
  type: 'PUZZLE_ATTEMPTED',
  correct: boolean,
  operation: string,      // coarse: 'add', 'sub', etc. (backward compat)
  subSkill: string,       // fine: 'add_carry', 'sub_borrow', etc. (new)
  band: number,           // sampled band
  centerBand: number,
  responseTimeMs: number,
  ...
}
```

Backward compatible — old events without `subSkill` still work. The reducer records both coarse and fine stats. The coarse stats are used for CRA tracking (per the interaction model spec). The fine stats are used for sub-skill weighting.

## Tests

```
classifyAddition:
  - '3 + 4 → add_single'
  - '23 + 14 → add_no_carry'
  - '28 + 15 → add_carry'
  - '85 + 47 → add_carry_tens'

classifySubtraction:
  - '8 - 3 → sub_single'
  - '47 - 23 → sub_no_borrow'
  - '42 - 17 → sub_borrow'
  - '103 - 47 → sub_borrow_tens'

classifyMultiplication:
  - '1 × 7 → mul_trivial'
  - '2 × 5 → mul_trivial'
  - '3 × 4 → mul_easy'
  - '7 × 8 → mul_hard'

generateNumbers with targeted sub-skill:
  - 'add_carry always produces ones sum >= 10'
  - 'add_no_carry always produces ones sum < 10'
  - 'sub_borrow always produces ones_a < ones_b'
  - 'sub_no_borrow always produces ones_a >= ones_b'

operationStats records both coarse and fine:
  - 'add_carry attempt records to both add and add_carry'
  - 'unknown sub-skill is silently ignored'

sub-skill weighting:
  - 'weak sub-skill gets ~40% of attempts within its operation'
  - 'strong sub-skill gets ~60% of attempts within its operation'
```

## Files Changed

```
src/domain/learning/
  challenge-generator.js   — classify functions, sub-skill targeting in generateNumbers,
                             two-pass weighting (operation then sub-skill)
  operation-stats.js       — expanded with fine-grained sub-skills, dual recording
  learner-profile.js       — reducer records subSkill in window entries and stats

test/domain/learning/
  challenge-generator.test.js — classification tests, targeted generation tests
  operation-stats.test.js     — (new file or expanded) dual recording tests
  learner-profile.test.js     — sub-skill in events tests

adapter.js                 — pass subSkill through events, classify legacy challenges
```

## Migration

Backward compatible. Old saves without fine-grained stats get zeros for all sub-skills (they'll populate on first play). Old events without `subSkill` field are treated as unclassified — the coarse `operation` field is still recorded as before.
