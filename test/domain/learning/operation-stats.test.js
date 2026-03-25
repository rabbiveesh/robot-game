import { describe, it, expect } from 'vitest';
import { createOperationStats, recordOperation } from '../../../src/domain/learning/operation-stats.js';

describe('operation-stats', () => {
  it('creates stats with all coarse and fine-grained entries at zero', () => {
    const stats = createOperationStats();
    expect(stats.add.attempts).toBe(0);
    expect(stats.add_carry.attempts).toBe(0);
    expect(stats.sub_borrow.attempts).toBe(0);
    expect(stats.mul_hard.attempts).toBe(0);
    expect(stats.div_easy.attempts).toBe(0);
    expect(stats.bond_small.attempts).toBe(0);
    expect(Object.isFrozen(stats)).toBe(true);
  });

  it('records to both coarse and fine-grained', () => {
    let stats = createOperationStats();
    stats = recordOperation(stats, 'add', true, 'add_carry');
    expect(stats.add.correct).toBe(1);
    expect(stats.add.attempts).toBe(1);
    expect(stats.add_carry.correct).toBe(1);
    expect(stats.add_carry.attempts).toBe(1);
  });

  it('records coarse even without subSkill', () => {
    let stats = createOperationStats();
    stats = recordOperation(stats, 'sub', false);
    expect(stats.sub.correct).toBe(0);
    expect(stats.sub.attempts).toBe(1);
  });

  it('ignores unknown subSkill gracefully', () => {
    let stats = createOperationStats();
    stats = recordOperation(stats, 'add', true, 'add_nonexistent');
    expect(stats.add.correct).toBe(1);
    expect(stats.add.attempts).toBe(1);
    // No crash, unknown sub-skill is silently ignored
  });

  it('tracks multiple sub-skills independently', () => {
    let stats = createOperationStats();
    stats = recordOperation(stats, 'add', true, 'add_single');
    stats = recordOperation(stats, 'add', true, 'add_single');
    stats = recordOperation(stats, 'add', false, 'add_carry');
    stats = recordOperation(stats, 'add', true, 'add_carry');
    expect(stats.add.correct).toBe(3);
    expect(stats.add.attempts).toBe(4);
    expect(stats.add_single.correct).toBe(2);
    expect(stats.add_single.attempts).toBe(2);
    expect(stats.add_carry.correct).toBe(1);
    expect(stats.add_carry.attempts).toBe(2);
  });

  it('returns frozen objects', () => {
    let stats = createOperationStats();
    stats = recordOperation(stats, 'add', true, 'add_carry');
    expect(Object.isFrozen(stats)).toBe(true);
    expect(Object.isFrozen(stats.add)).toBe(true);
    expect(Object.isFrozen(stats.add_carry)).toBe(true);
  });
});
