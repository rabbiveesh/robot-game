#!/usr/bin/env node

// CLI Intake Simulator — simulates a kid going through intake + challenges.
// Prints profile evolution to stdout so designers/parents can QA the adaptive system.
//
// Usage:
//   node tools/simulate.js --profile gifted
//   node tools/simulate.js --profile struggling --questions 50
//   node tools/simulate.js --profile seven-year-old --intake-only
//   node tools/simulate.js --intake correct,correct,wrong,correct --speed fast --questions 30
//   node tools/simulate.js --profile gifted --seed 42

import { createProfile, learnerReducer } from '../src/domain/learning/index.js';
import { generateChallenge } from '../src/domain/learning/challenge-generator.js';
import {
  generateIntakeQuestion, processIntakeResults, nextIntakeBand,
} from '../src/domain/learning/intake-assessor.js';
import { detectFrustration } from '../src/domain/learning/frustration-detector.js';
import { accuracy } from '../src/domain/learning/rolling-window.js';

// ─── COLORS ──────────────────────────────────────────────

const C = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  green: '\x1b[32m',
  red: '\x1b[31m',
  yellow: '\x1b[33m',
  cyan: '\x1b[36m',
  magenta: '\x1b[35m',
  white: '\x1b[37m',
  bgRed: '\x1b[41m',
  bgGreen: '\x1b[42m',
  bgYellow: '\x1b[43m',
};

// ─── SIMULATED KID PROFILES ─────────────────────────────

const PROFILES = {
  gifted: {
    name: 'Gifted 4yo',
    accuracy: { 1: 0.99, 2: 0.95, 3: 0.90, 4: 0.85, 5: 0.75, 6: 0.50, 7: 0.30, 8: 0.20, 9: 0.10, 10: 0.05 },
    speed: { fast: [800, 2500], normal: [1500, 4000] },
    boredomChance: 0.15,
    skipsText: true,
  },
  struggling: {
    name: 'Struggling 5yo',
    accuracy: { 1: 0.80, 2: 0.60, 3: 0.40, 4: 0.20, 5: 0.10, 6: 0.05, 7: 0.02, 8: 0.01, 9: 0.01, 10: 0.01 },
    speed: { fast: [4000, 8000], normal: [6000, 12000] },
    boredomChance: 0,
    skipsText: false,
  },
  'seven-year-old': {
    name: 'Typical 7yo',
    accuracy: { 1: 0.99, 2: 0.98, 3: 0.95, 4: 0.92, 5: 0.85, 6: 0.80, 7: 0.70, 8: 0.55, 9: 0.35, 10: 0.20 },
    speed: { fast: [1000, 3000], normal: [2000, 5000] },
    boredomChance: 0.10,
    skipsText: true,
  },
  '2e': {
    name: '2e kid (high reasoning, slow processing)',
    accuracy: { 1: 0.95, 2: 0.95, 3: 0.90, 4: 0.88, 5: 0.80, 6: 0.75, 7: 0.65, 8: 0.50, 9: 0.30, 10: 0.15 },
    speed: { fast: [5000, 9000], normal: [7000, 15000] },
    boredomChance: 0.05,
    skipsText: false,
  },
};

// ─── SEEDED PRNG ─────────────────────────────────────────

function createRng(seed) {
  let s = seed;
  for (let i = 0; i < 10; i++) {
    s = (s * 1664525 + 1013904223) & 0x7fffffff;
  }
  return () => {
    s = (s * 1664525 + 1013904223) & 0x7fffffff;
    return s / 0x7fffffff;
  };
}

function seedFromString(str) {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) - hash + str.charCodeAt(i)) | 0;
  }
  return Math.abs(hash) || 42;
}

// ─── PARSE CLI ARGS ──────────────────────────────────────

function parseArgs() {
  const args = process.argv.slice(2);
  const opts = {
    profile: null,
    intake: null,       // manual intake pattern e.g. 'correct,correct,wrong,correct'
    speed: null,        // 'fast' or 'normal'
    questions: 30,
    intakeOnly: false,
    seed: null,
  };

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case '--profile': opts.profile = args[++i]; break;
      case '--intake': opts.intake = args[++i]; break;
      case '--speed': opts.speed = args[++i]; break;
      case '--questions': opts.questions = parseInt(args[++i]) || 30; break;
      case '--intake-only': opts.intakeOnly = true; break;
      case '--seed': opts.seed = parseInt(args[++i]); break;
      case '--help': case '-h': printHelp(); process.exit(0);
      default:
        console.error(`Unknown arg: ${args[i]}`);
        printHelp();
        process.exit(1);
    }
  }

  if (!opts.profile && !opts.intake) {
    console.error('Need --profile or --intake');
    printHelp();
    process.exit(1);
  }

  return opts;
}

function printHelp() {
  console.log(`
${C.bold}Usage:${C.reset}
  node tools/simulate.js --profile <name> [options]
  node tools/simulate.js --intake <pattern> [options]

${C.bold}Profiles:${C.reset} gifted, struggling, seven-year-old, 2e

${C.bold}Options:${C.reset}
  --profile <name>     Simulated kid profile
  --intake <pattern>   Manual intake: correct,correct,wrong,correct
  --speed <fast|normal> Override response speed
  --questions <n>      Number of post-intake challenges (default: 30)
  --intake-only        Only run the intake quiz
  --seed <n>           RNG seed for reproducible output
`);
}

// ─── BAND NAMES ──────────────────────────────────────────

const BAND_NAMES = ['', 'Add <5', '+/- <10', '+/- <15', '+/- <20', 'x1 x2',
  '+/- <50', '+/- <100', 'x1-5', 'x1-12', 'Divide'];

// ─── SIMULATION ──────────────────────────────────────────

function simulateAnswer(kidProfile, band, rng, isEasyForKid) {
  const accProb = kidProfile.accuracy[band] ?? 0.5;
  const correct = rng() < accProb;

  // Boredom: on easy questions with a correct streak, sometimes mash fast wrong
  const boredom = isEasyForKid && !correct && rng() < (kidProfile.boredomChance || 0);

  const speedRange = kidProfile.speed?.fast || [1500, 4000];
  const normalRange = kidProfile.speed?.normal || [2000, 5000];
  const range = correct ? speedRange : normalRange;
  let responseTimeMs = range[0] + rng() * (range[1] - range[0]);

  // Boredom wrongs are very fast
  if (boredom) {
    responseTimeMs = 200 + rng() * 800;
  }

  return { correct, responseTimeMs, boredom };
}

function run() {
  const opts = parseArgs();
  const kidProfile = opts.profile ? PROFILES[opts.profile] : null;

  if (opts.profile && !kidProfile) {
    console.error(`Unknown profile: ${opts.profile}. Available: ${Object.keys(PROFILES).join(', ')}`);
    process.exit(1);
  }

  const seed = opts.seed ?? (opts.profile ? seedFromString(opts.profile) : 42);
  const rng = createRng(seed);

  const profileName = kidProfile?.name || 'Custom';
  console.log(`\n${C.bold}${C.cyan}Simulating: ${profileName}${C.reset}  ${C.dim}(seed: ${seed}, questions: ${opts.intakeOnly ? 'intake only' : opts.questions})${C.reset}\n`);

  // ─── INTAKE ──────────────────────────────────────────

  console.log(`${C.bold}${C.yellow}${'═'.repeat(50)}${C.reset}`);
  console.log(`${C.bold}${C.yellow}  INTAKE (Sparky's Calibration)${C.reset}`);
  console.log(`${C.bold}${C.yellow}${'═'.repeat(50)}${C.reset}\n`);

  let currentBand = 3;
  const intakeAnswers = [];

  // Manual intake pattern or simulated
  const manualPattern = opts.intake
    ? opts.intake.split(',').map(s => s.trim().toLowerCase() === 'correct')
    : null;
  const intakeCount = manualPattern ? manualPattern.length : 4;

  for (let i = 0; i < intakeCount; i++) {
    const challenge = generateIntakeQuestion(currentBand, i, rng);
    let correct, responseTimeMs;

    if (manualPattern) {
      correct = manualPattern[i];
      responseTimeMs = opts.speed === 'fast' ? 1500 + rng() * 1500 : 3000 + rng() * 4000;
    } else {
      const sim = simulateAnswer(kidProfile, currentBand, rng, false);
      correct = sim.correct;
      responseTimeMs = sim.responseTimeMs;
    }

    const skippedText = kidProfile?.skipsText && rng() < 0.5;

    intakeAnswers.push({ band: currentBand, correct, responseTimeMs, skippedText });

    const mark = correct ? `${C.green}✓${C.reset}` : `${C.red}✗${C.reset}`;
    const timeStr = `${(responseTimeMs / 1000).toFixed(1)}s`;
    const nextBand = nextIntakeBand(currentBand, correct, 10);
    const skipTag = skippedText ? `  ${C.dim}[skipped text]${C.reset}` : '';
    console.log(`  Q${i + 1}  band:${currentBand}  ${challenge.question.padEnd(20)} ${mark}  ${timeStr}  → next band: ${nextBand}${skipTag}`);

    currentBand = nextBand;
  }

  const intakeResult = processIntakeResults(intakeAnswers);

  console.log(`\n  ${C.bold}Intake result:${C.reset}`);
  console.log(`    Placed at band: ${C.bold}${intakeResult.mathBand}${C.reset} (${BAND_NAMES[intakeResult.mathBand] || '?'})`);
  console.log(`    Pace: ${intakeResult.pace.toFixed(2)}   Scaffolding: ${intakeResult.scaffolding.toFixed(2)}`);
  console.log(`    Promote threshold: ${intakeResult.promoteThreshold}  Stretch threshold: ${intakeResult.stretchThreshold}`);
  console.log(`    Text speed: ${intakeResult.textSpeed}`);
  console.log();

  if (opts.intakeOnly) {
    return;
  }

  // ─── PLAY SESSION ────────────────────────────────────

  let profile = createProfile();
  profile = learnerReducer(profile, { type: 'INTAKE_COMPLETED', ...intakeResult });
  let recentBehaviors = [];
  let frustrationEvents = 0;
  let totalCorrect = 0;
  const totalQuestions = opts.questions;

  console.log(`${C.bold}${C.yellow}${'═'.repeat(50)}${C.reset}`);
  console.log(`${C.bold}${C.yellow}  PLAY SESSION (${totalQuestions} challenges)${C.reset}`);
  console.log(`${C.bold}${C.yellow}${'═'.repeat(50)}${C.reset}\n`);

  for (let q = 1; q <= totalQuestions; q++) {
    const challenge = generateChallenge(profile, rng);
    const centerBand = profile.mathBand;
    const sampledBand = challenge.sampledBand;

    // Is this easy for the kid? (sampled band well below their ceiling)
    const kidAcc = kidProfile?.accuracy[sampledBand] ?? 0.7;
    const isEasyForKid = kidAcc > 0.85 && profile.streak >= 2;

    const sim = simulateAnswer(
      kidProfile || { accuracy: { [sampledBand]: 0.7 }, speed: { fast: [1500, 4000], normal: [2000, 5000] }, boredomChance: 0 },
      sampledBand,
      rng,
      isEasyForKid,
    );

    // Simulate text skipping behavior
    if (kidProfile?.skipsText && rng() < 0.3) {
      recentBehaviors.push({ signal: 'text_skipped', timestamp: Date.now() });
      profile = learnerReducer(profile, { type: 'BEHAVIOR', signal: 'text_skipped' });
    }

    const event = {
      type: 'PUZZLE_ATTEMPTED',
      correct: sim.correct,
      operation: challenge.operation,
      subSkill: challenge.subSkill,
      band: sampledBand,
      centerBand,
      responseTimeMs: sim.responseTimeMs,
      attemptNumber: 1,
      timestamp: Date.now(),
      features: challenge.features,
    };

    const prevBand = profile.mathBand;
    profile = learnerReducer(profile, event);
    const newBand = profile.mathBand;

    if (sim.correct) totalCorrect++;

    // Frustration detection
    const frust = detectFrustration(profile.rollingWindow, recentBehaviors);
    let frustTag = '';
    if (frust.level === 'high') {
      profile = learnerReducer(profile, { type: 'FRUSTRATION_DETECTED', level: 'high' });
      frustrationEvents++;
      frustTag = `  ${C.bgRed}${C.white} FRUSTRATION: ${frust.level} → ${frust.recommendation} ${C.reset}`;
    } else if (frust.level === 'mild') {
      frustTag = `  ${C.yellow}mild: ${frust.recommendation}${C.reset}`;
    }

    // Format output line
    const mark = sim.correct ? `${C.green}✓${C.reset}` : `${C.red}✗${C.reset}`;
    const timeStr = `${(sim.responseTimeMs / 1000).toFixed(1)}s`.padStart(5);
    const skillStr = (challenge.subSkill || challenge.operation).padEnd(14);
    const qStr = challenge.question.replace(/\n/g, ' ');
    const num = `#${q}`.padStart(4);
    const bandStr = sampledBand !== centerBand
      ? `${C.dim}c:${centerBand}${C.reset}→${sampledBand}`
      : `band:${sampledBand}`;
    const spreadStr = `sw:${profile.spreadWidth.toFixed(2)}`;

    let bandTag = '';
    if (newBand > prevBand) {
      bandTag = `  ${C.bgGreen}${C.bold} ⬆ PROMOTED → band:${newBand} (${BAND_NAMES[newBand]}) ${C.reset}`;
    } else if (newBand < prevBand) {
      bandTag = `  ${C.bgRed}${C.bold} ⬇ DEMOTED → band:${newBand} (${BAND_NAMES[newBand]}) ${C.reset}`;
    }

    let boredomTag = '';
    if (sim.boredom && !sim.correct) {
      // Check if the reducer actually treated it as boredom
      const lastEntry = profile.rollingWindow.entries[profile.rollingWindow.entries.length - 1];
      if (lastEntry?.boredom) {
        boredomTag = `  ${C.magenta}[BOREDOM — not penalized]${C.reset}`;
      }
    }

    console.log(`  ${num}  ${skillStr} ${bandStr.padEnd(12)} ${qStr.padEnd(22)} ${mark}  ${timeStr}  ${spreadStr}${bandTag}${boredomTag}${frustTag}`);
  }

  // ─── FINAL PROFILE ───────────────────────────────────

  const win = profile.rollingWindow;
  const finalAcc = win.entries.length > 0 ? accuracy(win) : 0;
  const finalCorrectInWindow = win.entries.filter(e => e.correct).length;

  console.log(`\n${C.bold}${C.yellow}${'═'.repeat(50)}${C.reset}`);
  console.log(`${C.bold}${C.yellow}  FINAL PROFILE${C.reset}`);
  console.log(`${C.bold}${C.yellow}${'═'.repeat(50)}${C.reset}\n`);

  console.log(`  Band: ${C.bold}${profile.mathBand}${C.reset} (${BAND_NAMES[profile.mathBand] || '?'})${''.padEnd(8)}Questions: ${totalQuestions}`);
  console.log(`  Spread: ${profile.spreadWidth.toFixed(2)}${''.padEnd(12)}Pace: ${profile.pace.toFixed(2)}${''.padEnd(8)}Scaffolding: ${profile.scaffolding.toFixed(2)}`);
  console.log(`  Frustration events: ${frustrationEvents}`);
  console.log(`  Overall accuracy: ${((totalCorrect / totalQuestions) * 100).toFixed(0)}% (${totalCorrect}/${totalQuestions})`);
  console.log(`  Rolling window accuracy: ${(finalAcc * 100).toFixed(0)}% (${finalCorrectInWindow}/${win.entries.length})`);
  console.log();

  // Sub-skill breakdown
  console.log(`  ${C.bold}Sub-skill breakdown:${C.reset}`);
  const subSkillGroups = [
    { header: 'Addition', skills: ['add_single', 'add_no_carry', 'add_carry', 'add_carry_tens'] },
    { header: 'Subtraction', skills: ['sub_single', 'sub_no_borrow', 'sub_borrow', 'sub_borrow_tens'] },
    { header: 'Multiplication', skills: ['mul_trivial', 'mul_easy', 'mul_hard'] },
    { header: 'Division', skills: ['div_easy', 'div_hard'] },
    { header: 'Number bonds', skills: ['bond_small', 'bond_large'] },
  ];
  for (const group of subSkillGroups) {
    const hasAny = group.skills.some(sk => profile.operationStats[sk]?.attempts > 0);
    if (!hasAny) continue;
    console.log(`    ${C.bold}${group.header}:${C.reset}`);
    for (const sk of group.skills) {
      const s = profile.operationStats[sk];
      if (!s || s.attempts === 0) continue;
      const pct = ((s.correct / s.attempts) * 100).toFixed(0);
      const tag = s.correct / s.attempts >= 0.75 ? `${C.green}strength${C.reset}` :
        s.correct / s.attempts < 0.5 ? `${C.red}growth area${C.reset}` :
          `${C.yellow}developing${C.reset}`;
      const label = sk.replace(/_/g, ' ');
      console.log(`      ${label.padEnd(16)} ${pct.padStart(3)}% (${s.correct}/${s.attempts})   ${tag}`);
    }
  }
  console.log();
}

run();
