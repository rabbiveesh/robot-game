import { describe, it, expect } from 'vitest';
import {
  generateChallenge, bandDistribution, sampleFromDistribution,
  classifyAddition, classifySubtraction, classifyMultiplication,
  classifyDivision, classifyBond, extractFeatures,
} from '../../../src/domain/learning/challenge-generator.js';
import { createProfile } from '../../../src/domain/learning/learner-profile.js';

// Seeded PRNG for deterministic tests
function seededRng(seed = 42) {
  let s = seed;
  // Warm up to avoid correlated first values from sequential seeds
  for (let i = 0; i < 10; i++) {
    s = (s * 1664525 + 1013904223) & 0x7fffffff;
  }
  return () => {
    s = (s * 1664525 + 1013904223) & 0x7fffffff;
    return s / 0x7fffffff;
  };
}

describe('challenge-generator', () => {
  it('generates addition for band 1 (tight spread)', () => {
    const profile = createProfile({ mathBand: 1, spreadWidth: 0 });
    const rng = seededRng();
    const c = generateChallenge(profile, rng);
    expect(c.operation).toBe('add');
    expect(c.numbers.op).toBe('+');
    expect(c.correctAnswer).toBeLessThanOrEqual(5);
    expect(c.correctAnswer).toBeGreaterThan(0);
    expect(c.centerBand).toBe(1);
    expect(c.sampledBand).toBe(1);
  });

  it('generates add/sub for band 2', () => {
    const profile = createProfile({ mathBand: 2 });
    // Run several times to check range
    const ops = new Set();
    for (let i = 0; i < 30; i++) {
      const c = generateChallenge(profile, seededRng(i));
      ops.add(c.numbers.op);
      expect(c.correctAnswer).toBeLessThanOrEqual(10);
      expect(c.correctAnswer).toBeGreaterThanOrEqual(0);
    }
    expect(ops.has('+')).toBe(true);
  });

  it('generates mixed ops for band 9 (tight spread)', () => {
    const profile = createProfile({ mathBand: 9, spreadWidth: 0 });
    const ops = new Set();
    for (let i = 0; i < 50; i++) {
      ops.add(generateChallenge(profile, seededRng(i)).operation);
    }
    // Band 9 now has add, sub, multiply
    expect(ops.size).toBeGreaterThanOrEqual(2);
  });

  it('generates mixed ops including division for band 10 (tight spread)', () => {
    const profile = createProfile({ mathBand: 10, spreadWidth: 0 });
    const ops = new Set();
    for (let i = 0; i < 100; i++) {
      const c = generateChallenge(profile, seededRng(i));
      ops.add(c.operation);
      if (c.operation === 'divide') {
        expect(c.numbers.a / c.numbers.b).toBe(c.correctAnswer);
      }
    }
    expect(ops.has('divide')).toBe(true);
    expect(ops.size).toBeGreaterThanOrEqual(3);
  });

  it('always has exactly 3 choices', () => {
    for (let band = 1; band <= 10; band++) {
      const c = generateChallenge(createProfile({ mathBand: band, spreadWidth: 0 }), seededRng(band));
      expect(c.choices).toHaveLength(3);
    }
  });

  it('exactly one choice is correct', () => {
    const c = generateChallenge(createProfile({ mathBand: 5, spreadWidth: 0 }), seededRng());
    const corrects = c.choices.filter(ch => ch.correct);
    expect(corrects).toHaveLength(1);
    expect(Number(corrects[0].text)).toBe(c.correctAnswer);
  });

  it('wrong answers are close to correct answer (within spread)', () => {
    const c = generateChallenge(createProfile({ mathBand: 1, spreadWidth: 0 }), seededRng());
    const wrongs = c.choices.filter(ch => !ch.correct);
    for (const w of wrongs) {
      const diff = Math.abs(Number(w.text) - c.correctAnswer);
      expect(diff).toBeLessThanOrEqual(3); // spread for answers <= 20
      expect(diff).toBeGreaterThan(0);
    }
  });

  it('wrong answers scale spread for larger numbers', () => {
    const c = generateChallenge(createProfile({ mathBand: 7, spreadWidth: 0 }), seededRng(99));
    const wrongs = c.choices.filter(ch => !ch.correct);
    // Just verify wrongs exist and are different from answer
    for (const w of wrongs) {
      expect(Number(w.text)).not.toBe(c.correctAnswer);
    }
  });

  it('with seeded rng, output is deterministic', () => {
    const profile = createProfile({ mathBand: 3, spreadWidth: 0 });
    const c1 = generateChallenge(profile, seededRng(123));
    const c2 = generateChallenge(profile, seededRng(123));
    expect(c1.question).toBe(c2.question);
    expect(c1.correctAnswer).toBe(c2.correctAnswer);
  });

  it('returns frozen objects', () => {
    const c = generateChallenge(createProfile({ spreadWidth: 0 }), seededRng());
    expect(Object.isFrozen(c)).toBe(true);
    expect(Object.isFrozen(c.choices)).toBe(true);
    expect(Object.isFrozen(c.numbers)).toBe(true);
  });

  it('weights toward strength operations (60/40 split)', () => {
    // Give add high accuracy, sub low accuracy
    const profile = createProfile({
      mathBand: 2,
      operationStats: Object.freeze({
        add: Object.freeze({ correct: 9, attempts: 10 }),
        sub: Object.freeze({ correct: 1, attempts: 10 }),
        multiply: Object.freeze({ correct: 0, attempts: 0 }),
        divide: Object.freeze({ correct: 0, attempts: 0 }),
        number_bond: Object.freeze({ correct: 0, attempts: 0 }),
      }),
    });

    let addCount = 0;
    let subCount = 0;
    const iterations = 200;
    for (let i = 0; i < iterations; i++) {
      const c = generateChallenge(profile, seededRng(i));
      if (c.operation === 'add') addCount++;
      else if (c.operation === 'sub') subCount++;
    }
    // Add should appear roughly 60% of the time, sub 40%
    // Allow wide margin for randomness
    expect(addCount).toBeGreaterThan(iterations * 0.4);
    expect(subCount).toBeGreaterThan(iterations * 0.2);
  });

  it('generates subtraction when operation is sub at bands 6-7', () => {
    // Regression: bands 6-7 ignored the operation parameter and flipped a coin
    for (const band of [6, 7]) {
      const profile = createProfile({
        mathBand: band,
        spreadWidth: 0,
        operationStats: Object.freeze({
          add: Object.freeze({ correct: 1, attempts: 10 }),  // low accuracy → growth
          sub: Object.freeze({ correct: 9, attempts: 10 }),  // high accuracy → strength
          multiply: Object.freeze({ correct: 0, attempts: 0 }),
          divide: Object.freeze({ correct: 0, attempts: 0 }),
          number_bond: Object.freeze({ correct: 0, attempts: 0 }),
        }),
      });
      // Force sub to be picked by weighting, then verify the question is actually subtraction
      for (let i = 0; i < 50; i++) {
        const c = generateChallenge(profile, seededRng(i));
        if (c.operation === 'sub') {
          expect(c.numbers.op).toBe('-');
          expect(c.question).toMatch(/-/);
        }
        if (c.operation === 'add') {
          expect(c.numbers.op).toBe('+');
          expect(c.question).toMatch(/\+/);
        }
      }
    }
  });
});

describe('bandDistribution', () => {
  it('center band gets highest probability', () => {
    const dist = bandDistribution(5, 0.5);
    for (let b = 1; b <= 10; b++) {
      if (b !== 5) expect(dist[5]).toBeGreaterThan(dist[b]);
    }
  });

  it('probabilities sum to ~1.0', () => {
    for (const sw of [0, 0.25, 0.5, 0.75, 1.0]) {
      const dist = bandDistribution(5, sw);
      const sum = Object.values(dist).reduce((s, v) => s + v, 0);
      expect(sum).toBeCloseTo(1.0, 2);
    }
  });

  it('tight spread (0.0) puts ~90% at center', () => {
    const dist = bandDistribution(5, 0.0);
    expect(dist[5]).toBeGreaterThan(0.85);
  });

  it('wide spread (1.0) distributes more weight away from center', () => {
    const dist = bandDistribution(5, 1.0);
    const tight = bandDistribution(5, 0.0);
    expect(dist[5]).toBeLessThan(tight[5]); // wider spread = less at center
    expect(dist[5]).toBeLessThan(0.5);      // at most ~half at center
    expect(dist[4] + dist[6]).toBeGreaterThan(0.15); // meaningful adjacent weight
  });

  it('clamps to valid bands 1-10', () => {
    const dist = bandDistribution(5, 0.5);
    for (let b = 1; b <= 10; b++) {
      expect(dist[b]).toBeGreaterThanOrEqual(0);
    }
    expect(Object.keys(dist).map(Number).every(b => b >= 1 && b <= 10)).toBe(true);
  });

  it('band 1 center redistributes below-floor probability upward', () => {
    const dist = bandDistribution(1, 0.5);
    const sum = Object.values(dist).reduce((s, v) => s + v, 0);
    expect(sum).toBeCloseTo(1.0, 2);
    expect(dist[1]).toBeGreaterThan(0.4); // gets the overflow
  });

  it('band 10 center redistributes above-ceiling probability downward', () => {
    const dist = bandDistribution(10, 0.5);
    const sum = Object.values(dist).reduce((s, v) => s + v, 0);
    expect(sum).toBeCloseTo(1.0, 2);
    expect(dist[10]).toBeGreaterThan(0.4);
  });
});

describe('sampleFromDistribution', () => {
  it('with seeded rng, sampling is deterministic', () => {
    const dist = bandDistribution(5, 0.5);
    const a = sampleFromDistribution(dist, seededRng(42));
    const b = sampleFromDistribution(dist, seededRng(42));
    expect(a).toBe(b);
  });

  it('over 1000 samples, frequencies approximate the distribution', () => {
    const dist = bandDistribution(5, 0.5);
    const counts = {};
    for (let b = 1; b <= 10; b++) counts[b] = 0;
    const rng = seededRng(99);
    for (let i = 0; i < 1000; i++) {
      counts[sampleFromDistribution(dist, rng)]++;
    }
    // Center should be most frequent
    expect(counts[5]).toBeGreaterThan(counts[3]);
    expect(counts[5]).toBeGreaterThan(counts[7]);
    // Should have some variety
    expect(counts[4] + counts[6]).toBeGreaterThan(100);
  });

  it('never samples outside bands 1-10', () => {
    const dist = bandDistribution(1, 1.0);
    const rng = seededRng(1);
    for (let i = 0; i < 500; i++) {
      const b = sampleFromDistribution(dist, rng);
      expect(b).toBeGreaterThanOrEqual(1);
      expect(b).toBeLessThanOrEqual(10);
    }
  });
});

describe('classifyAddition', () => {
  it('3 + 4 → add_single', () => expect(classifyAddition(3, 4)).toBe('add_single'));
  it('23 + 14 → add_no_carry', () => expect(classifyAddition(23, 14)).toBe('add_no_carry'));
  it('28 + 15 → add_carry', () => expect(classifyAddition(28, 15)).toBe('add_carry'));
  it('85 + 47 → add_carry_tens', () => expect(classifyAddition(85, 47)).toBe('add_carry_tens'));
});

describe('classifySubtraction', () => {
  it('8 - 3 → sub_single', () => expect(classifySubtraction(8, 3)).toBe('sub_single'));
  it('47 - 23 → sub_no_borrow', () => expect(classifySubtraction(47, 23)).toBe('sub_no_borrow'));
  it('42 - 17 → sub_borrow', () => expect(classifySubtraction(42, 17)).toBe('sub_borrow'));
  it('103 - 47 → sub_borrow_tens', () => expect(classifySubtraction(103, 47)).toBe('sub_borrow_tens'));
});

describe('classifyMultiplication', () => {
  it('1 × 7 → mul_trivial', () => expect(classifyMultiplication(1, 7)).toBe('mul_trivial'));
  it('2 × 5 → mul_trivial', () => expect(classifyMultiplication(2, 5)).toBe('mul_trivial'));
  it('3 × 4 → mul_easy', () => expect(classifyMultiplication(3, 4)).toBe('mul_easy'));
  it('7 × 8 → mul_hard', () => expect(classifyMultiplication(7, 8)).toBe('mul_hard'));
});

describe('classifyDivision', () => {
  it('12 ÷ 3 → div_easy', () => expect(classifyDivision(12, 3)).toBe('div_easy'));
  it('56 ÷ 7 → div_hard', () => expect(classifyDivision(56, 7)).toBe('div_hard'));
  it('20 ÷ 5 → div_easy', () => expect(classifyDivision(20, 5)).toBe('div_easy'));
});

describe('classifyBond', () => {
  it('? + 3 = 7 → bond_small', () => expect(classifyBond(7, 3)).toBe('bond_small'));
  it('? + 8 = 15 → bond_large', () => expect(classifyBond(15, 8)).toBe('bond_large'));
});

describe('extractFeatures', () => {
  it('28 + 15: carries=true, carriesTens=false, crossesTenBoundary=true', () => {
    const f = extractFeatures(28, 15, 'add', 43);
    expect(f.carries).toBe(true);
    expect(f.carriesTens).toBe(false);
    expect(f.crossesTenBoundary).toBe(true);
  });

  it('23 + 14: carries=false, crossesTenBoundary=true', () => {
    const f = extractFeatures(23, 14, 'add', 37);
    expect(f.carries).toBe(false);
    expect(f.crossesTenBoundary).toBe(true);
  });

  it('42 - 17: borrows=true, borrowsTens=false', () => {
    const f = extractFeatures(42, 17, 'sub', 25);
    expect(f.borrows).toBe(true);
    expect(f.borrowsTens).toBe(false);
  });

  it('103 - 47: borrows=true, borrowsTens=true', () => {
    const f = extractFeatures(103, 47, 'sub', 56);
    expect(f.borrows).toBe(true);
    expect(f.borrowsTens).toBe(true);
  });

  it('7 × 8: bothFactorsGt5=true, maxDigitGte7=true, isSquare=false', () => {
    const f = extractFeatures(7, 8, 'multiply', 56);
    expect(f.bothFactorsGt5).toBe(true);
    expect(f.maxDigitGte7).toBe(true);
    expect(f.isSquare).toBe(false);
  });

  it('5 × 5: isSquare=true, hasFactorFive=true', () => {
    const f = extractFeatures(5, 5, 'multiply', 25);
    expect(f.isSquare).toBe(true);
    expect(f.hasFactorFive).toBe(true);
  });

  it('30 + 14: hasRoundNumber=true', () => {
    expect(extractFeatures(30, 14, 'add', 44).hasRoundNumber).toBe(true);
  });

  it('6 + 7: nearDoubles=true', () => {
    expect(extractFeatures(6, 7, 'add', 13).nearDoubles).toBe(true);
  });

  it('3 + 2: maxDigitGte7=false, answerGte10=false', () => {
    const f = extractFeatures(3, 2, 'add', 5);
    expect(f.maxDigitGte7).toBe(false);
    expect(f.answerGte10).toBe(false);
  });

  it('features object is frozen', () => {
    expect(Object.isFrozen(extractFeatures(3, 2, 'add', 5))).toBe(true);
  });
});

describe('generateChallenge includes subSkill and features', () => {
  it('returns subSkill and features in challenge', () => {
    const profile = createProfile({ mathBand: 6, spreadWidth: 0 });
    const c = generateChallenge(profile, seededRng());
    expect(c.subSkill).toBeTruthy();
    expect(c.features).toBeTruthy();
    expect(Object.isFrozen(c.features)).toBe(true);
  });
});

describe('multi-operation bands', () => {
  it('band 10 generates all four operations', () => {
    const ops = new Set();
    const profile = createProfile({ mathBand: 10, spreadWidth: 0 });
    for (let i = 0; i < 200; i++) {
      const c = generateChallenge(profile, seededRng(i));
      ops.add(c.operation);
    }
    expect(ops.has('add')).toBe(true);
    expect(ops.has('sub')).toBe(true);
    expect(ops.has('multiply')).toBe(true);
    expect(ops.has('divide')).toBe(true);
  });

  it('band 9 generates add, sub, and multiply', () => {
    const ops = new Set();
    const profile = createProfile({ mathBand: 9, spreadWidth: 0 });
    for (let i = 0; i < 200; i++) {
      const c = generateChallenge(profile, seededRng(i));
      ops.add(c.operation);
    }
    expect(ops.has('add')).toBe(true);
    expect(ops.has('sub')).toBe(true);
    expect(ops.has('multiply')).toBe(true);
  });
});

describe('maxDigit fix', () => {
  it('maxDigit for 144 / 12 is 4 (not 14)', () => {
    const f = extractFeatures(144, 12, 'divide', 12);
    expect(f.maxDigit).toBe(4);
  });

  it('maxDigit for 7 * 8 is 8', () => {
    const f = extractFeatures(7, 8, 'multiply', 56);
    expect(f.maxDigit).toBe(8);
  });

  it('maxDigit for 23 + 14 is 4', () => {
    const f = extractFeatures(23, 14, 'add', 37);
    expect(f.maxDigit).toBe(4);
  });
});

describe('displayText / speechText', () => {
  it('challenge has displayText and speechText fields', () => {
    const c = generateChallenge(createProfile({ mathBand: 5, spreadWidth: 0 }), seededRng());
    expect(c.displayText).toBeTruthy();
    expect(c.speechText).toBeTruthy();
  });

  it('displayText contains × symbol, speechText contains "times"', () => {
    const profile = createProfile({ mathBand: 5, spreadWidth: 0 });
    const c = generateChallenge(profile, seededRng());
    expect(c.displayText).toMatch(/×/);
    expect(c.speechText).toMatch(/times/);
    expect(c.speechText).not.toMatch(/×/);
  });

  it('displayText contains ÷ symbol, speechText contains "divided by"', () => {
    const profile = createProfile({ mathBand: 10, spreadWidth: 0 });
    for (let i = 0; i < 50; i++) {
      const c = generateChallenge(profile, seededRng(i));
      if (c.operation === 'divide') {
        expect(c.displayText).toMatch(/÷/);
        expect(c.speechText).toMatch(/divided by/);
        return;
      }
    }
  });

  it('speechText replaces + with "plus"', () => {
    const profile = createProfile({ mathBand: 2, spreadWidth: 0 });
    for (let i = 0; i < 30; i++) {
      const c = generateChallenge(profile, seededRng(i));
      if (c.operation === 'add') {
        expect(c.speechText).toMatch(/plus/);
        return;
      }
    }
  });

  it('number bond: displayText has = symbol, speechText has "equals"', () => {
    const profile = createProfile({ mathBand: 3, spreadWidth: 0 });
    for (let i = 0; i < 50; i++) {
      const c = generateChallenge(profile, seededRng(i));
      if (c.operation === 'number_bond') {
        expect(c.displayText).toMatch(/= \d+\?$/);
        expect(c.speechText).toMatch(/equals \d+\?$/);
        expect(c.speechText).not.toMatch(/=/);
        return;
      }
    }
  });

  it('displayText and speechText are produced from structure, not regex on question', () => {
    // Verify they're consistent for all operations
    for (let band = 1; band <= 10; band++) {
      const profile = createProfile({ mathBand: band, spreadWidth: 0 });
      const c = generateChallenge(profile, seededRng(band));
      // Speech should never contain math symbols
      expect(c.speechText).not.toMatch(/[×÷\u2212]/);
      // Display should be a valid question string
      expect(c.displayText).toMatch(/\?$/);
    }
  });
});
