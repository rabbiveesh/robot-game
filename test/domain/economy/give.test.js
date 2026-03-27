import { describe, it, expect } from 'vitest';
import { canGive, processGive } from '../../../src/domain/economy/give.js';

describe('canGive', () => {
  it('returns true when dumDums > 0', () => {
    expect(canGive(5)).toBe(true);
    expect(canGive(1)).toBe(true);
  });

  it('returns false when dumDums === 0', () => {
    expect(canGive(0)).toBe(false);
  });
});

describe('processGive', () => {
  it('decrements dumDums by 1', () => {
    const result = processGive(5, 'robot', {});
    expect(result.newDumDums).toBe(4);
  });

  it('increments recipient total', () => {
    const result = processGive(5, 'robot', { robot: 3 });
    expect(result.newTotalGifts.robot).toBe(4);
  });

  it('starts recipient total at 1 if not previously given', () => {
    const result = processGive(5, 'kid_1', {});
    expect(result.newTotalGifts.kid_1).toBe(1);
  });

  it('returns milestone at count 1', () => {
    const result = processGive(5, 'robot', {});
    expect(result.milestone).toEqual({ recipientId: 'robot', total: 1, reaction: 'first' });
  });

  it('returns milestone at count 5', () => {
    const result = processGive(5, 'robot', { robot: 4 });
    expect(result.milestone.reaction).toBe('spin');
  });

  it('returns milestone at count 10', () => {
    const result = processGive(5, 'robot', { robot: 9 });
    expect(result.milestone.reaction).toBe('accessory');
  });

  it('returns milestone at count 20', () => {
    const result = processGive(5, 'robot', { robot: 19 });
    expect(result.milestone.reaction).toBe('color_change');
  });

  it('returns milestone at count 50', () => {
    const result = processGive(5, 'robot', { robot: 49 });
    expect(result.milestone.reaction).toBe('ultimate');
  });

  it('returns null milestone between thresholds', () => {
    const result = processGive(5, 'robot', { robot: 2 });
    expect(result.milestone).toBeNull();
  });

  it('returns null when dumDums === 0', () => {
    expect(processGive(0, 'robot', {})).toBeNull();
  });

  it('tracks per-recipient totals independently', () => {
    const gifts = { robot: 3, mommy: 1 };
    const result = processGive(5, 'mommy', gifts);
    expect(result.newTotalGifts.robot).toBe(3);
    expect(result.newTotalGifts.mommy).toBe(2);
  });

  it('result is frozen', () => {
    const result = processGive(5, 'robot', {});
    expect(Object.isFrozen(result)).toBe(true);
    expect(Object.isFrozen(result.newTotalGifts)).toBe(true);
  });
});
