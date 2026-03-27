// interaction-options.js — Determine available options for NPC interaction

export function getInteractionOptions(npc, playerState) {
  const options = [];

  options.push(Object.freeze({ type: 'talk', label: 'Talk', key: '1' }));

  if (npc.canReceiveGifts !== false && playerState.dumDums > 0) {
    options.push(Object.freeze({ type: 'give', label: 'Give Dum Dum', key: '2' }));
  }

  if (npc.hasShop) {
    options.push(Object.freeze({ type: 'shop', label: 'Buy', key: String(options.length + 1) }));
  }

  return Object.freeze(options);
}
