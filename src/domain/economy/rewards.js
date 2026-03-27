// rewards.js — Determine rewards for interactions

export function determineReward(interactionType, correct) {
  if (!correct) return null;
  return Object.freeze({ type: 'dum_dum', amount: 1 });
}
