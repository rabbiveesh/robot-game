import { describe, it, expect } from 'vitest';
import { createProfile, learnerReducer } from '../../../src/domain/learning/learner-profile.js';
import { createWindow } from '../../../src/domain/learning/rolling-window.js';

function makeAttempt(correct, overrides = {}) {
  return {
    type: 'PUZZLE_ATTEMPTED',
    correct,
    operation: 'add',
    band: 1,
    centerBand: 1,
    responseTimeMs: 2000,
    attemptNumber: 1,
    ...overrides,
  };
}

// Helper: feed N correct attempts at a given band into the reducer
function feedAttempts(state, count, correct, band) {
  for (let i = 0; i < count; i++) {
    state = learnerReducer(state, makeAttempt(correct, { band, centerBand: state.mathBand }));
  }
  return state;
}

describe('createProfile', () => {
  it('creates a frozen profile with defaults', () => {
    const p = createProfile();
    expect(p.mathBand).toBe(1);
    expect(p.streak).toBe(0);
    expect(p.spreadWidth).toBe(0.5);
    expect(p.promoteThreshold).toBe(0.75);
    expect(p.stretchThreshold).toBe(0.60);
    expect(p.intakeCompleted).toBe(false);
    expect(Object.isFrozen(p)).toBe(true);
  });

  it('accepts overrides', () => {
    const p = createProfile({ mathBand: 5, pace: 0.8, spreadWidth: 0.3 });
    expect(p.mathBand).toBe(5);
    expect(p.pace).toBe(0.8);
    expect(p.spreadWidth).toBe(0.3);
  });
});

describe('learnerReducer — PUZZLE_ATTEMPTED (streak display)', () => {
  it('increments streak on correct answer', () => {
    const s0 = createProfile();
    const s1 = learnerReducer(s0, makeAttempt(true));
    expect(s1.streak).toBe(1);
  });

  it('decrements streak on wrong answer', () => {
    const s0 = createProfile();
    const s1 = learnerReducer(s0, makeAttempt(false));
    expect(s1.streak).toBe(-1);
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
  });

  it('pushes to rolling window with band and centerBand', () => {
    let state = createProfile({ mathBand: 5 });
    state = learnerReducer(state, makeAttempt(true, { band: 6, centerBand: 5 }));
    expect(state.rollingWindow.entries).toHaveLength(1);
    expect(state.rollingWindow.entries[0].band).toBe(6);
    expect(state.rollingWindow.entries[0].centerBand).toBe(5);
  });

  it('records subSkill in window entry and operation stats', () => {
    let state = createProfile();
    state = learnerReducer(state, makeAttempt(true, { operation: 'add', subSkill: 'add_carry' }));
    expect(state.rollingWindow.entries[0].subSkill).toBe('add_carry');
    expect(state.operationStats.add.correct).toBe(1);
    expect(state.operationStats.add_carry.correct).toBe(1);
  });
});

describe('learnerReducer — accuracy-based promotion', () => {
  it('promotes when center accuracy >= 75% and stretch accuracy >= 60%', () => {
    let state = createProfile({ mathBand: 5 });
    // 4 correct at center band (100% accuracy at center)
    state = feedAttempts(state, 4, true, 5);
    // 2 correct at above-center (100% accuracy above)
    state = feedAttempts(state, 2, true, 6);
    expect(state.mathBand).toBe(6);
  });

  it('does not promote when center accuracy is below threshold', () => {
    let state = createProfile({ mathBand: 5 });
    // 2 correct, 2 wrong at center (50% < 75%)
    state = feedAttempts(state, 2, true, 5);
    state = feedAttempts(state, 2, false, 5);
    state = feedAttempts(state, 2, true, 6);
    expect(state.mathBand).toBe(5);
  });

  it('does not promote with insufficient attempts at center', () => {
    let state = createProfile({ mathBand: 5 });
    // Only 3 correct at center (need 4)
    state = feedAttempts(state, 3, true, 5);
    state = feedAttempts(state, 2, true, 6);
    expect(state.mathBand).toBe(5);
  });

  it('promotes even without a streak if accuracy is sustained', () => {
    let state = createProfile({ mathBand: 5 });
    // correct, wrong, correct, correct, correct at center = 80% with 5 attempts
    state = learnerReducer(state, makeAttempt(true, { band: 5, centerBand: 5 }));
    state = learnerReducer(state, makeAttempt(false, { band: 5, centerBand: 5 }));
    state = learnerReducer(state, makeAttempt(true, { band: 5, centerBand: 5 }));
    state = learnerReducer(state, makeAttempt(true, { band: 5, centerBand: 5 }));
    state = learnerReducer(state, makeAttempt(true, { band: 5, centerBand: 5 }));
    // 2 correct above center
    state = feedAttempts(state, 2, true, 6);
    expect(state.mathBand).toBe(6);
  });

  it('promotes without above-center data if spread is tight', () => {
    let state = createProfile({ mathBand: 5, spreadWidth: 0.0 });
    // 4 correct at center, no above-center attempts
    state = feedAttempts(state, 4, true, 5);
    expect(state.mathBand).toBe(6);
  });

  it('does not promote above band 10', () => {
    let state = createProfile({ mathBand: 10 });
    state = feedAttempts(state, 6, true, 10);
    expect(state.mathBand).toBe(10);
  });

  it('demotes when center accuracy < 50% with enough attempts', () => {
    let state = createProfile({ mathBand: 5 });
    // 1 correct, 3 wrong at center = 25%
    state = feedAttempts(state, 1, true, 5);
    state = feedAttempts(state, 3, false, 5);
    expect(state.mathBand).toBe(4);
  });

  it('does not demote with insufficient attempts', () => {
    let state = createProfile({ mathBand: 5 });
    // Only 3 attempts at center
    state = feedAttempts(state, 1, true, 5);
    state = feedAttempts(state, 2, false, 5);
    expect(state.mathBand).toBe(5);
  });

  it('does not demote below band 1', () => {
    let state = createProfile({ mathBand: 1 });
    state = feedAttempts(state, 4, false, 1);
    expect(state.mathBand).toBe(1);
  });
});

describe('learnerReducer — spread width adaptation', () => {
  it('tightens on demotion', () => {
    let state = createProfile({ mathBand: 5, spreadWidth: 0.5 });
    state = feedAttempts(state, 4, false, 5); // demote
    expect(state.mathBand).toBe(4);
    expect(state.spreadWidth).toBeLessThan(0.5);
  });

  it('tightens on frustration', () => {
    const s0 = createProfile({ mathBand: 4, spreadWidth: 0.5 });
    const s1 = learnerReducer(s0, { type: 'FRUSTRATION_DETECTED', level: 'high' });
    expect(s1.spreadWidth).toBeLessThan(0.5);
  });

  it('tightens briefly after promotion', () => {
    let state = createProfile({ mathBand: 5, spreadWidth: 0.5 });
    state = feedAttempts(state, 4, true, 5);
    state = feedAttempts(state, 2, true, 6);
    expect(state.mathBand).toBe(6);
    expect(state.spreadWidth).toBeLessThan(0.5);
  });

  it('widens on sustained good accuracy', () => {
    // Need 10+ entries with >75% accuracy, no promotion trigger.
    // Keep center accuracy at 75% exactly (3/4) so we don't promote,
    // but rolling accuracy is high overall.
    let state = createProfile({ mathBand: 5, spreadWidth: 0.3 });
    // 3 correct at center, 1 wrong at center = 75% at center (not > 75%, so no promote)
    state = feedAttempts(state, 3, true, 5);
    state = feedAttempts(state, 1, false, 5);
    // Pad with correct below-center to raise rolling accuracy above 75%
    state = feedAttempts(state, 8, true, 4);
    expect(state.spreadWidth).toBeGreaterThan(0.3);
  });

  it('does not widen above 1.0 or tighten below 0.1', () => {
    const s0 = createProfile({ mathBand: 4, spreadWidth: 0.1 });
    const s1 = learnerReducer(s0, { type: 'FRUSTRATION_DETECTED', level: 'high' });
    expect(s1.spreadWidth).toBeGreaterThanOrEqual(0.1);

    let state = createProfile({ mathBand: 5, spreadWidth: 0.95 });
    for (let i = 0; i < 15; i++) {
      state = learnerReducer(state, makeAttempt(true, { band: 4 + (i % 3), centerBand: 5 }));
    }
    expect(state.spreadWidth).toBeLessThanOrEqual(1.0);
  });

  it('widens faster at band 10 ceiling (0.1 instead of 0.05)', () => {
    let state = createProfile({ mathBand: 10, spreadWidth: 0.3 });
    // Feed attempts spread across bands so we don't trigger promotion (already at ceiling)
    for (let i = 0; i < 12; i++) {
      const band = 9 + (i % 2); // alternate 9 and 10
      state = learnerReducer(state, makeAttempt(true, { band, centerBand: 10 }));
    }
    // Should have widened by 0.1 per tick (at ceiling), not 0.05
    expect(state.spreadWidth).toBeGreaterThan(0.4);
  });
});

describe('learnerReducer — scaffolding adaptation', () => {
  it('scaffolding decreases on sustained high accuracy (>85% over 10+)', () => {
    let state = createProfile({ mathBand: 5, scaffolding: 0.7 });
    // 12 correct at various bands = 100% accuracy
    for (let i = 0; i < 12; i++) {
      state = learnerReducer(state, makeAttempt(true, { band: 4 + (i % 3), centerBand: 5 }));
    }
    expect(state.scaffolding).toBeLessThan(0.7);
  });

  it('scaffolding increases on sustained low accuracy (<50% over 10+)', () => {
    let state = createProfile({ mathBand: 5, scaffolding: 0.3 });
    // Mix: 3 correct, 9 wrong = 25% accuracy
    state = feedAttempts(state, 3, true, 5);
    state = feedAttempts(state, 9, false, 5);
    expect(state.scaffolding).toBeGreaterThan(0.3);
  });

  it('scaffolding does not go below 0 or above 1', () => {
    let state = createProfile({ mathBand: 5, scaffolding: 0.02 });
    for (let i = 0; i < 15; i++) {
      state = learnerReducer(state, makeAttempt(true, { band: 4 + (i % 3), centerBand: 5 }));
    }
    expect(state.scaffolding).toBeGreaterThanOrEqual(0);

    let state2 = createProfile({ mathBand: 5, scaffolding: 0.98 });
    state2 = feedAttempts(state2, 2, true, 5);
    state2 = feedAttempts(state2, 10, false, 5);
    expect(state2.scaffolding).toBeLessThanOrEqual(1);
  });
});

describe('learnerReducer — pace adaptation', () => {
  it('pace increases on fast correct answers (<3s)', () => {
    let state = createProfile({ mathBand: 5, pace: 0.5 });
    state = learnerReducer(state, makeAttempt(true, { responseTimeMs: 1500 }));
    expect(state.pace).toBeGreaterThan(0.5);
  });

  it('pace decreases on slow correct answers (>10s)', () => {
    let state = createProfile({ mathBand: 5, pace: 0.5 });
    state = learnerReducer(state, makeAttempt(true, { responseTimeMs: 12000 }));
    expect(state.pace).toBeLessThan(0.5);
  });

  it('pace does not adjust on null responseTimeMs', () => {
    let state = createProfile({ mathBand: 5, pace: 0.5 });
    state = learnerReducer(state, makeAttempt(true, { responseTimeMs: null }));
    expect(state.pace).toBe(0.5);
  });

  it('pace does not adjust on wrong answers', () => {
    let state = createProfile({ mathBand: 5, pace: 0.5 });
    state = learnerReducer(state, makeAttempt(false, { responseTimeMs: 1500 }));
    expect(state.pace).toBe(0.5);
  });
});

describe('learnerReducer — boredom detection', () => {
  it('boredom pattern: fast wrong between corrects is not a real failure', () => {
    let state = createProfile({ mathBand: 3 });
    state = learnerReducer(state, makeAttempt(true));
    state = learnerReducer(state, makeAttempt(true));
    expect(state.streak).toBe(2);
    state = learnerReducer(state, makeAttempt(false, { responseTimeMs: 500 }));
    expect(state.streak).toBeGreaterThanOrEqual(0);
    expect(state.mathBand).toBe(3);
  });

  it('boredom requires 2 prior corrects — single correct + fast wrong penalizes normally', () => {
    let state = createProfile({ mathBand: 3 });
    state = learnerReducer(state, makeAttempt(true));
    state = learnerReducer(state, makeAttempt(false, { responseTimeMs: 500 }));
    expect(state.streak).toBe(-1);
  });

  it('slow wrong is not boredom — penalizes normally', () => {
    let state = createProfile({ mathBand: 3 });
    state = learnerReducer(state, makeAttempt(true));
    state = learnerReducer(state, makeAttempt(true));
    state = learnerReducer(state, makeAttempt(false, { responseTimeMs: 5000 }));
    expect(state.streak).toBe(-1);
  });
});

describe('learnerReducer — BEHAVIOR', () => {
  it('text_skipped increases pace, decreases textSpeed, and increments skipCount', () => {
    const s0 = createProfile({ pace: 0.5, textSpeed: 0.035 });
    const s1 = learnerReducer(s0, { type: 'BEHAVIOR', signal: 'text_skipped' });
    expect(s1.pace).toBeCloseTo(0.6);
    expect(s1.textSpeed).toBeCloseTo(0.03);
    expect(s1.textSkipCount).toBe(1);
  });

  it('text_skipped still increments skipCount when pace is capped at 1.0', () => {
    const s0 = createProfile({ pace: 1.0, textSpeed: 0.01 });
    const s1 = learnerReducer(s0, { type: 'BEHAVIOR', signal: 'text_skipped' });
    expect(s1.pace).toBe(1.0);
    expect(s1.textSkipCount).toBe(1);
    const s2 = learnerReducer(s1, { type: 'BEHAVIOR', signal: 'text_skipped' });
    expect(s2.textSkipCount).toBe(2);
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
  it('high frustration drops band, tightens spread, reduces wrongsBeforeTeach', () => {
    const s0 = createProfile({ mathBand: 4, wrongsBeforeTeach: 2, pace: 0.5, spreadWidth: 0.5 });
    const s1 = learnerReducer(s0, { type: 'FRUSTRATION_DETECTED', level: 'high' });
    expect(s1.mathBand).toBe(3);
    expect(s1.wrongsBeforeTeach).toBe(1);
    expect(s1.pace).toBeCloseTo(0.3);
    expect(s1.streak).toBe(0);
    expect(s1.spreadWidth).toBeLessThan(0.5);
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
      promoteThreshold: 0.65,
      stretchThreshold: 0.50,
      textSpeed: 0.02,
    });
    expect(s1.mathBand).toBe(5);
    expect(s1.pace).toBe(0.7);
    expect(s1.scaffolding).toBe(0.3);
    expect(s1.promoteThreshold).toBe(0.65);
    expect(s1.stretchThreshold).toBe(0.50);
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
