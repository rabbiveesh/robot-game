// operation-stats.js — Immutable per-operation statistics (coarse + fine-grained)

const STAT_ZERO = Object.freeze({ correct: 0, attempts: 0 });

export function createOperationStats() {
  return Object.freeze({
    // Coarse (backward compat, CRA tracking)
    add: STAT_ZERO,
    sub: STAT_ZERO,
    multiply: STAT_ZERO,
    divide: STAT_ZERO,
    number_bond: STAT_ZERO,

    // Fine-grained sub-skills
    add_single: STAT_ZERO,
    add_no_carry: STAT_ZERO,
    add_carry: STAT_ZERO,
    add_carry_tens: STAT_ZERO,
    sub_single: STAT_ZERO,
    sub_no_borrow: STAT_ZERO,
    sub_borrow: STAT_ZERO,
    sub_borrow_tens: STAT_ZERO,
    mul_trivial: STAT_ZERO,
    mul_easy: STAT_ZERO,
    mul_hard: STAT_ZERO,
    div_easy: STAT_ZERO,
    div_hard: STAT_ZERO,
    bond_small: STAT_ZERO,
    bond_large: STAT_ZERO,
  });
}

function bumpStat(stats, key, correct) {
  const current = stats[key];
  if (!current) return stats;
  return Object.freeze({
    ...stats,
    [key]: Object.freeze({
      correct: current.correct + (correct ? 1 : 0),
      attempts: current.attempts + 1,
    }),
  });
}

export function recordOperation(stats, operation, correct, subSkill) {
  // Record coarse operation
  let updated = bumpStat(stats, operation, correct);
  // Record fine-grained sub-skill (if provided and known)
  if (subSkill && stats[subSkill] !== undefined) {
    updated = bumpStat(updated, subSkill, correct);
  }
  return updated;
}
