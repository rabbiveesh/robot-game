import { describe, it, expect } from 'vitest';
import { createChallengeState, challengeReducer } from '../../../src/domain/challenge/challenge-state.js';

const MOCK_CHALLENGE = Object.freeze({
  correctAnswer: 7,
  displayText: 'What is 3 + 4?',
  speechText: 'What is 3 plus 4?',
  question: 'What is 3 + 4?',
  operation: 'add',
  subSkill: 'add_single',
});

const MOCK_CONTEXT = Object.freeze({ source: 'robot', npcName: 'Sparky' });

function create(contextOverrides = {}) {
  return createChallengeState(MOCK_CHALLENGE, { ...MOCK_CONTEXT, ...contextOverrides });
}

describe('createChallengeState', () => {
  it('starts in presented phase with null feedback', () => {
    const s = create();
    expect(s.phase).toBe('presented');
    expect(s.feedback).toBeNull();
    expect(s.reward).toBeNull();
    expect(s.correct).toBeNull();
    expect(s.attempts).toBe(0);
  });

  it('has display and speech question text', () => {
    const s = create();
    expect(s.question.display).toBe('What is 3 + 4?');
    expect(s.question.speech).toBe('What is 3 plus 4?');
  });

  it('voice state is reset', () => {
    const s = create();
    expect(s.voice.listening).toBe(false);
    expect(s.voice.confirming).toBe(false);
    expect(s.voice.retries).toBe(0);
    expect(s.voice.text).toBeNull();
  });

  it('new challenge state has no trace of previous challenge', () => {
    const s1 = create();
    const s2 = challengeReducer(s1, { type: 'ANSWER_SUBMITTED', answer: 7 });
    expect(s2.phase).toBe('complete');
    const s3 = create();
    expect(s3.phase).toBe('presented');
    expect(s3.feedback).toBeNull();
    expect(s3.reward).toBeNull();
    expect(s3.voice.retries).toBe(0);
  });

  it('scaffold state starts clean', () => {
    const s = create();
    expect(s.hintUsed).toBe(false);
    expect(s.hintLevel).toBe(0);
    expect(s.toldMe).toBe(false);
  });
});

describe('renderHint', () => {
  it('default renderHint is abstract/choice/quiz', () => {
    const s = create();
    expect(s.renderHint.craStage).toBe('abstract');
    expect(s.renderHint.answerMode).toBe('choice');
    expect(s.renderHint.interactionType).toBe('quiz');
  });

  it('new challenge state includes renderHint from context', () => {
    const s = create({
      renderHint: { craStage: 'concrete', answerMode: 'free_input', interactionType: 'puzzle' },
    });
    expect(s.renderHint.craStage).toBe('concrete');
    expect(s.renderHint.answerMode).toBe('free_input');
    expect(s.renderHint.interactionType).toBe('puzzle');
  });
});

describe('challengeReducer — ANSWER_SUBMITTED', () => {
  it('correct answer → complete phase with reward', () => {
    const s = challengeReducer(create(), { type: 'ANSWER_SUBMITTED', answer: 7 });
    expect(s.phase).toBe('complete');
    expect(s.correct).toBe(true);
    expect(s.reward).toEqual({ type: 'dum_dum', amount: 1 });
    expect(s.feedback.display).toMatch(/Amazing/);
    expect(s.feedback.speech).toMatch(/Amazing/);
  });

  it('wrong answer (first) → feedback phase, no reward', () => {
    const s = challengeReducer(create(), { type: 'ANSWER_SUBMITTED', answer: 5 });
    expect(s.phase).toBe('feedback');
    expect(s.correct).toBeNull();
    expect(s.reward).toBeNull();
    expect(s.attempts).toBe(1);
    expect(s.feedback.display).toMatch(/not quite/);
  });

  it('wrong answer (second) → teaching phase, no reward', () => {
    let s = challengeReducer(create(), { type: 'ANSWER_SUBMITTED', answer: 5 });
    s = challengeReducer(s, { type: 'ANSWER_SUBMITTED', answer: 3 });
    expect(s.phase).toBe('teaching');
    expect(s.correct).toBe(false);
    expect(s.reward).toBeNull();
    expect(s.attempts).toBe(2);
  });

  it('reward is ALWAYS null when incorrect', () => {
    let s = create();
    for (let i = 0; i < 3; i++) {
      s = challengeReducer(s, { type: 'ANSWER_SUBMITTED', answer: 999 });
      expect(s.reward).toBeNull();
    }
  });

  it('reward is ALWAYS present when correct', () => {
    const s = challengeReducer(create(), { type: 'ANSWER_SUBMITTED', answer: 7 });
    expect(s.reward).not.toBeNull();
    expect(s.reward.type).toBe('dum_dum');
  });

  it('voice state resets on complete', () => {
    let s = create();
    s = challengeReducer(s, { type: 'VOICE_LISTEN_START' });
    expect(s.voice.listening).toBe(true);
    s = challengeReducer(s, { type: 'ANSWER_SUBMITTED', answer: 7 });
    expect(s.voice.listening).toBe(false);
    expect(s.voice.retries).toBe(0);
  });
});

describe('challengeReducer — RETRY', () => {
  it('feedback resets on retry', () => {
    let s = challengeReducer(create(), { type: 'ANSWER_SUBMITTED', answer: 5 });
    expect(s.feedback).not.toBeNull();
    s = challengeReducer(s, { type: 'RETRY' });
    expect(s.phase).toBe('presented');
    expect(s.feedback).toBeNull();
  });
});

describe('challengeReducer — TEACHING_COMPLETE', () => {
  it('moves to complete phase', () => {
    let s = challengeReducer(create(), { type: 'ANSWER_SUBMITTED', answer: 5 });
    s = challengeReducer(s, { type: 'ANSWER_SUBMITTED', answer: 3 });
    expect(s.phase).toBe('teaching');
    s = challengeReducer(s, { type: 'TEACHING_COMPLETE' });
    expect(s.phase).toBe('complete');
  });
});

describe('challengeReducer — SHOW_ME', () => {
  it('drops CRA from abstract to representational', () => {
    const s = challengeReducer(create(), { type: 'SHOW_ME' });
    expect(s.renderHint.craStage).toBe('representational');
  });

  it('drops CRA from representational to concrete', () => {
    const s0 = create({ renderHint: { craStage: 'representational', answerMode: 'choice', interactionType: 'quiz' } });
    const s = challengeReducer(s0, { type: 'SHOW_ME' });
    expect(s.renderHint.craStage).toBe('concrete');
  });

  it('at concrete returns state unchanged', () => {
    const s0 = create({ renderHint: { craStage: 'concrete', answerMode: 'choice', interactionType: 'quiz' } });
    const s = challengeReducer(s0, { type: 'SHOW_ME' });
    expect(s).toBe(s0); // same reference — no change
  });

  it('sets hintUsed true and increments hintLevel', () => {
    let s = challengeReducer(create(), { type: 'SHOW_ME' });
    expect(s.hintUsed).toBe(true);
    expect(s.hintLevel).toBe(1);
    s = challengeReducer(s, { type: 'SHOW_ME' });
    expect(s.hintLevel).toBe(2);
  });

  it('SHOW_ME updates renderHint.craStage', () => {
    const s = challengeReducer(create(), { type: 'SHOW_ME' });
    expect(s.renderHint.craStage).toBe('representational');
    expect(s.renderHint.answerMode).toBe('choice'); // unchanged
    expect(s.renderHint.interactionType).toBe('quiz'); // unchanged
  });
});

describe('challengeReducer — TELL_ME', () => {
  it('sets phase to teaching with concrete CRA', () => {
    const s = challengeReducer(create(), { type: 'TELL_ME' });
    expect(s.phase).toBe('teaching');
    expect(s.renderHint.craStage).toBe('concrete');
  });

  it('sets toldMe true and reward null', () => {
    const s = challengeReducer(create(), { type: 'TELL_ME' });
    expect(s.toldMe).toBe(true);
    expect(s.reward).toBeNull();
  });

  it('feedback contains the correct answer', () => {
    const s = challengeReducer(create(), { type: 'TELL_ME' });
    expect(s.feedback.display).toContain('7');
    expect(s.feedback.speech).toContain('7');
  });
});

describe('challengeReducer — voice lifecycle', () => {
  it('VOICE_LISTEN_START sets listening true', () => {
    const s = challengeReducer(create(), { type: 'VOICE_LISTEN_START' });
    expect(s.voice.listening).toBe(true);
    expect(s.voice.text).toBeNull();
  });

  it('VOICE_RESULT with null number → retry, not wrong answer', () => {
    let s = challengeReducer(create(), { type: 'VOICE_LISTEN_START' });
    s = challengeReducer(s, { type: 'VOICE_RESULT', number: null, confidence: 0.9 });
    expect(s.voice.retries).toBe(1);
    expect(s.phase).toBe('presented');
  });

  it('VOICE_RESULT with confidence < 0.5 → retry', () => {
    let s = challengeReducer(create(), { type: 'VOICE_LISTEN_START' });
    s = challengeReducer(s, { type: 'VOICE_RESULT', number: 7, confidence: 0.3 });
    expect(s.voice.retries).toBe(1);
    expect(s.phase).toBe('presented');
  });

  it('VOICE_RESULT with confidence 0.5-0.8 → confirming', () => {
    let s = challengeReducer(create(), { type: 'VOICE_LISTEN_START' });
    s = challengeReducer(s, { type: 'VOICE_RESULT', number: 7, confidence: 0.6 });
    expect(s.voice.confirming).toBe(true);
    expect(s.voice.confirmNumber).toBe(7);
  });

  it('VOICE_RESULT with confidence >= 0.8 → ready to submit', () => {
    let s = challengeReducer(create(), { type: 'VOICE_LISTEN_START' });
    s = challengeReducer(s, { type: 'VOICE_RESULT', number: 7, confidence: 0.9 });
    expect(s.voice.listening).toBe(false);
    expect(s.voice.confirming).toBe(false);
  });

  it('VOICE_CONFIRM yes → ready to submit', () => {
    let s = challengeReducer(create(), { type: 'VOICE_LISTEN_START' });
    s = challengeReducer(s, { type: 'VOICE_RESULT', number: 7, confidence: 0.6 });
    s = challengeReducer(s, { type: 'VOICE_CONFIRM', confirmed: true });
    expect(s.voice.confirming).toBe(false);
  });

  it('VOICE_CONFIRM no → retry', () => {
    let s = challengeReducer(create(), { type: 'VOICE_LISTEN_START' });
    s = challengeReducer(s, { type: 'VOICE_RESULT', number: 30, confidence: 0.6 });
    s = challengeReducer(s, { type: 'VOICE_CONFIRM', confirmed: false });
    expect(s.voice.confirming).toBe(false);
    expect(s.voice.retries).toBe(1);
  });

  it('VOICE_ERROR not-allowed → mic blocked text', () => {
    let s = challengeReducer(create(), { type: 'VOICE_LISTEN_START' });
    s = challengeReducer(s, { type: 'VOICE_ERROR', error: 'not-allowed' });
    expect(s.voice.text.display).toBe('Mic blocked');
  });
});

describe('display/speech separation', () => {
  it('question has both display and speech fields', () => {
    const s = create();
    expect(s.question.display).toBeTruthy();
    expect(s.question.speech).toBeTruthy();
  });

  it('feedback has both display and speech fields', () => {
    const s = challengeReducer(create(), { type: 'ANSWER_SUBMITTED', answer: 7 });
    expect(s.feedback.display).toBeTruthy();
    expect(s.feedback.speech).toBeTruthy();
  });

  it('voice text has both display and speech fields', () => {
    let s = challengeReducer(create(), { type: 'VOICE_LISTEN_START' });
    s = challengeReducer(s, { type: 'VOICE_RESULT', number: null, confidence: 0.9 });
    expect(s.voice.text.display).toBeTruthy();
    expect(s.voice.text.speech).toBeTruthy();
  });
});
