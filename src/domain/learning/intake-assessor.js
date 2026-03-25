// intake-assessor.js — Intake quiz logic (stateless)

import { generateChallenge } from './challenge-generator.js';
import { createProfile } from './learner-profile.js';

export function generateIntakeQuestion(currentBand, questionIndex, rng) {
  // Create a temporary profile at the given band to generate a challenge
  const tempProfile = createProfile({ mathBand: currentBand });
  return generateChallenge(tempProfile, rng);
}

export function processIntakeResults(answers, configuredBand = null) {
  // answers: [{ band, correct, responseTimeMs, skippedText }]
  // configuredBand: the starting band the parent picked on the title screen (if any)
  //
  // Determine final band:
  //   Final band = last correct band (min 1)
  //   If parent configured a starting band, clamp placement to configuredBand + 2
  //   (don't wildly exceed what the parent thinks their kid can do)

  let lastCorrectBand = 1;
  for (const answer of answers) {
    if (answer.correct) {
      lastCorrectBand = answer.band;
    }
  }
  let mathBand = Math.max(1, Math.min(10, lastCorrectBand));

  // Respect parent's configured band as an anchor — don't exceed it by more than 2
  if (configuredBand != null && configuredBand >= 1) {
    mathBand = Math.min(mathBand, configuredBand + 2);
  }

  // Analyze response times
  const times = answers
    .filter(a => a.responseTimeMs != null)
    .map(a => a.responseTimeMs);
  const avgTime = times.length > 0
    ? times.reduce((s, t) => s + t, 0) / times.length
    : 5000;

  // Pace: fast (<3s) -> higher, slow (>8s) -> lower
  let pace = 0.5;
  if (avgTime < 3000) pace = 0.7;
  else if (avgTime < 5000) pace = 0.6;
  else if (avgTime > 8000) pace = 0.3;
  else if (avgTime > 6000) pace = 0.4;

  // Scaffolding: inverse of pace roughly
  let scaffolding = 0.5;
  if (avgTime > 8000) scaffolding = 0.7;
  else if (avgTime > 6000) scaffolding = 0.6;
  else if (avgTime < 3000) scaffolding = 0.3;

  // Promotion thresholds: confident fast kids promote sooner (lower thresholds)
  let promoteThreshold = 0.75;
  let stretchThreshold = 0.60;
  if (avgTime < 3000 && answers.filter(a => a.correct).length >= 3) {
    promoteThreshold = 0.65;
    stretchThreshold = 0.50;
  }

  // Text speed: check for text skipping
  const skippedCount = answers.filter(a => a.skippedText).length;
  let textSpeed = 0.035;
  if (skippedCount >= 2) textSpeed = 0.02;
  else if (skippedCount >= 1) textSpeed = 0.025;

  return {
    mathBand,
    pace,
    scaffolding,
    promoteThreshold,
    stretchThreshold,
    textSpeed,
  };
}

// Calculate the next band for the next intake question
// ceiling: optional max band (e.g. configuredBand + 2) to avoid throwing
// multiplication at a kid whose parent said "Add <5"
export function nextIntakeBand(currentBand, correct, ceiling = 10) {
  if (correct) return Math.min(ceiling, currentBand + 2);
  return Math.max(1, currentBand - 1);
}
