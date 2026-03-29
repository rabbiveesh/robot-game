// domain-bridge.js — Thin wrapper around WASM domain exports.
// JSON in/out. The adapter doesn't know it's calling WASM.
// Falls back to JS domain if WASM isn't loaded yet.

let wasmModule = null;
let wasmReady = false;

export async function initWasm() {
  try {
    wasmModule = await import('../dist/wasm/robot_buddy_domain.js');
    await wasmModule.default();
    wasmReady = true;
    console.log('[WASM] Domain loaded');
  } catch (e) {
    console.warn('[WASM] Failed to load, falling back to JS domain:', e.message);
    wasmReady = false;
  }
}

export function isWasmReady() {
  return wasmReady;
}

// ─── Rolling Window (WASM) ──────────────────────────────

export function createRollingWindow(maxSize = 20) {
  if (!wasmReady) return null;
  return JSON.parse(wasmModule.create_rolling_window(maxSize));
}

export function pushWindowEntry(window, entry) {
  if (!wasmReady) return null;
  return JSON.parse(wasmModule.push_window_entry(
    JSON.stringify(window),
    JSON.stringify(entry),
  ));
}

export function windowAccuracy(window) {
  if (!wasmReady) return null;
  return wasmModule.window_accuracy(JSON.stringify(window));
}

// ─── Operation Stats (WASM) ─────────────────────────────

export function createOperationStats() {
  if (!wasmReady) return null;
  return JSON.parse(wasmModule.create_operation_stats());
}

export function recordOperation(stats, operation, correct, subSkill) {
  if (!wasmReady) return null;
  return JSON.parse(wasmModule.record_operation(
    JSON.stringify(stats),
    operation,
    correct,
    subSkill || '',
  ));
}
