import { describe, it, expect } from 'vitest';
import { determineReward } from '../../../src/domain/economy/rewards.js';

describe('determineReward', () => {
  it('correct challenge returns dum_dum reward', () => {
    const r = determineReward('challenge', true);
    expect(r).toEqual({ type: 'dum_dum', amount: 1 });
  });

  it('wrong challenge returns null', () => {
    expect(determineReward('challenge', false)).toBeNull();
  });

  it('correct chest returns dum_dum reward', () => {
    expect(determineReward('chest', true)).toEqual({ type: 'dum_dum', amount: 1 });
  });

  it('reward object is frozen', () => {
    expect(Object.isFrozen(determineReward('challenge', true))).toBe(true);
  });
});
