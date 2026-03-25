// learner-profile.js — State shape and root reducer

import { createWindow, pushEntry, accuracyAtBand, accuracyAboveBand, accuracy } from './rolling-window.js';
import { createOperationStats, recordOperation } from './operation-stats.js';

export function createProfile(overrides = {}) {
  return Object.freeze({
    mathBand: 1,
    streak: 0,                // display only — Sparky celebrates streaks, no mechanical effect
    pace: 0.5,
    scaffolding: 0.5,
    challengeFreq: 0.5,
    spreadWidth: 0.5,         // distribution width (0 = tight, 1 = wide)
    promoteThreshold: 0.75,   // accuracy at center band needed to promote
    stretchThreshold: 0.60,   // accuracy at above-center needed to promote
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
    textSkipCount: 0,
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
  const prev1 = entries[entries.length - 1];
  const prev2 = entries[entries.length - 2];
  return prev1.correct && prev2.correct;
}

// Check if accuracy-based promotion should fire.
// NOTE: accuracyAtBand filters by e.band (the sampledBand), not e.centerBand.
// After a promotion, the new center band will have very few entries because
// most prior entries were sampled at the OLD center. This is intentional —
// the kid must accumulate fresh evidence at the new band before promoting again.
function shouldPromote(window, centerBand, promoteThreshold, stretchThreshold) {
  const atCenter = accuracyAtBand(window, centerBand);
  const aboveCenter = accuracyAboveBand(window, centerBand);

  // Need at least 4 attempts at center band
  if (atCenter.count < 4) return false;
  if (atCenter.accuracy < promoteThreshold) return false;

  // Need at least 2 attempts above center with decent accuracy
  // If no above-center attempts exist yet (tight spread), skip this check
  if (aboveCenter.count >= 2 && aboveCenter.accuracy < stretchThreshold) return false;

  return true;
}

// Check if accuracy-based demotion should fire
function shouldDemote(window, centerBand) {
  const atCenter = accuracyAtBand(window, centerBand);
  // Need at least 4 attempts at center band
  if (atCenter.count < 4) return false;
  return atCenter.accuracy < 0.5;
}

export function learnerReducer(state, event) {
  switch (event.type) {
    case 'PUZZLE_ATTEMPTED': {
      const boredom = isBoredomWrong(state.rollingWindow, event);

      const windowEntry = Object.freeze({
        correct: event.correct,
        operation: event.operation,
        band: event.band,
        centerBand: event.centerBand ?? event.band,
        responseTimeMs: event.responseTimeMs,
        boredom,
        timestamp: event.timestamp,
      });
      const newWindow = pushEntry(state.rollingWindow, windowEntry);
      const newStats = recordOperation(state.operationStats, event.operation, event.correct);

      // Streak is display-only — update it for UI but it doesn't drive promotion
      let newStreak = state.streak;
      if (boredom) {
        newStreak = Math.max(0, state.streak);
      } else if (event.correct) {
        newStreak = Math.max(0, state.streak) + 1;
      } else {
        newStreak = Math.min(0, state.streak) - 1;
      }

      // Accuracy-based promotion and demotion
      let newBand = state.mathBand;
      let newSpreadWidth = state.spreadWidth;

      if (!boredom) {
        if (shouldPromote(newWindow, state.mathBand, state.promoteThreshold, state.stretchThreshold)) {
          newBand = Math.min(10, state.mathBand + 1);
          newStreak = 0;
          // Tighten spread briefly after promotion — let kid adjust to new center
          newSpreadWidth = Math.max(0.2, state.spreadWidth - 0.1);
        } else if (shouldDemote(newWindow, state.mathBand)) {
          newBand = Math.max(1, state.mathBand - 1);
          newStreak = 0;
          // Tighten spread on demotion
          newSpreadWidth = Math.max(0.1, state.spreadWidth - 0.15);
        }
      }

      // Widen spread on sustained good performance
      if (newBand === state.mathBand) { // no promotion/demotion this tick
        const rollingAcc = accuracy(newWindow);
        if (newWindow.entries.length >= 10 && rollingAcc > 0.75 && newSpreadWidth < 0.8) {
          newSpreadWidth = Math.min(1.0, newSpreadWidth + 0.05);
        }
      }

      return Object.freeze({
        ...state,
        streak: newStreak,
        mathBand: newBand,
        spreadWidth: newSpreadWidth,
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
            textSkipCount: state.textSkipCount + 1,
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
          spreadWidth: Math.max(0.1, state.spreadWidth - 0.15),
        });
      }
      return state;
    }

    case 'INTAKE_COMPLETED': {
      return Object.freeze({
        ...state,
        mathBand: event.mathBand ?? state.mathBand,
        pace: event.pace ?? state.pace,
        scaffolding: event.scaffolding ?? state.scaffolding,
        promoteThreshold: event.promoteThreshold ?? state.promoteThreshold,
        stretchThreshold: event.stretchThreshold ?? state.stretchThreshold,
        textSpeed: event.textSpeed ?? state.textSpeed,
        intakeCompleted: true,
      });
    }

    default:
      return state;
  }
}
