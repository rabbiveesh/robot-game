// challenge-generator.js — Pure challenge generation from profile + rng

// ─── BAND DISTRIBUTION ──────────────────────────────────

// Produce a probability map { [band]: probability } for bands 1-10
// spreadWidth: 0.0 = tight (90% at center), 1.0 = wide (30% at center)
export function bandDistribution(centerBand, spreadWidth) {
  const sw = Math.max(0, Math.min(1, spreadWidth));

  // Interpolate center weight: 90% at sw=0, 50% at sw=0.5, 30% at sw=1.0
  const centerWeight = 0.9 - 0.6 * sw;

  // Build raw weights for offsets from center
  const offsets = [
    { d: 0, base: centerWeight },
    { d: 1, base: 0.05 + 0.15 * sw },   // ±1
    { d: 2, base: Math.max(0, 0.1 * sw - 0.005) },   // ±2
    { d: 3, base: Math.max(0, 0.05 * (sw - 0.5) * 2) },  // ±3 only at wide
  ];

  const raw = {};
  for (let band = 1; band <= 10; band++) raw[band] = 0;

  for (const { d, base } of offsets) {
    if (d === 0) {
      raw[centerBand] += base;
    } else {
      const hi = centerBand + d;
      const lo = centerBand - d;
      const perSide = base / 2;
      if (hi >= 1 && hi <= 10) raw[hi] += perSide;
      else raw[Math.min(10, centerBand + Math.max(0, d - 1)) || centerBand] += perSide; // redistribute
      if (lo >= 1 && lo <= 10) raw[lo] += perSide;
      else raw[Math.max(1, centerBand - Math.max(0, d - 1)) || centerBand] += perSide; // redistribute
    }
  }

  // Normalize to sum to 1.0
  const total = Object.values(raw).reduce((s, v) => s + v, 0);
  const dist = {};
  for (let band = 1; band <= 10; band++) {
    dist[band] = total > 0 ? raw[band] / total : (band === centerBand ? 1 : 0);
  }
  return dist;
}

// Sample a band from a probability distribution using the given rng
export function sampleFromDistribution(dist, rng) {
  const r = rng();
  let cumulative = 0;
  for (let band = 1; band <= 10; band++) {
    cumulative += dist[band] || 0;
    if (r < cumulative) return band;
  }
  return 10; // floating point safety
}

// ─── OPERATIONS ─────────────────────────────────────────

// Map operations to their internal name
const OPERATIONS = ['add', 'sub', 'multiply', 'divide', 'number_bond'];

// Which operations are available at each band
const BAND_OPERATIONS = {
  1: ['add'],
  2: ['add', 'sub'],
  3: ['add', 'sub', 'number_bond'],
  4: ['add', 'sub', 'number_bond'],
  5: ['multiply'],
  6: ['add', 'sub'],
  7: ['add', 'sub'],
  8: ['multiply'],
  9: ['multiply'],
  10: ['divide'],
};

function pickOperation(profile, sampledBand, rng) {
  const available = BAND_OPERATIONS[sampledBand] || ['add'];
  if (available.length === 1) return available[0];

  // 60% chance of strength (highest accuracy), 40% growth area (lowest accuracy)
  const stats = profile.operationStats;
  const withAccuracy = available.map(op => {
    const s = stats[op];
    const acc = s && s.attempts > 0 ? s.correct / s.attempts : 0.5;
    return { op, acc };
  });
  withAccuracy.sort((a, b) => b.acc - a.acc);

  const strengths = withAccuracy.slice(0, Math.ceil(withAccuracy.length / 2));
  const growth = withAccuracy.slice(Math.ceil(withAccuracy.length / 2));

  if (growth.length > 0 && rng() < 0.4) {
    return growth[Math.floor(rng() * growth.length)].op;
  }
  return strengths[Math.floor(rng() * strengths.length)].op;
}

function generateNumbers(band, operation, rng) {
  let a, b, answer, question, op;

  switch (band) {
    case 1: // Addition within 5
      a = Math.floor(rng() * 4) + 1;
      b = Math.floor(rng() * (5 - a)) + 1;
      answer = a + b;
      op = '+';
      question = `What is ${a} + ${b}?`;
      break;

    case 2: {
      const doSub = operation === 'sub' || (operation !== 'add' && rng() < 0.3);
      if (doSub) {
        a = Math.floor(rng() * 7) + 3;
        b = Math.floor(rng() * (a - 1)) + 1;
        answer = a - b;
        op = '-';
        question = `What is ${a} - ${b}?`;
      } else {
        a = Math.floor(rng() * 7) + 1;
        b = Math.floor(rng() * (10 - a)) + 1;
        answer = a + b;
        op = '+';
        question = `What is ${a} + ${b}?`;
      }
      break;
    }

    case 3: {
      if (operation === 'number_bond' || (operation !== 'add' && operation !== 'sub' && rng() < 0.25)) {
        const total = Math.floor(rng() * 10) + 5;
        b = Math.floor(rng() * (total - 1)) + 1;
        answer = total - b;
        a = total;
        op = '+';
        question = `What + ${b} = ${total}?`;
      } else if (operation === 'sub' || rng() < 0.4) {
        a = Math.floor(rng() * 10) + 5;
        b = Math.floor(rng() * Math.min(a - 1, 8)) + 1;
        answer = a - b;
        op = '-';
        question = `What is ${a} - ${b}?`;
      } else {
        a = Math.floor(rng() * 10) + 2;
        b = Math.floor(rng() * (15 - a)) + 1;
        answer = a + b;
        op = '+';
        question = `What is ${a} + ${b}?`;
      }
      break;
    }

    case 4: {
      if (operation === 'number_bond' || (operation !== 'add' && operation !== 'sub' && rng() < 0.2)) {
        const total = Math.floor(rng() * 10) + 10;
        b = Math.floor(rng() * (total - 2)) + 1;
        answer = total - b;
        a = total;
        op = '+';
        question = `What + ${b} = ${total}?`;
      } else if (operation === 'sub' || rng() < 0.45) {
        a = Math.floor(rng() * 12) + 8;
        b = Math.floor(rng() * Math.min(a - 1, 10)) + 1;
        answer = a - b;
        op = '-';
        question = `What is ${a} - ${b}?`;
      } else {
        a = Math.floor(rng() * 14) + 2;
        b = Math.floor(rng() * (20 - a)) + 1;
        answer = a + b;
        op = '+';
        question = `What is ${a} + ${b}?`;
      }
      break;
    }

    case 5: {
      const multiplier = rng() < 0.4 ? 1 : 2;
      b = Math.floor(rng() * 10) + 1;
      a = multiplier;
      answer = a * b;
      op = '\u00d7';
      question = `What is ${a} \u00d7 ${b}?`;
      break;
    }

    case 6: {
      const doSub6 = operation === 'sub' || (operation !== 'add' && rng() < 0.45);
      if (doSub6) {
        a = Math.floor(rng() * 30) + 20;
        b = Math.floor(rng() * (a - 5)) + 5;
        answer = a - b;
        op = '-';
        question = `What is ${a} - ${b}?`;
      } else {
        a = Math.floor(rng() * 35) + 5;
        b = Math.floor(rng() * (50 - a - 1)) + 1;
        answer = a + b;
        op = '+';
        question = `What is ${a} + ${b}?`;
      }
      break;
    }

    case 7: {
      const doSub7 = operation === 'sub' || (operation !== 'add' && rng() < 0.45);
      if (doSub7) {
        a = Math.floor(rng() * 70) + 25;
        b = Math.floor(rng() * (a - 5)) + 5;
        answer = a - b;
        op = '-';
        question = `What is ${a} - ${b}?`;
      } else {
        a = Math.floor(rng() * 80) + 5;
        b = Math.floor(rng() * (100 - a - 1)) + 1;
        answer = a + b;
        op = '+';
        question = `What is ${a} + ${b}?`;
      }
      break;
    }

    case 8: {
      a = Math.floor(rng() * 5) + 1;
      b = Math.floor(rng() * 10) + 1;
      answer = a * b;
      op = '\u00d7';
      question = `What is ${a} \u00d7 ${b}?`;
      break;
    }

    case 9: {
      a = Math.floor(rng() * 12) + 1;
      b = Math.floor(rng() * 12) + 1;
      answer = a * b;
      op = '\u00d7';
      question = `What is ${a} \u00d7 ${b}?`;
      break;
    }

    case 10: {
      const divisor = Math.floor(rng() * 11) + 2;
      answer = Math.floor(rng() * 12) + 1;
      a = divisor * answer;
      b = divisor;
      op = '\u00f7';
      question = `What is ${a} \u00f7 ${b}?`;
      break;
    }

    default:
      a = 1; b = 1; answer = 2; op = '+';
      question = 'What is 1 + 1?';
  }

  return { a, b, answer, question, op };
}

function makeChoices(answer, rng) {
  const choices = [{ text: String(answer), correct: true }];
  const wrongs = new Set();
  const spread = answer <= 20 ? 3 : answer <= 50 ? 5 : answer <= 100 ? 10 : 15;

  while (wrongs.size < 2) {
    let wrong = answer + (Math.floor(rng() * spread) + 1) * (rng() < 0.5 ? 1 : -1);
    if (wrong < 0) wrong = answer + Math.floor(rng() * spread) + 1;
    if (wrong !== answer && !wrongs.has(wrong)) {
      wrongs.add(wrong);
      choices.push({ text: String(wrong), correct: false });
    }
  }

  // Shuffle with rng
  for (let i = choices.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    [choices[i], choices[j]] = [choices[j], choices[i]];
  }

  return choices;
}

export function generateChallenge(profile, rng) {
  // Sample a band from the distribution around the center
  const spreadWidth = profile.spreadWidth ?? 0.5;
  const dist = bandDistribution(profile.mathBand, spreadWidth);
  const sampledBand = sampleFromDistribution(dist, rng);

  const operation = pickOperation(profile, sampledBand, rng);
  const { a, b, answer, question, op } = generateNumbers(sampledBand, operation, rng);
  const choices = makeChoices(answer, rng);

  return Object.freeze({
    question,
    correctAnswer: answer,
    choices: Object.freeze(choices.map(c => Object.freeze(c))),
    operation,
    centerBand: profile.mathBand,
    sampledBand,
    band: sampledBand, // backward compat — adapter and events use this
    numbers: Object.freeze({ a, b, op }),
  });
}
