// ═══════════════════════════════════════════════════════════
// APPROACH A: Mutable objects with methods (OOP-ish)
// ═══════════════════════════════════════════════════════════

class LearnerProfileMutable {
  constructor(data = {}) {
    this.mathBand = data.mathBand ?? 1;
    this.pace = data.pace ?? 0.5;
    this.scaffolding = data.scaffolding ?? 0.5;
    this.streakToPromote = data.streakToPromote ?? 3;
    this.wrongsBeforeTeach = data.wrongsBeforeTeach ?? 2;
    this.streak = data.streak ?? 0;
    this.rollingWindow = data.rollingWindow ?? [];
  }

  recordPuzzleAttempt(event) {
    this.rollingWindow.push(event);
    if (this.rollingWindow.length > 20) this.rollingWindow.shift();

    if (event.correct) {
      this.streak = Math.max(0, this.streak) + 1;
      if (this.streak >= this.streakToPromote && this.mathBand < 10) {
        this.mathBand++;
        this.streak = 0;
      }
    } else {
      this.streak = Math.min(0, this.streak) - 1;
      if (this.streak <= -2 && this.mathBand > 1) {
        this.mathBand--;
        this.streak = 0;
      }
    }
  }

  // Problem: what was the state BEFORE this call? Gone. Can't replay, can't undo,
  // can't diff. Test assertions have to check mutable state at specific moments.
}

// Test:
// const profile = new LearnerProfileMutable();
// profile.recordPuzzleAttempt({ correct: true, ... });
// profile.recordPuzzleAttempt({ correct: true, ... });
// expect(profile.streak).toBe(2);  // works, but we can't see the history


// ═══════════════════════════════════════════════════════════
// APPROACH B: Immutable state + reducer
// ═══════════════════════════════════════════════════════════

// ── State is a plain frozen object. Never mutated. ──

function createProfile(overrides = {}) {
  return Object.freeze({
    mathBand: 1,
    pace: 0.5,
    scaffolding: 0.5,
    streakToPromote: 3,
    wrongsBeforeTeach: 2,
    streak: 0,
    rollingWindow: [],
    ...overrides,
  });
}

// ── Events are plain objects describing what happened ──

// { type: 'PUZZLE_ATTEMPTED', correct: true, operation: 'add', band: 3, responseTimeMs: 2400 }
// { type: 'TEACHING_RETRY', correct: true, representationStyle: 'concrete' }
// { type: 'BEHAVIOR', signal: 'text_skipped' }

// ── Reducer: (state, event) → newState. Pure function. ──

function learnerReducer(state, event) {
  switch (event.type) {
    case 'PUZZLE_ATTEMPTED': {
      const window = [...state.rollingWindow, event].slice(-20);
      let { streak, mathBand } = state;

      if (event.correct) {
        streak = Math.max(0, streak) + 1;
        if (streak >= state.streakToPromote && mathBand < 10) {
          mathBand++;
          streak = 0;
        }
      } else {
        streak = Math.min(0, streak) - 1;
        if (streak <= -2 && mathBand > 1) {
          mathBand--;
          streak = 0;
        }
      }

      return Object.freeze({ ...state, streak, mathBand, rollingWindow: window });
    }

    case 'BEHAVIOR': {
      switch (event.signal) {
        case 'text_skipped':
          return Object.freeze({
            ...state,
            pace: Math.min(1, state.pace + 0.05),
            textSpeed: Math.max(0.015, state.textSpeed - 0.003),
          });
        case 'rapid_clicking':
          // Kid is mashing — don't speed up, flag frustration
          return Object.freeze({
            ...state,
            wrongsBeforeTeach: Math.max(1, state.wrongsBeforeTeach - 1),
          });
        default:
          return state;
      }
    }

    case 'FRUSTRATION_DETECTED': {
      if (event.level === 'high') {
        return Object.freeze({
          ...state,
          mathBand: Math.max(1, state.mathBand - 1),
          wrongsBeforeTeach: 1,
          pace: Math.max(0, state.pace - 0.15),
          scaffolding: Math.max(0, state.scaffolding - 0.2),
        });
      }
      return state;
    }

    case 'INTAKE_COMPLETED': {
      return Object.freeze({
        ...state,
        mathBand: event.placedBand,
        pace: event.pace,
        scaffolding: event.scaffolding,
        streakToPromote: event.streakToPromote,
        intakeCompleted: true,
      });
    }

    default:
      return state;
  }
}

// ═══════════════════════════════════════════════════════════
// WHY THE REDUCER IS BETTER FOR US
// ═══════════════════════════════════════════════════════════

// 1. TESTABILITY — every transition is independently testable:

function exampleTests() {
  // Test a single transition in isolation
  const before = createProfile({ mathBand: 3, streak: 2, streakToPromote: 3 });
  const after = learnerReducer(before, {
    type: 'PUZZLE_ATTEMPTED', correct: true, operation: 'add', band: 3, responseTimeMs: 2000,
  });

  console.assert(after.mathBand === 4, 'should promote after 3rd correct');
  console.assert(after.streak === 0, 'streak should reset after promotion');
  console.assert(before.mathBand === 3, 'original state is untouched');

  // Test a sequence — we can see every intermediate state
  const events = [
    { type: 'PUZZLE_ATTEMPTED', correct: false, operation: 'sub', band: 3, responseTimeMs: 8000 },
    { type: 'PUZZLE_ATTEMPTED', correct: false, operation: 'sub', band: 3, responseTimeMs: 9000 },
    { type: 'FRUSTRATION_DETECTED', level: 'high' },
  ];

  const history = [];
  let state = createProfile({ mathBand: 3 });
  for (const event of events) {
    state = learnerReducer(state, event);
    history.push({ event, state });
  }

  console.assert(state.mathBand === 1, 'demoted twice: 3→2 from streak, 2→1 from frustration');
  console.assert(state.wrongsBeforeTeach === 1, 'frustration reduced teaching threshold');

  // We can inspect any point in history:
  console.assert(history[1].state.mathBand === 2, 'after 2nd wrong, demoted to 2');
  console.assert(history[2].state.mathBand === 1, 'after frustration, demoted to 1');
}

// 2. REPLAY — we can reconstruct any state from events:

function replayProfile(events) {
  return events.reduce(learnerReducer, createProfile());
}

// 3. STEALTH ASSESSMENT AUDIT — we store the event log, not just final state.
//    A parent dashboard can show: "Here's what happened in today's session"
//    and we can replay it to understand WHY the profile looks the way it does.

// 4. SAVE/LOAD — state is already plain JSON. No serialization logic needed.
//    Save: JSON.stringify(state)
//    Load: createProfile(JSON.parse(saved))

// 5. TIME TRAVEL — for debugging / parent dashboard:
//    "What would have happened if we'd set streakToPromote to 2 instead of 3?"
//    Just replay the same events with different initial state.

// 6. COMPOSITION — multiple reducers for different concerns:

function frustrationReducer(state, event) {
  // Only handles frustration-related state changes
  if (event.type !== 'PUZZLE_ATTEMPTED') return state;
  // ... frustration-specific logic
  return state;
}

function craReducer(state, event) {
  // Only handles CRA progression
  if (event.type !== 'TEACHING_RETRY') return state;
  // ... CRA-specific logic
  return state;
}

// Compose them:
function rootReducer(state, event) {
  state = learnerReducer(state, event);
  state = frustrationReducer(state, event);
  state = craReducer(state, event);
  return state;
}

// Each sub-reducer is independently testable with its own test file.


// ═══════════════════════════════════════════════════════════
// THE TRADEOFF
// ═══════════════════════════════════════════════════════════

// Mutable:
//   + Less boilerplate (no spread, no freeze)
//   + Familiar OOP patterns
//   - Can't inspect history
//   - Harder to test sequences
//   - State can be mutated from anywhere (bugs)
//   - Serialization needs explicit toJSON/fromJSON

// Reducer:
//   + Event log IS the audit trail (stealth assessment!)
//   + Every transition independently testable
//   + State can never be corrupted by accident
//   + Replay, time-travel, "what if" analysis
//   + Save/load is trivial (it's already JSON)
//   + Composable — split complex logic into focused sub-reducers
//   - More verbose (spread everywhere)
//   - Need discipline to never mutate (Object.freeze helps)
//   - Slightly more indirection (events instead of method calls)

// For THIS project, reducer wins because:
// - Stealth assessment IS an event log. The reducer pattern makes it native.
// - Parent dashboard needs history and replay. Comes free.
// - The frustration detector needs to analyze sequences. Event log is the input.
// - Testability of the Learning domain is the #1 arch requirement.

export { createProfile, learnerReducer, replayProfile };
