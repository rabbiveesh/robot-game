// operation-stats.js — Immutable per-operation statistics

export function createOperationStats() {
  return Object.freeze({
    add: Object.freeze({ correct: 0, attempts: 0 }),
    sub: Object.freeze({ correct: 0, attempts: 0 }),
    multiply: Object.freeze({ correct: 0, attempts: 0 }),
    divide: Object.freeze({ correct: 0, attempts: 0 }),
    number_bond: Object.freeze({ correct: 0, attempts: 0 }),
  });
}

export function recordOperation(stats, operation, correct) {
  const current = stats[operation];
  if (!current) return stats;
  const updated = Object.freeze({
    correct: current.correct + (correct ? 1 : 0),
    attempts: current.attempts + 1,
  });
  return Object.freeze({ ...stats, [operation]: updated });
}
