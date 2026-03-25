// frustration-detector.js — Analyzes rolling window + behavioral signals

import { accuracy, consecutiveWrong } from './rolling-window.js';

export function detectFrustration(window, recentBehaviors = []) {
  // HIGH: 3+ consecutive wrong on same band
  const consWrong = consecutiveWrong(window);
  if (consWrong >= 3) {
    return { level: 'high', recommendation: 'drop_band' };
  }

  // HIGH: rapid_clicking behavior in last 3 events
  const recentRapid = recentBehaviors
    .slice(-3)
    .filter(b => b.signal === 'rapid_clicking');
  if (recentRapid.length > 0) {
    return { level: 'high', recommendation: 'drop_band' };
  }

  // HIGH: accuracy < 40% in rolling window (need enough data)
  if (window.entries.length >= 5 && accuracy(window) < 0.4) {
    return { level: 'high', recommendation: 'switch_to_chat' };
  }

  // MILD: long idle (>15s) after wrong answer
  const lastEntry = window.entries[window.entries.length - 1];
  if (lastEntry && !lastEntry.correct && lastEntry.responseTimeMs > 15000) {
    return { level: 'mild', recommendation: 'encourage' };
  }

  // MILD: chose easier path twice in a row
  const lastTwoBehaviors = recentBehaviors.slice(-2);
  if (
    lastTwoBehaviors.length === 2 &&
    lastTwoBehaviors.every(b => b.signal === 'chose_easier_path')
  ) {
    return { level: 'mild', recommendation: 'offer_easier_path' };
  }

  return { level: 'none', recommendation: 'continue' };
}
