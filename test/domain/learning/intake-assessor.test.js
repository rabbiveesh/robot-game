import { describe, it, expect } from 'vitest';
import {
  generateIntakeQuestion, processIntakeResults, nextIntakeBand,
} from '../../../src/domain/learning/intake-assessor.js';

function seededRng(seed = 42) {
  let s = seed;
  for (let i = 0; i < 10; i++) {
    s = (s * 1664525 + 1013904223) & 0x7fffffff;
  }
  return () => {
    s = (s * 1664525 + 1013904223) & 0x7fffffff;
    return s / 0x7fffffff;
  };
}

describe('generateIntakeQuestion', () => {
  it('generates a challenge at the given band', () => {
    const q = generateIntakeQuestion(3, 0, seededRng());
    expect(q.band).toBe(3);
    expect(q.question).toBeTruthy();
    expect(q.choices).toHaveLength(3);
  });
});

describe('nextIntakeBand', () => {
  it('increases band by 2 on correct', () => {
    expect(nextIntakeBand(3, true)).toBe(5);
  });

  it('decreases band by 1 on wrong', () => {
    expect(nextIntakeBand(3, false)).toBe(2);
  });

  it('does not exceed band 10', () => {
    expect(nextIntakeBand(9, true)).toBe(10);
    expect(nextIntakeBand(10, true)).toBe(10);
  });

  it('respects ceiling parameter', () => {
    expect(nextIntakeBand(3, true, 4)).toBe(4); // would be 5, clamped to 4
    expect(nextIntakeBand(2, true, 3)).toBe(3); // would be 4, clamped to 3
  });

  it('does not go below band 1', () => {
    expect(nextIntakeBand(1, false)).toBe(1);
  });
});

describe('processIntakeResults', () => {
  it('places kid in correct band after all-correct intake', () => {
    // Start band 3, correct -> band 5, correct -> band 7, correct -> band 9, correct -> band 10
    const answers = [
      { band: 3, correct: true, responseTimeMs: 2000 },
      { band: 5, correct: true, responseTimeMs: 2500 },
      { band: 7, correct: true, responseTimeMs: 3000 },
      { band: 9, correct: true, responseTimeMs: 2000 },
    ];
    const result = processIntakeResults(answers);
    expect(result.mathBand).toBe(9); // last correct band
  });

  it('places kid in band 1 after all-wrong intake', () => {
    const answers = [
      { band: 3, correct: false, responseTimeMs: 5000 },
      { band: 2, correct: false, responseTimeMs: 6000 },
      { band: 1, correct: false, responseTimeMs: 7000 },
      { band: 1, correct: false, responseTimeMs: 8000 },
    ];
    const result = processIntakeResults(answers);
    expect(result.mathBand).toBe(1);
  });

  it('fast responder gets higher pace dial', () => {
    const answers = [
      { band: 3, correct: true, responseTimeMs: 1500 },
      { band: 5, correct: true, responseTimeMs: 2000 },
      { band: 7, correct: true, responseTimeMs: 1800 },
      { band: 9, correct: false, responseTimeMs: 2200 },
    ];
    const result = processIntakeResults(answers);
    expect(result.pace).toBeGreaterThanOrEqual(0.7);
  });

  it('slow responder gets lower pace and more scaffolding', () => {
    const answers = [
      { band: 3, correct: true, responseTimeMs: 9000 },
      { band: 5, correct: false, responseTimeMs: 10000 },
      { band: 4, correct: true, responseTimeMs: 8500 },
      { band: 6, correct: false, responseTimeMs: 11000 },
    ];
    const result = processIntakeResults(answers);
    expect(result.pace).toBeLessThanOrEqual(0.3);
    expect(result.scaffolding).toBeGreaterThanOrEqual(0.7);
  });

  it('text skipper gets faster textSpeed', () => {
    const answers = [
      { band: 3, correct: true, responseTimeMs: 3000, skippedText: true },
      { band: 5, correct: true, responseTimeMs: 3000, skippedText: true },
      { band: 7, correct: false, responseTimeMs: 3000 },
      { band: 6, correct: true, responseTimeMs: 3000 },
    ];
    const result = processIntakeResults(answers);
    expect(result.textSpeed).toBeLessThan(0.035);
  });

  it('confident fast kids get lower promote thresholds', () => {
    const answers = [
      { band: 3, correct: true, responseTimeMs: 1500 },
      { band: 5, correct: true, responseTimeMs: 2000 },
      { band: 7, correct: true, responseTimeMs: 1800 },
      { band: 9, correct: true, responseTimeMs: 2200 },
    ];
    const result = processIntakeResults(answers);
    expect(result.promoteThreshold).toBeLessThan(0.75);
    expect(result.stretchThreshold).toBeLessThan(0.60);
  });

  it('clamps placement to configuredBand + 2 when parent set a low band', () => {
    // Kid aces intake up to band 9, but parent configured band 1 (Add <5)
    const answers = [
      { band: 3, correct: true, responseTimeMs: 2000 },
      { band: 5, correct: true, responseTimeMs: 2500 },
      { band: 7, correct: true, responseTimeMs: 3000 },
      { band: 9, correct: true, responseTimeMs: 2000 },
    ];
    const result = processIntakeResults(answers, 1);
    expect(result.mathBand).toBe(3); // clamped to configuredBand(1) + 2
  });

  it('does not clamp when configuredBand is null', () => {
    const answers = [
      { band: 3, correct: true, responseTimeMs: 2000 },
      { band: 5, correct: true, responseTimeMs: 2500 },
      { band: 7, correct: true, responseTimeMs: 3000 },
      { band: 9, correct: true, responseTimeMs: 2000 },
    ];
    const result = processIntakeResults(answers, null);
    expect(result.mathBand).toBe(9);
  });

  it('allows placement at configuredBand + 2 when kid demonstrates ability', () => {
    // Parent set band 4, kid proves they can handle band 6
    const answers = [
      { band: 3, correct: true, responseTimeMs: 2000 },
      { band: 5, correct: true, responseTimeMs: 2500 },
      { band: 7, correct: false, responseTimeMs: 5000 },
      { band: 6, correct: true, responseTimeMs: 3000 },
    ];
    const result = processIntakeResults(answers, 4);
    expect(result.mathBand).toBe(6); // configuredBand(4) + 2 = 6, kid proved band 6
  });
});
