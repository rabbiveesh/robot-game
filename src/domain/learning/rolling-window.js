// rolling-window.js — Immutable sliding window for tracking recent attempts

export function createWindow(entries = [], maxSize = 20) {
  return Object.freeze({
    entries: Object.freeze(entries.slice(-maxSize)),
    maxSize,
  });
}

export function pushEntry(window, entry) {
  const entries = [...window.entries, entry].slice(-window.maxSize);
  return createWindow(entries, window.maxSize);
}

export function accuracy(window) {
  if (window.entries.length === 0) return 0;
  const correct = window.entries.filter(e => e.correct).length;
  return correct / window.entries.length;
}

export function avgResponseTime(window) {
  const times = window.entries.filter(e => e.responseTimeMs != null).map(e => e.responseTimeMs);
  if (times.length === 0) return 0;
  return times.reduce((sum, t) => sum + t, 0) / times.length;
}

export function consecutiveWrong(window) {
  let count = 0;
  for (let i = window.entries.length - 1; i >= 0; i--) {
    if (!window.entries[i].correct) count++;
    else break;
  }
  return count;
}

export function operationAccuracy(window, operation) {
  const ops = window.entries.filter(e => e.operation === operation);
  if (ops.length === 0) return null;
  const correct = ops.filter(e => e.correct).length;
  return correct / ops.length;
}
