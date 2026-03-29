import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';

// Load the WASM module in Node — same exports as the browser bridge
let wasm;

beforeAll(async () => {
  // wasm-pack builds a .js glue file that works in Node with minor shimming
  const wasmPath = join(import.meta.dirname, '../../dist/wasm/robot_buddy_domain_bg.wasm');
  const jsPath = join(import.meta.dirname, '../../dist/wasm/robot_buddy_domain.js');

  // Import the JS glue
  const mod = await import(jsPath);
  const wasmBytes = await readFile(wasmPath);
  await mod.default(wasmBytes);
  wasm = mod;
});

describe('WASM bridge integration — field names are camelCase', () => {
  it('create_profile returns camelCase fields', () => {
    const profile = JSON.parse(wasm.create_profile());
    // Must be camelCase, not snake_case
    expect(profile.mathBand).toBeDefined();
    expect(profile.spreadWidth).toBeDefined();
    expect(profile.craStages).toBeDefined();
    expect(profile.rollingWindow).toBeDefined();
    expect(profile.operationStats).toBeDefined();
    expect(profile.promoteThreshold).toBeDefined();
    expect(profile.intakeCompleted).toBeDefined();
    expect(profile.textSkipCount).toBeDefined();
    // Must NOT have snake_case
    expect(profile.math_band).toBeUndefined();
    expect(profile.spread_width).toBeUndefined();
    expect(profile.cra_stages).toBeUndefined();
  });

  it('generate_challenge returns camelCase fields', () => {
    const profile = JSON.parse(wasm.create_profile());
    const challenge = JSON.parse(wasm.generate_challenge(JSON.stringify({
      mathBand: profile.mathBand,
      spreadWidth: 0,
      operationStats: profile.operationStats,
    }), 42));
    expect(challenge.displayText).toBeDefined();
    expect(challenge.speechText).toBeDefined();
    expect(challenge.correctAnswer).toBeDefined();
    expect(challenge.sampledBand).toBeDefined();
    expect(challenge.centerBand).toBeDefined();
    expect(challenge.subSkill).toBeDefined();
    // Must NOT have snake_case
    expect(challenge.display_text).toBeUndefined();
    expect(challenge.correct_answer).toBeUndefined();
  });

  it('learner_reducer accepts and returns camelCase', () => {
    const profile = JSON.parse(wasm.create_profile());
    const event = {
      type: 'PUZZLE_ATTEMPTED',
      correct: true,
      operation: 'add',
      band: 1,
      centerBand: 1,
      responseTimeMs: 2000,
      hintUsed: false,
      toldMe: false,
    };
    const newProfile = JSON.parse(wasm.learner_reducer(
      JSON.stringify(profile),
      JSON.stringify(event),
    ));
    expect(newProfile.mathBand).toBeDefined();
    expect(newProfile.streak).toBe(1);
    expect(newProfile.rollingWindow.entries).toHaveLength(1);
    const entry = newProfile.rollingWindow.entries[0];
    expect(entry.correct).toBe(true);
    expect(entry.operation).toBe('add');
    expect(entry.responseTimeMs).toBe(2000);
    // snake_case must not appear
    expect(newProfile.math_band).toBeUndefined();
  });

  it('challenge_reducer accepts and returns camelCase', () => {
    const state = {
      phase: 'presented',
      correctAnswer: 7,
      attempts: 0,
      maxAttempts: 2,
      correct: null,
      question: { display: 'What is 3 + 4?', speech: 'What is 3 plus 4?' },
      feedback: null,
      reward: null,
      renderHint: { craStage: 'abstract', answerMode: 'choice', interactionType: 'quiz' },
      hintUsed: false,
      hintLevel: 0,
      toldMe: false,
      voice: { listening: false, confirming: false, confirmNumber: null, retries: 0, text: null },
    };
    const action = { type: 'answerSubmitted', answer: 7 };
    const result = JSON.parse(wasm.challenge_reducer(
      JSON.stringify(state),
      JSON.stringify(action),
    ));
    expect(result.phase).toBe('complete');
    expect(result.correct).toBe(true);
    expect(result.reward).toBeDefined();
    expect(result.reward.rewardType).toBe('dum_dum');
    // snake_case must not appear
    expect(result.correct_answer).toBeUndefined();
    expect(result.reward.reward_type).toBeUndefined();
  });

  it('full round-trip: create profile, generate challenge, submit answer', () => {
    // Create
    const profile = JSON.parse(wasm.create_profile());
    expect(profile.mathBand).toBe(1);

    // Generate
    const challenge = JSON.parse(wasm.generate_challenge(JSON.stringify({
      mathBand: 1,
      spreadWidth: 0,
      operationStats: profile.operationStats,
    }), 42));
    expect(challenge.correctAnswer).toBeGreaterThan(0);
    expect(challenge.choices.length).toBe(3);

    // Submit correct answer
    const event = {
      type: 'PUZZLE_ATTEMPTED',
      correct: true,
      operation: challenge.operation,
      band: challenge.band,
      centerBand: 1,
      responseTimeMs: 1500,
      hintUsed: false,
      toldMe: false,
    };
    const updated = JSON.parse(wasm.learner_reducer(
      JSON.stringify(profile),
      JSON.stringify(event),
    ));
    expect(updated.streak).toBe(1);
    expect(updated.rollingWindow.entries.length).toBe(1);
  });

  it('detect_frustration returns camelCase', () => {
    const window = { entries: [], maxSize: 20 };
    const result = JSON.parse(wasm.detect_frustration(
      JSON.stringify(window),
      JSON.stringify([]),
    ));
    expect(result.level).toBe('none');
    expect(result.recommendation).toBe('continue');
  });

  it('process_intake_results returns camelCase', () => {
    const answers = [
      { band: 3, correct: true, responseTimeMs: 2000, skippedText: false },
      { band: 5, correct: true, responseTimeMs: 2500, skippedText: false },
      { band: 7, correct: false, responseTimeMs: 5000, skippedText: false },
      { band: 6, correct: true, responseTimeMs: 3000, skippedText: false },
    ];
    const result = JSON.parse(wasm.process_intake_results(JSON.stringify(answers), -1));
    expect(result.mathBand).toBeDefined();
    expect(result.promoteThreshold).toBeDefined();
    expect(result.math_band).toBeUndefined();
  });
});
