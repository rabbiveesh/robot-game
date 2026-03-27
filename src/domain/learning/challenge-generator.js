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

// ─── SUB-SKILL CLASSIFICATION ───────────────────────────

export function classifyAddition(a, b) {
  if (a < 10 && b < 10) return 'add_single';
  const onesSum = (a % 10) + (b % 10);
  if (onesSum < 10) return 'add_no_carry';
  const tensSum = Math.floor(a / 10) + Math.floor(b / 10) + (onesSum >= 10 ? 1 : 0);
  if (tensSum >= 10) return 'add_carry_tens';
  return 'add_carry';
}

export function classifySubtraction(a, b) {
  if (a < 10 && b < 10) return 'sub_single';
  const onesA = a % 10;
  const onesB = b % 10;
  if (onesA >= onesB) return 'sub_no_borrow';
  // Borrow from ones — check if tens borrow also propagates
  const tensA = Math.floor(a / 10) % 10;
  const tensB = Math.floor(b / 10) % 10;
  // After borrowing 1 from tens for ones, does tens column still need to borrow?
  if (tensA - 1 < tensB) return 'sub_borrow_tens';
  return 'sub_borrow';
}

export function classifyMultiplication(a, b) {
  const smaller = Math.min(a, b);
  const larger = Math.max(a, b);
  if (smaller <= 2) return 'mul_trivial';
  if (smaller <= 5 && larger <= 6) return 'mul_easy';
  return 'mul_hard';
}

export function classifyDivision(dividend, divisor) {
  const answer = dividend / divisor;
  const mulClass = classifyMultiplication(divisor, answer);
  if (mulClass === 'mul_trivial') return 'div_easy';
  if (mulClass === 'mul_easy') return 'div_easy';
  return 'div_hard';
}

export function classifyBond(total, part) {
  if (total <= 10) return 'bond_small';
  return 'bond_large';
}

function classifyChallenge(a, b, operation) {
  switch (operation) {
    case 'add': return classifyAddition(a, b);
    case 'sub': return classifySubtraction(a, b);
    case 'multiply': return classifyMultiplication(a, b);
    case 'divide': return classifyDivision(a, b);
    case 'number_bond': return classifyBond(a, b); // a=total, b=part for bonds
    default: return null;
  }
}

// ─── FEATURE EXTRACTION ─────────────────────────────────

export function extractFeatures(a, b, operation, answer) {
  const onesA = a % 10;
  const onesB = b % 10;
  const tensA = Math.floor(a / 10);
  const tensB = Math.floor(b / 10);

  return Object.freeze({
    carries: operation === 'add' && onesA + onesB >= 10,
    carriesTens: operation === 'add' && tensA + tensB + (onesA + onesB >= 10 ? 1 : 0) >= 10,
    borrows: operation === 'sub' && onesA < onesB,
    borrowsTens: operation === 'sub' && (tensA % 10) - (onesA < onesB ? 1 : 0) < (tensB % 10),
    crossesTenBoundary: Math.floor(a / 10) !== Math.floor(answer / 10),
    maxDigit: Math.max(...String(a).split('').map(Number), ...String(b).split('').map(Number)),
    maxDigitGte7: Math.max(onesA, onesB) >= 7,
    hasRoundNumber: onesA === 0 || onesB === 0,
    nearDoubles: Math.abs(a - b) <= 2 && operation === 'add',
    answerSize: answer,
    answerGte10: answer >= 10,
    answerGte20: answer >= 20,
    answerGte50: answer >= 50,
    operandSize: Math.max(a, b),
    isSquare: operation === 'multiply' && a === b,
    hasFactorFive: (operation === 'multiply' || operation === 'divide') && (a % 5 === 0 || b % 5 === 0),
    bothFactorsGt5: operation === 'multiply' && Math.min(a, b) > 5,
    onesPair: `${Math.min(onesA, onesB)}_${Math.max(onesA, onesB)}`,
  });
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
  9: ['add', 'sub', 'multiply'],              // mix — hard add/sub + multiplication
  10: ['add', 'sub', 'multiply', 'divide'],    // everything — maintain fluency across all ops
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

const DISPLAY_OP = { '+': '+', '-': '\u2212', '\u00d7': '\u00d7', '\u00f7': '\u00f7' };
const SPEECH_OP = { '+': 'plus', '-': 'minus', '\u00d7': 'times', '\u00f7': 'divided by' };

function generateNumbers(band, operation, rng) {
  let a, b, answer, question, op;
  let format = 'standard'; // 'standard' = "What is a op b?" or 'bond' = "What op b = total?"
  let bondTotal = null;

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
        format = 'bond'; bondTotal = total;
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
        format = 'bond'; bondTotal = total;
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

  return { a, b, answer, question, op, format, bondTotal };
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
  const { a, b, answer, question, op, format, bondTotal } = generateNumbers(sampledBand, operation, rng);
  const choices = makeChoices(answer, rng);
  const subSkill = classifyChallenge(a, b, operation);
  const features = extractFeatures(a, b, operation, answer);

  // Produce display + speech from structured data (not regex on question string)
  const dOp = DISPLAY_OP[op] || op;
  const sOp = SPEECH_OP[op] || op;
  let displayText, speechText;
  if (format === 'bond') {
    displayText = `What ${dOp} ${b} = ${bondTotal}?`;
    speechText = `What ${sOp} ${b} equals ${bondTotal}?`;
  } else {
    displayText = `What is ${a} ${dOp} ${b}?`;
    speechText = `What is ${a} ${sOp} ${b}?`;
  }

  return Object.freeze({
    question,
    displayText,
    speechText,
    correctAnswer: answer,
    choices: Object.freeze(choices.map(c => Object.freeze(c))),
    operation,
    subSkill,
    features,
    centerBand: profile.mathBand,
    sampledBand,
    band: sampledBand,
    numbers: Object.freeze({ a, b, op }),
  });
}
