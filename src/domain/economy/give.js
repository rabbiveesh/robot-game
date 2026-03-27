// give.js — Dum Dum gift mechanic

const MILESTONES = [
  { count: 1,  reaction: 'first' },
  { count: 5,  reaction: 'spin' },
  { count: 10, reaction: 'accessory' },
  { count: 20, reaction: 'color_change' },
  { count: 50, reaction: 'ultimate' },
];

function checkMilestone(recipientId, total) {
  for (let i = MILESTONES.length - 1; i >= 0; i--) {
    if (total === MILESTONES[i].count) {
      return Object.freeze({ recipientId, total, reaction: MILESTONES[i].reaction });
    }
  }
  return null;
}

export function canGive(dumDums) {
  return dumDums > 0;
}

export function processGive(dumDums, recipientId, totalGiftsGiven) {
  if (dumDums <= 0) return null;

  const newTotal = (totalGiftsGiven[recipientId] || 0) + 1;
  const milestone = checkMilestone(recipientId, newTotal);

  return Object.freeze({
    newDumDums: dumDums - 1,
    newTotalGifts: Object.freeze({ ...totalGiftsGiven, [recipientId]: newTotal }),
    milestone,
  });
}
