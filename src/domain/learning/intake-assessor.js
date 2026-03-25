// intake-assessor.js — Intake quiz logic (stateless)

import { generateChallenge } from './challenge-generator.js';
import { createProfile } from './learner-profile.js';

export function generateIntakeQuestion(currentBand, questionIndex, rng) {
  // Create a temporary profile at the given band to generate a challenge
  const tempProfile = createProfile({ mathBand: currentBand });
  return generateChallenge(tempProfile, rng);
}

export function processIntakeResults(answers) {
  // answers: [{ band, correct, responseTimeMs, skippedText }]
  // Determine final band:
  //   Start at band 3
  //   Correct -> next question band +2
  //   Wrong -> next question band -1
  //   Final band = last correct band (min 1)

  let lastCorrectBand = 1;
  for (const answer of answers) {
    if (answer.correct) {
      lastCorrectBand = answer.band;
    }
  }
  const mathBand = Math.max(1, Math.min(10, lastCorrectBand));

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

  // Streak to promote: confident kids can promote faster
  let streakToPromote = 3;
  if (avgTime < 3000 && answers.filter(a => a.correct).length >= 3) {
    streakToPromote = 2;
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
    streakToPromote,
    textSpeed,
  };
}

// Calculate the next band for the next intake question
export function nextIntakeBand(currentBand, correct) {
  if (correct) return Math.min(10, currentBand + 2);
  return Math.max(1, currentBand - 1);
}
