import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';

let wasm;

beforeAll(async () => {
  const jsPath = join(import.meta.dirname, '../../dist/wasm/robot_buddy_domain.js');
  const wasmPath = join(import.meta.dirname, '../../dist/wasm/robot_buddy_domain_bg.wasm');
  const mod = await import(jsPath);
  const wasmBytes = await readFile(wasmPath);
  await mod.default(wasmBytes);
  wasm = mod;
});

// ─── HELPERS ────────────────────────────────────────────

function createProfile() { return JSON.parse(wasm.create_profile()); }
function reduce(state, event) { return JSON.parse(wasm.learner_reducer(JSON.stringify(state), JSON.stringify(event))); }
function genChallenge(profile) {
  return JSON.parse(wasm.generate_challenge(JSON.stringify({
    mathBand: profile.mathBand,
    spreadWidth: profile.spreadWidth ?? 0,
    operationStats: profile.operationStats,
  }), 42));
}
function challengeReduce(state, action) {
  return JSON.parse(wasm.challenge_reducer(JSON.stringify(state), JSON.stringify(action)));
}

function correctAttempt(band) {
  return { type: 'PUZZLE_ATTEMPTED', correct: true, operation: 'add', band, centerBand: band, responseTimeMs: 2000, hintUsed: false, toldMe: false };
}

// ─── LEARNER PROFILE FIELDS ─────────────────────────────

describe('LearnerProfile — all fields camelCase', () => {
  it('has all expected top-level fields', () => {
    const p = createProfile();
    const expected = [
      'mathBand', 'streak', 'pace', 'scaffolding', 'challengeFreq',
      'spreadWidth', 'promoteThreshold', 'stretchThreshold', 'wrongsBeforeTeach',
      'hintVisibility', 'textSpeed', 'framingStyle', 'representationStyle',
      'craStages', 'intakeCompleted', 'textSkipCount', 'rollingWindow', 'operationStats',
    ];
    for (const field of expected) {
      expect(p[field], `missing field: ${field}`).toBeDefined();
    }
  });

  it('has NO snake_case top-level fields', () => {
    const p = createProfile();
    const forbidden = [
      'math_band', 'spread_width', 'promote_threshold', 'stretch_threshold',
      'wrongs_before_teach', 'hint_visibility', 'text_speed', 'framing_style',
      'representation_style', 'cra_stages', 'intake_completed', 'text_skip_count',
      'rolling_window', 'operation_stats', 'challenge_freq',
    ];
    for (const field of forbidden) {
      expect(p[field], `snake_case found: ${field}`).toBeUndefined();
    }
  });

  it('craStages has all operations', () => {
    const p = createProfile();
    expect(p.craStages.add).toBe('concrete');
    expect(p.craStages.sub).toBe('concrete');
    expect(p.craStages.multiply).toBe('concrete');
    expect(p.craStages.divide).toBe('concrete');
    expect(p.craStages.number_bond).toBe('concrete');
  });

  it('rollingWindow has camelCase fields', () => {
    const p = createProfile();
    expect(p.rollingWindow.maxSize).toBe(20);
    expect(p.rollingWindow.entries).toEqual([]);
    expect(p.rollingWindow.max_size).toBeUndefined();
  });
});

// ─── OPERATION STATS (FLATTENED) ────────────────────────

describe('OperationStats — nested { coarse, fine }', () => {
  it('has coarse and fine maps', () => {
    const p = createProfile();
    expect(p.operationStats.coarse).toBeDefined();
    expect(p.operationStats.fine).toBeDefined();
  });

  it('coarse has all operations', () => {
    const p = createProfile();
    expect(p.operationStats.coarse.add).toBeDefined();
    expect(p.operationStats.coarse.sub).toBeDefined();
    expect(p.operationStats.coarse.multiply).toBeDefined();
    expect(p.operationStats.coarse.divide).toBeDefined();
    expect(p.operationStats.coarse.number_bond).toBeDefined();
  });

  it('fine has all sub-skills', () => {
    const p = createProfile();
    const fine = p.operationStats.fine;
    expect(fine.add_single).toBeDefined();
    expect(fine.add_carry).toBeDefined();
    expect(fine.sub_borrow).toBeDefined();
    expect(fine.mul_hard).toBeDefined();
    expect(fine.div_easy).toBeDefined();
    expect(fine.bond_small).toBeDefined();
  });

  it('stat entries have correct and attempts', () => {
    const p = createProfile();
    expect(p.operationStats.coarse.add.correct).toBe(0);
    expect(p.operationStats.coarse.add.attempts).toBe(0);
  });

  it('recording updates both coarse and fine', () => {
    let p = createProfile();
    p = reduce(p, { type: 'PUZZLE_ATTEMPTED', correct: true, operation: 'add', subSkill: 'add_carry', band: 6, centerBand: 6, responseTimeMs: 2000, hintUsed: false, toldMe: false });
    expect(p.operationStats.coarse.add.correct).toBe(1);
    expect(p.operationStats.fine.add_carry.correct).toBe(1);
  });
});

// ─── CHALLENGE — ALL FIELDS ─────────────────────────────

describe('Challenge — all fields camelCase', () => {
  it('has all expected fields', () => {
    const p = createProfile();
    const c = genChallenge(p);
    const expected = [
      'question', 'displayText', 'speechText', 'correctAnswer', 'choices',
      'operation', 'subSkill', 'features', 'centerBand', 'sampledBand', 'band', 'numbers',
    ];
    for (const field of expected) {
      expect(c[field], `missing: ${field}`).toBeDefined();
    }
  });

  it('features has camelCase fields', () => {
    const p = createProfile();
    const c = genChallenge(p);
    const f = c.features;
    const expected = [
      'carries', 'carriesTens', 'borrows', 'borrowsTens', 'crossesTenBoundary',
      'maxDigit', 'maxDigitGte7', 'hasRoundNumber', 'nearDoubles', 'answerSize',
      'answerGte10', 'answerGte20', 'answerGte50', 'operandSize', 'isSquare',
      'hasFactorFive', 'bothFactorsGt5',
    ];
    for (const field of expected) {
      expect(f[field] !== undefined, `missing feature: ${field}`).toBe(true);
    }
    // No snake_case
    expect(f.carries_tens).toBeUndefined();
    expect(f.max_digit).toBeUndefined();
    expect(f.answer_size).toBeUndefined();
  });

  it('choices are { text, correct }', () => {
    const c = genChallenge(createProfile());
    expect(c.choices.length).toBe(3);
    const corr = c.choices.filter(ch => ch.correct);
    expect(corr.length).toBe(1);
    expect(Number(corr[0].text)).toBe(c.correctAnswer);
  });

  it('numbers has a, b, op', () => {
    const c = genChallenge(createProfile());
    expect(c.numbers.a).toBeDefined();
    expect(c.numbers.b).toBeDefined();
    expect(c.numbers.op).toBeDefined();
  });
});

// ─── WINDOW ENTRIES ─────────────────────────────────────

describe('WindowEntry — all fields camelCase', () => {
  it('entry has camelCase fields after reducing', () => {
    let p = createProfile();
    p = reduce(p, correctAttempt(1));
    const e = p.rollingWindow.entries[0];
    expect(e.correct).toBe(true);
    expect(e.operation).toBe('add');
    expect(e.band).toBe(1);
    expect(e.centerBand).toBe(1);
    expect(e.responseTimeMs).toBe(2000);
    expect(e.hintUsed).toBe(false);
    expect(e.toldMe).toBe(false);
    expect(e.boredom).toBe(false);
    // No snake_case
    expect(e.center_band).toBeUndefined();
    expect(e.response_time_ms).toBeUndefined();
    expect(e.hint_used).toBeUndefined();
    expect(e.told_me).toBeUndefined();
    expect(e.cra_level_shown).toBeUndefined();
  });
});

// ─── CHALLENGE STATE ────────────────────────────────────

describe('ChallengeState — all fields camelCase', () => {
  function makeState() {
    return {
      phase: 'presented', correctAnswer: 7, attempts: 0, maxAttempts: 2,
      correct: null, question: { display: '3 + 4?', speech: '3 plus 4?' },
      feedback: null, reward: null,
      renderHint: { craStage: 'abstract', answerMode: 'choice', interactionType: 'quiz' },
      hintUsed: false, hintLevel: 0, toldMe: false,
      voice: { listening: false, confirming: false, confirmNumber: null, retries: 0, text: null },
    };
  }

  it('correct answer produces camelCase reward', () => {
    const s = challengeReduce(makeState(), { type: 'answerSubmitted', answer: 7 });
    expect(s.phase).toBe('complete');
    expect(s.reward).toBeDefined();
    expect(s.reward.rewardType).toBe('dum_dum');
    expect(s.reward.amount).toBe(1);
    expect(s.reward.reward_type).toBeUndefined();
  });

  it('renderHint has camelCase fields', () => {
    const s = challengeReduce(makeState(), { type: 'showMe' });
    expect(s.renderHint.craStage).toBe('representational');
    expect(s.renderHint.answerMode).toBe('choice');
    expect(s.renderHint.interactionType).toBe('quiz');
    expect(s.renderHint.cra_stage).toBeUndefined();
    expect(s.renderHint.answer_mode).toBeUndefined();
  });

  it('voice state has camelCase fields', () => {
    const s = challengeReduce(makeState(), { type: 'voiceResult', number: 7, confidence: 0.6 });
    expect(s.voice.confirming).toBe(true);
    expect(s.voice.confirmNumber).toBe(7);
    expect(s.voice.confirm_number).toBeUndefined();
  });

  it('feedback has display and speech', () => {
    const s = challengeReduce(makeState(), { type: 'answerSubmitted', answer: 5 });
    expect(s.feedback.display).toBeTruthy();
    expect(s.feedback.speech).toBeTruthy();
  });

  it('tellMe produces feedback with answer', () => {
    const s = challengeReduce(makeState(), { type: 'tellMe' });
    expect(s.toldMe).toBe(true);
    expect(s.feedback.display).toContain('7');
    expect(s.renderHint.craStage).toBe('concrete');
  });

  it('maxAttempts not max_attempts', () => {
    const s = makeState();
    expect(s.maxAttempts).toBe(2);
    const r = challengeReduce(s, { type: 'answerSubmitted', answer: 5 });
    expect(r.maxAttempts).toBe(2);
    expect(r.max_attempts).toBeUndefined();
  });
});

// ─── INTAKE ─────────────────────────────────────────────

describe('IntakeResult — all fields camelCase', () => {
  it('returns camelCase fields', () => {
    const answers = [
      { band: 3, correct: true, responseTimeMs: 2000, skippedText: false },
      { band: 5, correct: true, responseTimeMs: 2500, skippedText: false },
    ];
    const r = JSON.parse(wasm.process_intake_results(JSON.stringify(answers), -1));
    expect(r.mathBand).toBeDefined();
    expect(r.promoteThreshold).toBeDefined();
    expect(r.stretchThreshold).toBeDefined();
    expect(r.textSpeed).toBeDefined();
    expect(r.math_band).toBeUndefined();
    expect(r.promote_threshold).toBeUndefined();
  });
});

// ─── FRUSTRATION ────────────────────────────────────────

describe('FrustrationResult — fields', () => {
  it('returns level and recommendation', () => {
    const r = JSON.parse(wasm.detect_frustration(JSON.stringify({ entries: [], maxSize: 20 }), '[]'));
    expect(r.level).toBe('none');
    expect(r.recommendation).toBe('continue');
  });
});

// ─── ROUND-TRIP SAVE/LOAD ───────────────────────────────

describe('Save/load round-trip', () => {
  it('profile survives JSON serialization and deserialization through reducer', () => {
    let p = createProfile();
    for (let i = 0; i < 3; i++) {
      p = reduce(p, correctAttempt(1));
    }
    expect(p.streak).toBe(3);
    expect(p.craStages.add).toBe('representational');
    expect(p.rollingWindow.entries.length).toBe(3);
    expect(p.operationStats.coarse.add.correct).toBe(3);

    // Simulate save/load
    const saved = JSON.parse(JSON.stringify(p));
    const afterLoad = reduce(saved, correctAttempt(1));
    expect(afterLoad.mathBand).toBe(2); // promoted
    expect(afterLoad.rollingWindow.entries.length).toBe(4);
    expect(afterLoad.operationStats.coarse.add.correct).toBe(4);
  });

  it('window maxSize survives round-trip', () => {
    let p = createProfile();
    p = reduce(p, correctAttempt(1));
    const saved = JSON.parse(JSON.stringify(p));
    expect(saved.rollingWindow.maxSize).toBe(20);
    const afterLoad = reduce(saved, correctAttempt(1));
    expect(afterLoad.rollingWindow.maxSize).toBe(20);
  });
});

// ─── OLD SAVE COMPATIBILITY ─────────────────────────────

describe('Old save compatibility — missing fields get defaults', () => {
  it('profile with missing textSkipCount deserializes through reducer', () => {
    // Simulate an old save that lacks textSkipCount
    const oldProfile = JSON.parse(wasm.create_profile());
    delete oldProfile.textSkipCount;
    const result = reduce(oldProfile, { type: 'BEHAVIOR', signal: 'text_skipped' });
    expect(result.textSkipCount).toBe(1); // default 0 + 1
  });

  it('profile with missing spreadWidth deserializes through reducer', () => {
    const oldProfile = JSON.parse(wasm.create_profile());
    delete oldProfile.spreadWidth;
    const result = reduce(oldProfile, correctAttempt(1));
    expect(result.spreadWidth).toBeDefined();
  });

  it('profile with missing craStages deserializes through reducer', () => {
    const oldProfile = JSON.parse(wasm.create_profile());
    delete oldProfile.craStages;
    const result = reduce(oldProfile, correctAttempt(1));
    expect(result.craStages).toBeDefined();
    expect(result.craStages.add).toBe('concrete');
  });

  it('profile with missing rollingWindow deserializes through reducer', () => {
    const oldProfile = JSON.parse(wasm.create_profile());
    delete oldProfile.rollingWindow;
    const result = reduce(oldProfile, correctAttempt(1));
    expect(result.rollingWindow).toBeDefined();
    expect(result.rollingWindow.entries.length).toBe(1);
  });
});
