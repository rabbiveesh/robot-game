import { describe, it, expect } from 'vitest';
import { getInteractionOptions } from '../../../src/domain/economy/interaction-options.js';

describe('getInteractionOptions', () => {
  it('always includes talk option', () => {
    const opts = getInteractionOptions({ id: 'robot' }, { dumDums: 0 });
    expect(opts[0]).toEqual({ type: 'talk', label: 'Talk', key: '1' });
  });

  it('includes give when dumDums > 0', () => {
    const opts = getInteractionOptions({ id: 'robot' }, { dumDums: 3 });
    expect(opts.some(o => o.type === 'give')).toBe(true);
  });

  it('excludes give when dumDums === 0', () => {
    const opts = getInteractionOptions({ id: 'robot' }, { dumDums: 0 });
    expect(opts.some(o => o.type === 'give')).toBe(false);
  });

  it('excludes give when npc.canReceiveGifts is false', () => {
    const opts = getInteractionOptions({ id: 'chest', canReceiveGifts: false }, { dumDums: 5 });
    expect(opts.some(o => o.type === 'give')).toBe(false);
  });

  it('includes shop when npc.hasShop is true', () => {
    const opts = getInteractionOptions({ id: 'bolt', hasShop: true }, { dumDums: 0 });
    expect(opts.some(o => o.type === 'shop')).toBe(true);
  });

  it('options are frozen', () => {
    const opts = getInteractionOptions({ id: 'robot' }, { dumDums: 3 });
    expect(Object.isFrozen(opts)).toBe(true);
    expect(Object.isFrozen(opts[0])).toBe(true);
  });

  it('options have key assignments (1, 2, 3)', () => {
    const opts = getInteractionOptions({ id: 'bolt', hasShop: true }, { dumDums: 3 });
    expect(opts[0].key).toBe('1');
    expect(opts[1].key).toBe('2');
    expect(opts[2].key).toBe('3');
  });
});
