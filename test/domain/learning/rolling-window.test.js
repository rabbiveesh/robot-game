import { describe, it, expect } from 'vitest';
import {
  createWindow, pushEntry, accuracy, avgResponseTime,
  consecutiveWrong, operationAccuracy,
} from '../../../src/domain/learning/rolling-window.js';

describe('rolling-window', () => {
  it('creates an empty window', () => {
    const w = createWindow();
    expect(w.entries).toEqual([]);
    expect(w.maxSize).toBe(20);
  });

  it('creates a window with initial entries', () => {
    const w = createWindow([{ correct: true }, { correct: false }]);
    expect(w.entries).toHaveLength(2);
  });

  it('truncates to maxSize on creation', () => {
    const entries = Array.from({ length: 25 }, (_, i) => ({ correct: true, id: i }));
    const w = createWindow(entries, 10);
    expect(w.entries).toHaveLength(10);
    expect(w.entries[0].id).toBe(15); // last 10
  });

  it('pushEntry appends and respects maxSize', () => {
    let w = createWindow([], 3);
    w = pushEntry(w, { correct: true, id: 1 });
    w = pushEntry(w, { correct: true, id: 2 });
    w = pushEntry(w, { correct: false, id: 3 });
    w = pushEntry(w, { correct: true, id: 4 });
    expect(w.entries).toHaveLength(3);
    expect(w.entries[0].id).toBe(2);
  });

  it('returns frozen objects', () => {
    const w = createWindow([{ correct: true }]);
    expect(Object.isFrozen(w)).toBe(true);
    expect(Object.isFrozen(w.entries)).toBe(true);
  });

  it('accuracy returns 0 for empty window', () => {
    expect(accuracy(createWindow())).toBe(0);
  });

  it('accuracy calculates correctly', () => {
    const w = createWindow([
      { correct: true }, { correct: true }, { correct: false },
      { correct: true }, { correct: false },
    ]);
    expect(accuracy(w)).toBeCloseTo(0.6);
  });

  it('avgResponseTime returns 0 for empty window', () => {
    expect(avgResponseTime(createWindow())).toBe(0);
  });

  it('avgResponseTime calculates correctly', () => {
    const w = createWindow([
      { responseTimeMs: 1000 }, { responseTimeMs: 3000 }, { responseTimeMs: 2000 },
    ]);
    expect(avgResponseTime(w)).toBe(2000);
  });

  it('avgResponseTime ignores entries without responseTimeMs', () => {
    const w = createWindow([
      { responseTimeMs: 1000 }, { correct: true }, { responseTimeMs: 3000 },
    ]);
    expect(avgResponseTime(w)).toBe(2000);
  });

  it('consecutiveWrong counts from end', () => {
    const w = createWindow([
      { correct: true }, { correct: false }, { correct: false }, { correct: false },
    ]);
    expect(consecutiveWrong(w)).toBe(3);
  });

  it('consecutiveWrong returns 0 when last is correct', () => {
    const w = createWindow([
      { correct: false }, { correct: false }, { correct: true },
    ]);
    expect(consecutiveWrong(w)).toBe(0);
  });

  it('operationAccuracy filters by operation', () => {
    const w = createWindow([
      { correct: true, operation: 'add' },
      { correct: false, operation: 'sub' },
      { correct: true, operation: 'add' },
      { correct: false, operation: 'add' },
    ]);
    expect(operationAccuracy(w, 'add')).toBeCloseTo(2 / 3);
    expect(operationAccuracy(w, 'sub')).toBe(0);
  });

  it('operationAccuracy returns null for unknown operation', () => {
    const w = createWindow([{ correct: true, operation: 'add' }]);
    expect(operationAccuracy(w, 'divide')).toBeNull();
  });
});
