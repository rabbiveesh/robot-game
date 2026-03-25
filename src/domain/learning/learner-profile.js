// learner-profile.js — State shape and root reducer

import { createWindow, pushEntry } from './rolling-window.js';
import { createOperationStats, recordOperation } from './operation-stats.js';

export function createProfile(overrides = {}) {
  return Object.freeze({
    mathBand: 1,
    streak: 0,
    pace: 0.5,
    scaffolding: 0.5,
    challengeFreq: 0.5,
    streakToPromote: 3,
    wrongsBeforeTeach: 2,
    hintVisibility: 0.5,
    textSpeed: 0.035,
    framingStyle: 0.5,
    representationStyle: 0.5,
    craStages: Object.freeze({
      add: 'concrete',
      sub: 'concrete',
      multiply: 'concrete',
      divide: 'concrete',
      number_bond: 'concrete',
    }),
    intakeCompleted: false,
    rollingWindow: createWindow(),
    operationStats: createOperationStats(),
    ...overrides,
  });
}

// Detect boredom pattern: correct-correct-wrong(fast)-correct
// A fast wrong sandwiched between corrects is not a real failure
function isBoredomWrong(window, entry) {
  if (entry.correct) return false;
  if (entry.responseTimeMs == null || entry.responseTimeMs > 2000) return false;
  const entries = window.entries;
  if (entries.length < 2) return false;
  // Last two entries were correct
  const prev1 = entries[entries.length - 1];
  const prev2 = entries[entries.length - 2];
  return prev1.correct && prev2.correct;
}

export function learnerReducer(state, event) {
  switch (event.type) {
    case 'PUZZLE_ATTEMPTED': {
      const boredom = isBoredomWrong(state.rollingWindow, event);

      const windowEntry = Object.freeze({
        correct: event.correct,
        operation: event.operation,
        band: event.band,
        responseTimeMs: event.responseTimeMs,
        boredom,
        timestamp: event.timestamp,
      });
      const newWindow = pushEntry(state.rollingWindow, windowEntry);
      const newStats = recordOperation(state.operationStats, event.operation, event.correct);

      let newStreak = state.streak;
      let newBand = state.mathBand;

      if (boredom) {
        // Don't count boredom wrongs against the kid
        // Reset streak to 0 rather than going negative
        newStreak = Math.max(0, state.streak);
      } else if (event.correct) {
        newStreak = Math.max(0, state.streak) + 1;
        if (newStreak >= state.streakToPromote) {
          newBand = Math.min(10, state.mathBand + 1);
          newStreak = 0;
        }
      } else {
        newStreak = Math.min(0, state.streak) - 1;
        if (newStreak <= -2) {
          newBand = Math.max(1, state.mathBand - 1);
          newStreak = 0;
        }
      }

      return Object.freeze({
        ...state,
        streak: newStreak,
        mathBand: newBand,
        rollingWindow: newWindow,
        operationStats: newStats,
      });
    }

    case 'TEACHING_RETRY': {
      // After teaching mode, the kid retries — don't double-penalize.
      // TODO: Track which CRA representation (concrete/representational/abstract)
      // was shown and whether the retry succeeded. This is how we advance CRA
      // stages per operation — e.g., if concrete dots consistently lead to correct
      // retries for addition, promote add to 'representational'. This is the
      // core feedback loop for CRA progression and the whole point of the dial.
      return state;
    }

    case 'BEHAVIOR': {
      switch (event.signal) {
        case 'text_skipped':
          return Object.freeze({
            ...state,
            pace: Math.min(1, state.pace + 0.1),
            textSpeed: Math.max(0.01, state.textSpeed - 0.005),
          });
        case 'rapid_clicking':
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
          pace: Math.max(0, state.pace - 0.2),
          streak: 0,
        });
      }
      // mild frustration — just encourage, no dial changes
      return state;
    }

    case 'INTAKE_COMPLETED': {
      return Object.freeze({
        ...state,
        mathBand: event.mathBand ?? state.mathBand,
        pace: event.pace ?? state.pace,
        scaffolding: event.scaffolding ?? state.scaffolding,
        streakToPromote: event.streakToPromote ?? state.streakToPromote,
        textSpeed: event.textSpeed ?? state.textSpeed,
        intakeCompleted: true,
      });
    }

    default:
      return state;
  }
}
