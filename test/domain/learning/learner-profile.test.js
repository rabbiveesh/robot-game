import { describe, it, expect } from 'vitest';
import { createProfile, learnerReducer } from '../../../src/domain/learning/learner-profile.js';

function makeAttempt(correct, overrides = {}) {
  return {
    type: 'PUZZLE_ATTEMPTED',
    correct,
    operation: 'add',
    band: 1,
    responseTimeMs: 2000,
    attemptNumber: 1,
    ...overrides,
  };
}

describe('createProfile', () => {
  it('creates a frozen profile with defaults', () => {
    const p = createProfile();
    expect(p.mathBand).toBe(1);
    expect(p.streak).toBe(0);
    expect(p.intakeCompleted).toBe(false);
    expect(Object.isFrozen(p)).toBe(true);
  });

  it('accepts overrides', () => {
    const p = createProfile({ mathBand: 5, pace: 0.8 });
    expect(p.mathBand).toBe(5);
    expect(p.pace).toBe(0.8);
  });
});

describe('learnerReducer — PUZZLE_ATTEMPTED', () => {
  it('increments streak on correct answer', () => {
    const s0 = createProfile();
    const s1 = learnerReducer(s0, makeAttempt(true));
    expect(s1.streak).toBe(1);
  });

  it('promotes band after N correct in a row (configurable N)', () => {
    let state = createProfile({ streakToPromote: 3 });
    state = learnerReducer(state, makeAttempt(true));
    state = learnerReducer(state, makeAttempt(true));
    expect(state.streak).toBe(2);
    expect(state.mathBand).toBe(1);
    state = learnerReducer(state, makeAttempt(true));
    expect(state.mathBand).toBe(2);
    expect(state.streak).toBe(0);
  });

  it('promotes with streakToPromote = 2', () => {
    let state = createProfile({ streakToPromote: 2 });
    state = learnerReducer(state, makeAttempt(true));
    state = learnerReducer(state, makeAttempt(true));
    expect(state.mathBand).toBe(2);
  });

  it('decrements streak on wrong answer', () => {
    const s0 = createProfile();
    const s1 = learnerReducer(s0, makeAttempt(false));
    expect(s1.streak).toBe(-1);
  });

  it('demotes band after 2 wrong in a row', () => {
    let state = createProfile({ mathBand: 3 });
    state = learnerReducer(state, makeAttempt(false));
    expect(state.mathBand).toBe(3);
    state = learnerReducer(state, makeAttempt(false));
    expect(state.mathBand).toBe(2);
    expect(state.streak).toBe(0);
  });

  it('does not demote below band 1', () => {
    let state = createProfile({ mathBand: 1 });
    state = learnerReducer(state, makeAttempt(false));
    state = learnerReducer(state, makeAttempt(false));
    expect(state.mathBand).toBe(1);
  });

  it('does not promote above band 10', () => {
    let state = createProfile({ mathBand: 10, streakToPromote: 2 });
    state = learnerReducer(state, makeAttempt(true));
    state = learnerReducer(state, makeAttempt(true));
    expect(state.mathBand).toBe(10);
  });

  it('boredom pattern: fast wrong between corrects is not a real failure', () => {
    let state = createProfile({ mathBand: 3 });
    // Two corrects
    state = learnerReducer(state, makeAttempt(true));
    state = learnerReducer(state, makeAttempt(true));
    expect(state.streak).toBe(2);

    // Fast wrong (< 2s) — boredom pattern
    state = learnerReducer(state, makeAttempt(false, { responseTimeMs: 500 }));
    // Should not go negative — boredom wrongs don't penalize
    expect(state.streak).toBeGreaterThanOrEqual(0);
    expect(state.mathBand).toBe(3); // no demotion
  });

  it('boredom requires 2 prior corrects — single correct + fast wrong penalizes normally', () => {
    let state = createProfile({ mathBand: 3 });
    // Only one correct before fast wrong — not enough for boredom pattern
    state = learnerReducer(state, makeAttempt(true));
    state = learnerReducer(state, makeAttempt(false, { responseTimeMs: 500 }));
    // Should penalize normally since there's only 1 prior correct, not 2
    expect(state.streak).toBe(-1);
  });

  it('slow wrong is not boredom — penalizes normally', () => {
    let state = createProfile({ mathBand: 3 });
    state = learnerReducer(state, makeAttempt(true));
    state = learnerReducer(state, makeAttempt(true));
    // Slow wrong (> 2s) — real mistake
    state = learnerReducer(state, makeAttempt(false, { responseTimeMs: 5000 }));
    expect(state.streak).toBe(-1);
  });

  it('state is frozen — original state unchanged after reduction', () => {
    const s0 = createProfile();
    const s1 = learnerReducer(s0, makeAttempt(true));
    expect(s0.streak).toBe(0);
    expect(s1.streak).toBe(1);
    expect(Object.isFrozen(s1)).toBe(true);
  });

  it('tracks operation stats', () => {
    let state = createProfile();
    state = learnerReducer(state, makeAttempt(true, { operation: 'add' }));
    state = learnerReducer(state, makeAttempt(false, { operation: 'add' }));
    state = learnerReducer(state, makeAttempt(true, { operation: 'sub' }));
    expect(state.operationStats.add.correct).toBe(1);
    expect(state.operationStats.add.attempts).toBe(2);
    expect(state.operationStats.sub.correct).toBe(1);
    expect(state.operationStats.sub.attempts).toBe(1);
  });

  it('pushes to rolling window', () => {
    let state = createProfile();
    state = learnerReducer(state, makeAttempt(true));
    expect(state.rollingWindow.entries).toHaveLength(1);
    expect(state.rollingWindow.entries[0].correct).toBe(true);
  });
});

describe('learnerReducer — BEHAVIOR', () => {
  it('text_skipped increases pace and decreases textSpeed', () => {
    const s0 = createProfile({ pace: 0.5, textSpeed: 0.035 });
    const s1 = learnerReducer(s0, { type: 'BEHAVIOR', signal: 'text_skipped' });
    expect(s1.pace).toBeCloseTo(0.6);
    expect(s1.textSpeed).toBeCloseTo(0.03);
  });

  it('rapid_clicking reduces wrongsBeforeTeach', () => {
    const s0 = createProfile({ wrongsBeforeTeach: 2 });
    const s1 = learnerReducer(s0, { type: 'BEHAVIOR', signal: 'rapid_clicking' });
    expect(s1.wrongsBeforeTeach).toBe(1);
  });

  it('rapid_clicking does not reduce wrongsBeforeTeach below 1', () => {
    const s0 = createProfile({ wrongsBeforeTeach: 1 });
    const s1 = learnerReducer(s0, { type: 'BEHAVIOR', signal: 'rapid_clicking' });
    expect(s1.wrongsBeforeTeach).toBe(1);
  });
});

describe('learnerReducer — FRUSTRATION_DETECTED', () => {
  it('high frustration drops band and reduces wrongsBeforeTeach', () => {
    const s0 = createProfile({ mathBand: 4, wrongsBeforeTeach: 2, pace: 0.5 });
    const s1 = learnerReducer(s0, { type: 'FRUSTRATION_DETECTED', level: 'high' });
    expect(s1.mathBand).toBe(3);
    expect(s1.wrongsBeforeTeach).toBe(1);
    expect(s1.pace).toBeCloseTo(0.3);
    expect(s1.streak).toBe(0);
  });

  it('high frustration does not drop below band 1', () => {
    const s0 = createProfile({ mathBand: 1 });
    const s1 = learnerReducer(s0, { type: 'FRUSTRATION_DETECTED', level: 'high' });
    expect(s1.mathBand).toBe(1);
  });

  it('mild frustration does not change state', () => {
    const s0 = createProfile({ mathBand: 4 });
    const s1 = learnerReducer(s0, { type: 'FRUSTRATION_DETECTED', level: 'mild' });
    expect(s1).toBe(s0);
  });
});

describe('learnerReducer — INTAKE_COMPLETED', () => {
  it('sets dials from intake results', () => {
    const s0 = createProfile();
    const s1 = learnerReducer(s0, {
      type: 'INTAKE_COMPLETED',
      mathBand: 5,
      pace: 0.7,
      scaffolding: 0.3,
      streakToPromote: 2,
      textSpeed: 0.02,
    });
    expect(s1.mathBand).toBe(5);
    expect(s1.pace).toBe(0.7);
    expect(s1.scaffolding).toBe(0.3);
    expect(s1.streakToPromote).toBe(2);
    expect(s1.textSpeed).toBe(0.02);
    expect(s1.intakeCompleted).toBe(true);
  });
});

describe('learnerReducer — unknown event', () => {
  it('returns state unchanged', () => {
    const s0 = createProfile();
    const s1 = learnerReducer(s0, { type: 'UNKNOWN_EVENT' });
    expect(s1).toBe(s0);
  });
});
