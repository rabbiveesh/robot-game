import { describe, it, expect } from 'vitest';
import { generateChallenge, bandDistribution, sampleFromDistribution } from '../../../src/domain/learning/challenge-generator.js';
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

  it('generates multiplication for band 9 (tight spread)', () => {
    const profile = createProfile({ mathBand: 9, spreadWidth: 0 });
    const c = generateChallenge(profile, seededRng());
    expect(c.operation).toBe('multiply');
    expect(c.numbers.op).toBe('\u00d7');
  });

  it('generates division for band 10 (tight spread)', () => {
    const profile = createProfile({ mathBand: 10, spreadWidth: 0 });
    const c = generateChallenge(profile, seededRng());
    expect(c.operation).toBe('divide');
    expect(c.numbers.op).toBe('\u00f7');
    expect(c.numbers.a / c.numbers.b).toBe(c.correctAnswer);
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
