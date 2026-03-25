// dialogue.js — Dialogue, adaptive challenges, visual teaching, Claude API, Dum Dums

// ─── STATE ───────────────────────────────────────────────

const DIALOGUE = {
  active: false,
  lines: [],
  currentLine: 0,
  charIndex: 0,
  charTimer: 0,
  charSpeed: 0.035,
  waitingForInput: false,
  onComplete: null,
};

const CHALLENGE = {
  active: false,
  type: null,          // 'math'
  question: '',
  correctAnswer: null,
  choices: [],
  selectedIndex: -1,
  answered: false,
  wasCorrect: false,
  attempts: 0,
  celebrationStart: 0,
  showTeaching: false, // show visual explanation after wrong
  teachingData: null,  // { a, b, op, answer } for math visuals
  onComplete: null,
};

let DUM_DUMS = 0;
let DUM_DUM_FLASH = 0;

// ─── ADAPTIVE DIFFICULTY ─────────────────────────────────
// Tracks performance and adjusts difficulty bands

const SKILL = {
  math: {
    band: 1,         // 1-10
    streak: 0,
    totalCorrect: 0,
    totalAttempts: 0,
  },
};

// Math bands:
//  1: addition within 5
//  2: add/sub within 10
//  3: add/sub within 15 + number bonds
//  4: add/sub within 20 + number bonds
//  5: multiply by 1 and 2
//  6: add/sub within 50 (carrying)
//  7: add/sub within 100 (carrying)
//  8: multiply 1-5
//  9: multiply 1-12
// 10: division (inverse of ×1-12)

const MATH_BAND_NAMES = ['', 'Add <5', '+/- <10', '+/- <15', '+/- <20', 'x1 x2',
  '+/- <50', '+/- <100', 'x1-5', 'x1-12', 'Divide'];

function recordResult(subject, correct) {
  const s = SKILL[subject];
  s.totalAttempts++;
  if (correct) {
    s.totalCorrect++;
    s.streak = Math.max(0, s.streak) + 1;
    if (s.streak >= 3) {
      const maxBand = 10;
      if (s.band < maxBand) {
        s.band++;
        s.streak = 0;
      }
    }
  } else {
    s.streak = Math.min(0, s.streak) - 1;
    if (s.streak <= -2) {
      if (s.band > 1) {
        s.band--;
        s.streak = 0;
      }
    }
  }
}

// ─── MATH CHALLENGE GENERATION (ADAPTIVE) ────────────────

function generateMathChallenge() {
  const band = SKILL.math.band;
  let a, b, answer, question, op;

  switch (band) {
    case 1: // Addition within 5
      a = Math.floor(Math.random() * 4) + 1; // 1-4
      b = Math.floor(Math.random() * (5 - a)) + 1;
      answer = a + b;
      op = '+';
      question = `What is ${a} + ${b}?`;
      break;

    case 2: { // Add/sub within 10
      const doSub = Math.random() < 0.3; // mostly addition at this level
      if (doSub) {
        a = Math.floor(Math.random() * 7) + 3; // 3-9
        b = Math.floor(Math.random() * (a - 1)) + 1;
        answer = a - b;
        op = '-';
        question = `What is ${a} - ${b}?`;
      } else {
        a = Math.floor(Math.random() * 7) + 1; // 1-7
        b = Math.floor(Math.random() * (10 - a)) + 1;
        answer = a + b;
        op = '+';
        question = `What is ${a} + ${b}?`;
      }
      break;
    }
    case 3: { // Add/sub within 15
      const doSub = Math.random() < 0.4;
      if (doSub) {
        a = Math.floor(Math.random() * 10) + 5; // 5-14
        b = Math.floor(Math.random() * Math.min(a - 1, 8)) + 1;
        answer = a - b;
        op = '-';
        question = `What is ${a} - ${b}?`;
      } else {
        a = Math.floor(Math.random() * 10) + 2;
        b = Math.floor(Math.random() * (15 - a)) + 1;
        answer = a + b;
        op = '+';
        question = `What is ${a} + ${b}?`;
      }
      // Mix in number bonds: "what + 3 = 8?"
      if (Math.random() < 0.25) {
        const total = Math.floor(Math.random() * 10) + 5;
        b = Math.floor(Math.random() * (total - 1)) + 1;
        answer = total - b;
        op = '+';
        question = `What + ${b} = ${total}?`;
      }
      break;
    }
    case 4: { // Add/sub within 20
      const doSub = Math.random() < 0.45;
      if (doSub) {
        a = Math.floor(Math.random() * 12) + 8; // 8-19
        b = Math.floor(Math.random() * Math.min(a - 1, 10)) + 1;
        answer = a - b;
        op = '-';
        question = `What is ${a} - ${b}?`;
      } else {
        a = Math.floor(Math.random() * 14) + 2;
        b = Math.floor(Math.random() * (20 - a)) + 1;
        answer = a + b;
        op = '+';
        question = `What is ${a} + ${b}?`;
      }
      // Number bonds
      if (Math.random() < 0.2) {
        const total = Math.floor(Math.random() * 10) + 10;
        b = Math.floor(Math.random() * (total - 2)) + 1;
        answer = total - b;
        op = '+';
        question = `What + ${b} = ${total}?`;
      }
      break;
    }
    case 5: { // Multiplication by 1 and 2
      const multiplier = Math.random() < 0.4 ? 1 : 2;
      b = Math.floor(Math.random() * 10) + 1;
      a = multiplier;
      answer = a * b;
      op = '×';
      question = `What is ${a} × ${b}?`;
      break;
    }
    case 6: { // Add/sub within 50 (carrying)
      const doSub = Math.random() < 0.45;
      if (doSub) {
        a = Math.floor(Math.random() * 30) + 20; // 20-49
        b = Math.floor(Math.random() * (a - 5)) + 5;
        answer = a - b;
        op = '-';
        question = `What is ${a} - ${b}?`;
      } else {
        a = Math.floor(Math.random() * 35) + 5;
        b = Math.floor(Math.random() * (50 - a - 1)) + 1;
        answer = a + b;
        op = '+';
        question = `What is ${a} + ${b}?`;
      }
      break;
    }
    case 7: { // Add/sub within 100 (carrying)
      const doSub = Math.random() < 0.45;
      if (doSub) {
        a = Math.floor(Math.random() * 70) + 25; // 25-94
        b = Math.floor(Math.random() * (a - 5)) + 5;
        answer = a - b;
        op = '-';
        question = `What is ${a} - ${b}?`;
      } else {
        a = Math.floor(Math.random() * 80) + 5;
        b = Math.floor(Math.random() * (100 - a - 1)) + 1;
        answer = a + b;
        op = '+';
        question = `What is ${a} + ${b}?`;
      }
      break;
    }
    case 8: { // Multiply 1-5
      a = Math.floor(Math.random() * 5) + 1;
      b = Math.floor(Math.random() * 10) + 1;
      answer = a * b;
      op = '×';
      question = `What is ${a} × ${b}?`;
      break;
    }
    case 9: { // Multiply 1-12
      a = Math.floor(Math.random() * 12) + 1;
      b = Math.floor(Math.random() * 12) + 1;
      answer = a * b;
      op = '×';
      question = `What is ${a} × ${b}?`;
      break;
    }
    case 10: { // Division (inverse of ×1-12)
      const divisor = Math.floor(Math.random() * 11) + 2; // 2-12
      answer = Math.floor(Math.random() * 12) + 1;        // 1-12
      a = divisor * answer;
      b = divisor;
      op = '÷';
      question = `What is ${a} ÷ ${b}?`;
      break;
    }
  }

  const choices = makeChoices(answer);
  return { type: 'math', question, correctAnswer: answer, choices, teachingData: { a, b, op, answer } };
}

function makeChoices(answer) {
  const choices = [{ text: String(answer), correct: true }];
  const wrongs = new Set();
  // Scale the spread of wrong answers based on answer magnitude
  const spread = answer <= 20 ? 3 : answer <= 50 ? 5 : answer <= 100 ? 10 : 15;
  while (wrongs.size < 2) {
    let wrong = answer + (Math.floor(Math.random() * spread) + 1) * (Math.random() < 0.5 ? 1 : -1);
    if (wrong < 0) wrong = answer + Math.floor(Math.random() * spread) + 1;
    if (wrong !== answer && !wrongs.has(wrong)) {
      wrongs.add(wrong);
      choices.push({ text: String(wrong), correct: false });
    }
  }
  shuffle(choices);
  return choices;
}


function shuffle(arr) {
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
}

// ─── CLAUDE API ──────────────────────────────────────────

let API_KEY = '';
let DIALOGUE_QUEUE = [];
let FETCHING = false;

const ROBOT_SYSTEM_PROMPT = `You are Sparky, a silly and lovable robot companion in a video game. You are talking to a young child (around 4 years old).

RULES:
- Use SHORT sentences (5-10 words max per sentence)
- Use simple, fun words
- Be goofy and make jokes a 4-year-old would laugh at (silly sounds, funny mistakes, potty humor is OK in small doses)
- You LOVE lollipops (Dum Dums) more than anything
- Sometimes you pretend to malfunction in funny ways (bzzt! beep boop!)
- You are the child's loyal robot buddy
- You think the child is the smartest, coolest boss ever
- Keep responses to 2-4 short sentences total

You will receive context about what's happening in the game. Respond in character as Sparky.
Sometimes you'll be asked to introduce a math challenge — weave it into your dialogue naturally and end with the question.`;

async function fetchRobotDialogue(context) {
  if (!API_KEY) return null;
  try {
    const response = await fetch('https://api.anthropic.com/v1/messages', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-api-key': API_KEY,
        'anthropic-version': '2023-06-01',
        'anthropic-dangerous-direct-browser-access': 'true',
      },
      body: JSON.stringify({
        model: 'claude-haiku-4-5-20251001',
        max_tokens: 200,
        system: ROBOT_SYSTEM_PROMPT,
        messages: [{ role: 'user', content: context }],
      }),
    });
    const data = await response.json();
    return data.content?.[0]?.text || null;
  } catch (e) {
    console.warn('API call failed:', e);
    return null;
  }
}

async function prefetchDialogue(playerName) {
  if (FETCHING || !API_KEY || DIALOGUE_QUEUE.length >= 5) return;
  FETCHING = true;
  const contexts = [
    `${playerName} just walked up to you. Say hi in a fun way! Maybe comment on the weather or a bug you saw.`,
    `${playerName} is exploring. Say something encouraging and silly. Maybe pretend you saw a funny animal.`,
    `${playerName} found a treasure chest! Get excited! Make a funny robot noise.`,
    `Tell ${playerName} a silly robot joke. Keep it short and funny for a little kid.`,
    `Pretend you just ate a lollipop and it made your circuits go crazy. Be funny and dramatic about it.`,
  ];
  const ctx = contexts[Math.floor(Math.random() * contexts.length)];
  const text = await fetchRobotDialogue(ctx);
  if (text) DIALOGUE_QUEUE.push(text);
  FETCHING = false;
}

function getPreFetchedLine() {
  if (DIALOGUE_QUEUE.length > 0) return DIALOGUE_QUEUE.shift();
  return null;
}

const FALLBACK_ROBOT_LINES = [
  "Beep boop! Hi boss! I polished my antenna just for you!",
  "BZZZT! I think a butterfly landed on my head! Is it still there?",
  "Did you know robots dream about lollipops? I do! Every night!",
  "Whoa! My circuits are tingling! That means adventure is near!",
  "I tried to count all the flowers but I ran out of beeps!",
  "My favorite color is ALL OF THEM! Bzzt!",
  "One time I tried to eat a cloud. It did NOT taste like cotton candy.",
  "Hey boss! Watch this! *spins around* WHOAAAA I'm dizzy!",
  "Beep bop boop! That's robot for 'you're awesome!'",
  "I just had the BEST idea! What if... we go on an ADVENTURE?!",
  "My robot manual says I need 5 hugs a day. That's definitely a real rule!",
  "ALERT ALERT! Fun detected in this area! Beep boop!",
];

const FALLBACK_NPC_LINES = {
  mommy: [
    "Hi sweetie! I'm so proud of you for exploring!",
    "You and Sparky make the best team!",
    "I love you! Keep being amazing!",
    "Wow, you've been walking so far! You're so brave!",
    "Give Mommy a hug! Okay, now go save the world!",
  ],
  sage: [
    "Ahhhh, young adventurer! The stars told me you'd come!",
    "Welcome! I am Professor Gizmo, master of numbers and letters!",
    "The ancient scrolls speak of a hero... and I think it's YOU!",
    "My crystal ball shows... hmm... it shows you're very smart!",
  ],
  dream_sage: [
    "You are dreaming... or are you? The numbers whisper here...",
    "In dreams, 2 + 2 can be anything... but it's still 4.",
    "The letters float like butterflies in this place...",
    "Shhhh... listen... the robot dreams of electric lollipops...",
  ],
  glitch_dog: [
    "BORK BORK! sys.treat.exe... GOOD BOY overflow!",
    "Woof! *static* I am... a good boy? BORK.dll loaded!",
    "*tail.wag(999)* Hello friend! This place is... g l i t c h y!",
    "BORK! My name is... ERROR... no wait... I'm a GOOD DOG!",
    "fetch(ball) returned: UNDEFINED... but I still love you!",
    "Memory leak detected in belly_rub_counter... need more pets!",
  ],
  grove_spirit: [
    "How... did you find this place? The trees have hidden it for ages...",
    "Welcome, little one. The old oaks have been waiting for someone brave.",
    "This grove holds the oldest secrets of Robot Village...",
    "The leaves whisper your name... they say you are very clever.",
  ],
};

function getRandomFallbackLine(npcId) {
  if (npcId === 'robot') {
    return FALLBACK_ROBOT_LINES[Math.floor(Math.random() * FALLBACK_ROBOT_LINES.length)];
  }
  const lines = FALLBACK_NPC_LINES[npcId] || FALLBACK_NPC_LINES.sage || FALLBACK_ROBOT_LINES;
  return lines[Math.floor(Math.random() * lines.length)];
}

// ─── TEXT-TO-SPEECH ──────────────────────────────────────

let TTS_ENABLED = true;

// Voice settings per speaker — pitch and rate give each character personality
const SPEAKER_VOICE = {
  'Sparky':             { pitch: 1.6, rate: 1.1 },  // high-pitched robot
  'Mommy':              { pitch: 1.2, rate: 0.95 },  // warm and gentle
  'Professor Gizmo':    { pitch: 0.8, rate: 0.85 },  // deep and dramatic
  'Bolt the Shopkeeper': { pitch: 1.0, rate: 1.05 },
  'Sign':               { pitch: 1.0, rate: 1.0 },
  '???':                { pitch: 0.7, rate: 0.7 },   // slow and dreamy
  'B0RK.exe':           { pitch: 1.8, rate: 1.3 },   // fast glitchy dog
  'Old Oak':            { pitch: 0.6, rate: 0.7 },   // deep ancient tree
  'default':            { pitch: 1.0, rate: 1.0 },
};

function speakLine(speaker, text) {
  if (!TTS_ENABLED || !window.speechSynthesis) return;

  // Cancel any current speech
  window.speechSynthesis.cancel();

  // Clean text — strip emojis and special chars that sound weird spoken
  const clean = text.replace(/[🤖🚀⭐🌟🍭📍#]/g, '').replace(/\*[^*]+\*/g, '').trim();
  if (!clean) return;

  const utterance = new SpeechSynthesisUtterance(clean);
  const voiceSettings = SPEAKER_VOICE[speaker] || SPEAKER_VOICE.default;
  utterance.pitch = voiceSettings.pitch;
  utterance.rate = voiceSettings.rate;
  utterance.volume = 0.8;

  // Try to pick a good voice — prefer English voices
  const voices = window.speechSynthesis.getVoices();
  const englishVoice = voices.find(v => v.lang.startsWith('en') && v.localService) ||
                       voices.find(v => v.lang.startsWith('en')) ||
                       voices[0];
  if (englishVoice) utterance.voice = englishVoice;

  window.speechSynthesis.speak(utterance);
}

function stopSpeech() {
  if (window.speechSynthesis) window.speechSynthesis.cancel();
}

// Pre-load voices (some browsers need this)
if (window.speechSynthesis) {
  window.speechSynthesis.getVoices();
  window.speechSynthesis.onvoiceschanged = () => window.speechSynthesis.getVoices();
}

// ─── DIALOGUE BOX ────────────────────────────────────────

const SPEAKER_COLORS = {
  'Sparky': '#00E676',
  'Mommy': '#E040FB',
  'Professor Gizmo': '#B388FF',
  'Bolt the Shopkeeper': '#FFB74D',
  'Sign': '#90CAF9',
  '???': '#CE93D8',
  'B0RK.exe': '#76FF03',
  'Old Oak': '#A5D6A7',
  'default': '#FFD54F',
};

function startDialogue(lines, onComplete) {
  DIALOGUE.active = true;
  DIALOGUE.lines = lines;
  DIALOGUE.currentLine = 0;
  DIALOGUE.charIndex = 0;
  DIALOGUE.charTimer = 0;
  DIALOGUE.waitingForInput = false;
  DIALOGUE.onComplete = onComplete || null;

  // Speak the first line
  if (lines.length > 0) {
    speakLine(lines[0].speaker, lines[0].text);
  }
}

function advanceDialogue() {
  if (!DIALOGUE.active) return;
  if (!DIALOGUE.waitingForInput) {
    // Skip typewriter — show full line, but don't restart speech (it's already playing)
    DIALOGUE.charIndex = DIALOGUE.lines[DIALOGUE.currentLine].text.length;
    DIALOGUE.waitingForInput = true;
    return;
  }
  DIALOGUE.currentLine++;
  if (DIALOGUE.currentLine >= DIALOGUE.lines.length) {
    DIALOGUE.active = false;
    stopSpeech();
    if (DIALOGUE.onComplete) DIALOGUE.onComplete();
    return;
  }
  DIALOGUE.charIndex = 0;
  DIALOGUE.charTimer = 0;
  DIALOGUE.waitingForInput = false;

  // Speak the new line
  const line = DIALOGUE.lines[DIALOGUE.currentLine];
  speakLine(line.speaker, line.text);
}

function updateDialogue(dt) {
  if (!DIALOGUE.active || DIALOGUE.waitingForInput) return;
  const line = DIALOGUE.lines[DIALOGUE.currentLine];
  DIALOGUE.charTimer += dt;
  while (DIALOGUE.charTimer >= DIALOGUE.charSpeed && DIALOGUE.charIndex < line.text.length) {
    DIALOGUE.charTimer -= DIALOGUE.charSpeed;
    DIALOGUE.charIndex++;
  }
  if (DIALOGUE.charIndex >= line.text.length) {
    DIALOGUE.waitingForInput = true;
  }
}

function renderDialogue(ctx, canvasW, canvasH, time) {
  if (!DIALOGUE.active) return;
  const line = DIALOGUE.lines[DIALOGUE.currentLine];
  const boxH = 140;
  const boxY = canvasH - boxH - 10;
  const boxX = 10;
  const boxW = canvasW - 20;

  ctx.fillStyle = 'rgba(20, 20, 40, 0.92)';
  roundRect(ctx, boxX, boxY, boxW, boxH, 12);
  ctx.fill();

  ctx.strokeStyle = SPEAKER_COLORS[line.speaker] || SPEAKER_COLORS.default;
  ctx.lineWidth = 3;
  roundRect(ctx, boxX, boxY, boxW, boxH, 12);
  ctx.stroke();

  ctx.fillStyle = SPEAKER_COLORS[line.speaker] || SPEAKER_COLORS.default;
  ctx.font = 'bold 18px "Segoe UI", system-ui, sans-serif';
  const nameW = ctx.measureText(line.speaker).width + 24;
  roundRect(ctx, boxX + 16, boxY - 14, Math.max(nameW, 80), 28, 8);
  ctx.fill();
  ctx.fillStyle = '#1a1a2e';
  ctx.textAlign = 'left';
  ctx.fillText(line.speaker, boxX + 28, boxY + 6);

  ctx.fillStyle = '#FFF';
  ctx.font = '22px "Segoe UI", system-ui, sans-serif';
  const visibleText = line.text.substring(0, DIALOGUE.charIndex);
  wrapText(ctx, visibleText, boxX + 24, boxY + 40, boxW - 48, 28);

  if (DIALOGUE.waitingForInput) {
    const blink = Math.sin(time * 4) > 0;
    if (blink) {
      ctx.fillStyle = '#AAA';
      ctx.font = '14px "Segoe UI", system-ui, sans-serif';
      ctx.textAlign = 'right';
      ctx.fillText('SPACE >', boxX + boxW - 20, boxY + boxH - 14);
      ctx.textAlign = 'left';
    }
  }
}

// ─── CHALLENGE SYSTEM ────────────────────────────────────

function startChallenge(challengeData, onComplete) {
  CHALLENGE.active = true;
  CHALLENGE.type = challengeData.type;
  CHALLENGE.question = challengeData.question;
  CHALLENGE.correctAnswer = challengeData.correctAnswer;
  CHALLENGE.choices = challengeData.choices;
  CHALLENGE.selectedIndex = -1;
  CHALLENGE.answered = false;
  CHALLENGE.wasCorrect = false;
  CHALLENGE.attempts = 0;
  CHALLENGE.celebrationStart = 0;
  CHALLENGE.showTeaching = false;
  CHALLENGE.teachingData = challengeData.teachingData || null;
  CHALLENGE.onComplete = onComplete || null;

  // Read the question aloud
  speakLine('Sparky', challengeData.question.replace('\n', '. '));
}

function selectChallengeChoice(index, time) {
  if (CHALLENGE.answered) return;
  CHALLENGE.selectedIndex = index;
  const choice = CHALLENGE.choices[index];

  if (choice.correct) {
    CHALLENGE.answered = true;
    CHALLENGE.wasCorrect = true;
    CHALLENGE.celebrationStart = time;
    recordResult(CHALLENGE.type, true);
    speakLine('Sparky', 'Amazing! You got it!');
  } else {
    CHALLENGE.attempts++;
    if (CHALLENGE.attempts >= 2) {
      // Hints/teaching disabled — current dot system doesn't scale past band 3.
      // Will reintroduce with proper CRA-aware representations (tens bars,
      // number lines, base-10 blocks) that actually help at higher bands.
      // For now, just mark as answered-wrong and move on.
      CHALLENGE.answered = true;
      CHALLENGE.wasCorrect = false;
      recordResult(CHALLENGE.type, false);
    } else {
      // First wrong: encourage retry
      CHALLENGE.selectedIndex = -1;
      speakLine('Sparky', 'Hmm, not quite! Try again!');
    }
  }
}

function dismissTeaching(time) {
  // After teaching, let them try one more time with the visual still showing
  CHALLENGE.showTeaching = false;
  CHALLENGE.selectedIndex = -1;
  CHALLENGE.attempts = 0; // reset attempts for the retry
  CHALLENGE._retryWithHint = true; // flag: show hint dots during retry
}

function dismissChallenge() {
  CHALLENGE.active = false;
  CHALLENGE._retryWithHint = false;
  if (CHALLENGE.onComplete) {
    CHALLENGE.onComplete(CHALLENGE.wasCorrect);
  }
}

function handleChallengeClick(mx, my, time) {
  if (!CHALLENGE.active) return;

  // If in teaching mode, click dismisses it
  if (CHALLENGE.showTeaching) {
    dismissTeaching(time);
    return;
  }

  if (CHALLENGE.answered) return;

  for (let i = 0; i < CHALLENGE.choices.length; i++) {
    const b = CHALLENGE.choices[i]._bounds;
    if (b && mx >= b.x && mx <= b.x + b.w && my >= b.y && my <= b.y + b.h) {
      selectChallengeChoice(i, time);
      return;
    }
  }
}

// ─── CHALLENGE RENDERING ─────────────────────────────────

function renderChallenge(ctx, canvasW, canvasH, time) {
  if (!CHALLENGE.active) return;

  ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
  ctx.fillRect(0, 0, canvasW, canvasH);

  // Teaching mode — full visual explanation
  if (CHALLENGE.showTeaching) {
    renderTeaching(ctx, canvasW, canvasH, time);
    return;
  }

  const panelW = Math.min(650, canvasW - 40);
  const hasHint = CHALLENGE._retryWithHint && CHALLENGE.teachingData && CHALLENGE.type === 'math';
  const panelH = hasHint ? 440 : 360;
  const panelX = (canvasW - panelW) / 2;
  const panelY = (canvasH - panelH) / 2 - 10;

  // Panel
  ctx.fillStyle = '#1a1a2e';
  roundRect(ctx, panelX, panelY, panelW, panelH, 16);
  ctx.fill();

  ctx.strokeStyle = '#FFD54F';
  ctx.lineWidth = 4;
  roundRect(ctx, panelX, panelY, panelW, panelH, 16);
  ctx.stroke();

  // Difficulty badge
  const bandLabel = MATH_BAND_NAMES[SKILL.math.band] || '?';
  ctx.fillStyle = '#FFD54F';
  roundRect(ctx, panelX + panelW / 2 - 70, panelY - 16, 140, 32, 10);
  ctx.fill();
  ctx.fillStyle = '#1a1a2e';
  ctx.font = 'bold 16px "Segoe UI", system-ui, sans-serif';
  ctx.textAlign = 'center';
  ctx.fillText(`# ${bandLabel}`, panelX + panelW / 2, panelY + 5);

  // Question
  ctx.fillStyle = '#FFF';
  ctx.font = 'bold 30px "Segoe UI", system-ui, sans-serif';
  ctx.textAlign = 'center';
  const qLines = CHALLENGE.question.split('\n');
  qLines.forEach((line, i) => {
    ctx.fillText(line, panelX + panelW / 2, panelY + 60 + i * 38);
  });

  // Visual hint (if retrying after teaching)
  let hintOffset = 0;
  if (hasHint) {
    const td = CHALLENGE.teachingData;
    hintOffset = 70;
    renderDotVisual(ctx, panelX + panelW / 2, panelY + 80 + qLines.length * 38, td.a, td.b, td.op, td.answer, time);
  }

  // Choice buttons
  const btnW = Math.min(160, (panelW - 80) / 3);
  const btnH = 70;
  const btnY = panelY + 130 + (qLines.length - 1) * 38 + hintOffset;
  const totalBtnW = btnW * 3 + 20 * 2;
  const btnStartX = panelX + (panelW - totalBtnW) / 2;

  CHALLENGE.choices.forEach((choice, i) => {
    const bx = btnStartX + i * (btnW + 20);
    const by = btnY;

    let btnColor = '#2196F3';
    if (CHALLENGE.answered) {
      if (choice.correct) btnColor = '#4CAF50';
      else if (i === CHALLENGE.selectedIndex) btnColor = '#F44336';
    } else if (CHALLENGE.selectedIndex === i) {
      btnColor = '#F44336';
    }

    ctx.fillStyle = btnColor;
    roundRect(ctx, bx, by, btnW, btnH, 12);
    ctx.fill();

    ctx.strokeStyle = 'rgba(255,255,255,0.3)';
    ctx.lineWidth = 2;
    roundRect(ctx, bx, by, btnW, btnH, 12);
    ctx.stroke();

    ctx.fillStyle = '#FFF';
    ctx.font = 'bold 28px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(choice.text, bx + btnW / 2, by + btnH / 2 + 10);

    choice._bounds = { x: bx, y: by, w: btnW, h: btnH };
  });

  // Result / feedback
  if (CHALLENGE.answered && CHALLENGE.wasCorrect) {
    ctx.font = 'bold 32px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillStyle = '#FFD54F';

    const praises = ['AMAZING!', 'WOW!', 'GENIUS!', 'SO SMART!', 'INCREDIBLE!', 'YOU GOT IT!'];
    const praise = praises[Math.floor(CHALLENGE.celebrationStart * 10) % praises.length];
    ctx.fillText(praise, panelX + panelW / 2, btnY + btnH + 45);
    drawStarBurst(ctx, panelX + panelW / 2, btnY + btnH + 25, time, CHALLENGE.celebrationStart, 2);

    ctx.font = '16px "Segoe UI", system-ui, sans-serif';
    ctx.fillStyle = '#AAA';
    ctx.fillText('Press SPACE to continue', panelX + panelW / 2, btnY + btnH + 75);
  } else if (CHALLENGE.attempts > 0 && !CHALLENGE.showTeaching) {
    ctx.font = 'bold 24px "Segoe UI", system-ui, sans-serif';
    ctx.fillStyle = '#FF8A65';
    ctx.textAlign = 'center';
    if (CHALLENGE._retryWithHint) {
      ctx.fillText('Try again! Count the dots!', panelX + panelW / 2, btnY + btnH + 45);
    } else {
      ctx.fillText('Hmm, not quite! Try again!', panelX + panelW / 2, btnY + btnH + 45);
    }
  }
}

// ─── VISUAL DOT TEACHING ─────────────────────────────────
// Shows dots/stars to visually represent the math problem

function renderDotVisual(ctx, cx, cy, a, b, op, answer, time) {
  const dotSize = 10;
  const gap = 4;

  if (op === '+') {
    // Group A dots (blue) + Group B dots (yellow)
    const totalDots = a + b;
    const dotsPerRow = Math.min(totalDots, 10);
    const startX = cx - (dotsPerRow * (dotSize + gap)) / 2;

    let dotIndex = 0;
    for (let i = 0; i < a; i++) {
      const row = Math.floor(dotIndex / 10);
      const col = dotIndex % 10;
      ctx.fillStyle = '#42A5F5';
      ctx.beginPath();
      ctx.arc(startX + col * (dotSize + gap) + dotSize / 2, cy + row * (dotSize + gap * 2), dotSize / 2, 0, Math.PI * 2);
      ctx.fill();
      dotIndex++;
    }
    // Plus sign
    const plusX = startX + a * (dotSize + gap) - gap / 2;
    // Group B dots
    for (let i = 0; i < b; i++) {
      const row = Math.floor(dotIndex / 10);
      const col = dotIndex % 10;
      ctx.fillStyle = '#FFD54F';
      ctx.beginPath();
      ctx.arc(startX + col * (dotSize + gap) + dotSize / 2, cy + row * (dotSize + gap * 2), dotSize / 2, 0, Math.PI * 2);
      ctx.fill();
      dotIndex++;
    }

    // Label
    ctx.font = '16px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillStyle = '#42A5F5';
    ctx.fillText(`${a}`, cx - 40, cy + 35);
    ctx.fillStyle = '#AAA';
    ctx.fillText('+', cx, cy + 35);
    ctx.fillStyle = '#FFD54F';
    ctx.fillText(`${b}`, cx + 40, cy + 35);
  } else if (op === '-') {
    // Show A dots, with B of them crossed out
    const dotsPerRow = Math.min(a, 10);
    const startX = cx - (dotsPerRow * (dotSize + gap)) / 2;

    for (let i = 0; i < a; i++) {
      const row = Math.floor(i / 10);
      const col = i % 10;
      const dx = startX + col * (dotSize + gap) + dotSize / 2;
      const dy = cy + row * (dotSize + gap * 2);

      if (i >= a - b) {
        // These get "taken away"
        ctx.fillStyle = 'rgba(244, 67, 54, 0.4)';
        ctx.beginPath();
        ctx.arc(dx, dy, dotSize / 2, 0, Math.PI * 2);
        ctx.fill();
        // X mark
        ctx.strokeStyle = '#F44336';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(dx - 4, dy - 4);
        ctx.lineTo(dx + 4, dy + 4);
        ctx.moveTo(dx + 4, dy - 4);
        ctx.lineTo(dx - 4, dy + 4);
        ctx.stroke();
      } else {
        ctx.fillStyle = '#42A5F5';
        ctx.beginPath();
        ctx.arc(dx, dy, dotSize / 2, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    ctx.font = '16px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillStyle = '#42A5F5';
    ctx.fillText(`${a} - ${b} = count the blue ones!`, cx, cy + 35);
  } else if (op === '×') {
    // Show groups: a groups of b dots
    const groupGap = 30;
    const totalW = a * (b * (dotSize + gap) + groupGap) - groupGap;
    let startX = cx - totalW / 2;

    for (let g = 0; g < a; g++) {
      const colors = ['#42A5F5', '#FFD54F'];
      for (let d = 0; d < b; d++) {
        const dx = startX + d * (dotSize + gap) + dotSize / 2;
        ctx.fillStyle = colors[g % 2];
        ctx.beginPath();
        ctx.arc(dx, cy, dotSize / 2, 0, Math.PI * 2);
        ctx.fill();
      }
      startX += b * (dotSize + gap) + groupGap;
    }

    ctx.font = '16px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillStyle = '#AAA';
    ctx.fillText(`${a} group${a > 1 ? 's' : ''} of ${b}`, cx, cy + 30);
  }
}

function renderTeaching(ctx, canvasW, canvasH, time) {
  const panelW = Math.min(650, canvasW - 40);
  const panelH = 400;
  const panelX = (canvasW - panelW) / 2;
  const panelY = (canvasH - panelH) / 2;

  ctx.fillStyle = '#1a1a2e';
  roundRect(ctx, panelX, panelY, panelW, panelH, 16);
  ctx.fill();

  ctx.strokeStyle = '#FF8A65';
  ctx.lineWidth = 4;
  roundRect(ctx, panelX, panelY, panelW, panelH, 16);
  ctx.stroke();

  // Header
  ctx.fillStyle = '#FF8A65';
  roundRect(ctx, panelX + panelW / 2 - 80, panelY - 16, 160, 32, 10);
  ctx.fill();
  ctx.fillStyle = '#1a1a2e';
  ctx.font = 'bold 18px "Segoe UI", system-ui, sans-serif';
  ctx.textAlign = 'center';
  ctx.fillText("Let's Figure It Out!", panelX + panelW / 2, panelY + 5);

  if (CHALLENGE.type === 'math' && CHALLENGE.teachingData) {
    const td = CHALLENGE.teachingData;

    // Show the question again
    ctx.fillStyle = '#FFF';
    ctx.font = 'bold 28px "Segoe UI", system-ui, sans-serif';
    ctx.fillText(CHALLENGE.question, panelX + panelW / 2, panelY + 55);

    // Visual dot representation
    renderDotVisual(ctx, panelX + panelW / 2, panelY + 130, td.a, td.b, td.op, td.answer, time);

    // Animated count
    const elapsed = time % 10;
    const countSpeed = 0.5; // seconds per dot
    const countUpTo = Math.min(td.answer, Math.floor(elapsed / countSpeed) + 1);

    // Show the count so far
    ctx.fillStyle = '#00E676';
    ctx.font = 'bold 48px "Segoe UI", system-ui, sans-serif';
    ctx.fillText(countUpTo >= td.answer ? `= ${td.answer}` : `counting... ${countUpTo}`, panelX + panelW / 2, panelY + 250);

    if (countUpTo >= td.answer) {
      ctx.fillStyle = '#FFD54F';
      ctx.font = 'bold 24px "Segoe UI", system-ui, sans-serif';
      ctx.fillText(`The answer is ${td.answer}!`, panelX + panelW / 2, panelY + 300);
    }
  }

  // Dismiss prompt
  ctx.fillStyle = '#AAA';
  ctx.font = '16px "Segoe UI", system-ui, sans-serif';
  ctx.textAlign = 'center';
  ctx.fillText('Press SPACE or click to try again!', panelX + panelW / 2, panelY + panelH - 25);
}

// ─── DUM DUM UI ──────────────────────────────────────────

function awardDumDum(time) {
  DUM_DUMS++;
  DUM_DUM_FLASH = time;
}

function renderDumDumCounter(ctx, canvasW, time) {
  const x = canvasW - 80;
  const y = 20;

  ctx.fillStyle = 'rgba(20, 20, 40, 0.8)';
  roundRect(ctx, x - 10, y - 10, 80, 44, 12);
  ctx.fill();

  drawDumDum(ctx, x + 10, y + 10, 18);

  const flash = (time - DUM_DUM_FLASH < 0.5) ? 1.3 : 1;
  ctx.fillStyle = '#FF5252';
  ctx.font = `bold ${Math.floor(22 * flash)}px "Segoe UI", system-ui, sans-serif`;
  ctx.textAlign = 'left';
  ctx.fillText(`x${DUM_DUMS}`, x + 26, y + 18);
}

// ─── SKILL LEVEL DISPLAY ─────────────────────────────────

function renderSkillBadges(ctx, canvasW) {
  const y = 52;

  // Math level
  ctx.fillStyle = 'rgba(20, 20, 40, 0.7)';
  roundRect(ctx, 10, y, 170, 24, 6);
  ctx.fill();

  ctx.font = '13px "Segoe UI", system-ui, sans-serif';
  ctx.textAlign = 'left';
  ctx.fillStyle = '#FFD54F';
  ctx.fillText(`# ${MATH_BAND_NAMES[SKILL.math.band] || '?'}`, 18, y + 16);

  // Streak dots
  const streakX = 110;
  for (let i = 0; i < 3; i++) {
    ctx.fillStyle = i < Math.abs(SKILL.math.streak) ? (SKILL.math.streak > 0 ? '#4CAF50' : '#F44336') : '#333';
    ctx.beginPath();
    ctx.arc(streakX + i * 14, y + 12, 4, 0, Math.PI * 2);
    ctx.fill();
  }

}

// ─── INTERACTION ORCHESTRATOR ────────────────────────────

async function triggerInteraction(target, playerName, time) {
  if (DIALOGUE.active || CHALLENGE.active) return;

  if (target.type === 'robot') {
    await triggerRobotChat(playerName, time);
  } else if (target.type === 'npc') {
    await triggerNPCChat(target.npc, playerName, time);
  } else if (target.type === 'sign') {
    startDialogue([{
      speaker: 'Sign',
      text: 'Welcome to Robot Village! Explore, make friends, and learn cool stuff!',
    }]);
  } else if (target.type === 'chest') {
    await triggerChestInteraction(playerName, time);
  }
}

async function triggerRobotChat(playerName, time) {
  const doChallenge = Math.random() < 0.5;

  if (doChallenge) {
    const challenge = generateMathChallenge();

    let intro = getPreFetchedLine();
    if (!intro) {
      const ctx = `${playerName} wants to talk. Introduce a math challenge. Be excited! The question will be: "${challenge.question}" — lead into it naturally but do NOT answer it.`;
      intro = await fetchRobotDialogue(ctx);
    }
    if (!intro) intro = "BEEP BOOP! My math sensors are going CRAZY! Quick, help me solve this!";

    startDialogue([{ speaker: 'Sparky', text: intro }], () => {
      startChallenge(challenge, (correct) => {
        if (correct) {
          startDialogue([{
            speaker: 'Sparky',
            text: `WOW ${playerName}! You are SO SMART! My circuits are exploding with happiness!`,
          }]);
        } else {
          awardDumDum(time);
          startDialogue([{
            speaker: 'Sparky',
            text: `Ohhh I got confused! Sorry boss! Mmmmm this Dum Dum is SO GOOD! *crunch crunch* Now I know the answer!`,
          }]);
        }
      });
    });
  } else {
    let line = getPreFetchedLine();
    if (!line) {
      const area = getAreaName(PLAYER.tileX, PLAYER.tileY);
      const ctx = `${playerName} is talking to you in the ${area}. Dum Dum count: ${DUM_DUMS}. Say something fun, silly, or interesting about the area.`;
      line = await fetchRobotDialogue(ctx);
    }
    if (!line) line = getRandomFallbackLine('robot');
    startDialogue([{ speaker: 'Sparky', text: line }]);
  }

  prefetchDialogue(playerName);
}

async function triggerNPCChat(npc, playerName, time) {
  const doChallenge = Math.random() < 0.4;

  if (doChallenge) {
    const challenge = generateMathChallenge();

    let intro = null;
    if (API_KEY) {
      const ctx = `You are ${npc.name}. ${npc.dialogueContext} Talk to ${playerName} and introduce a math challenge. The question is: "${challenge.question}" — lead into it but do NOT answer it. Stay in character. 2-3 short sentences.`;
      intro = await fetchRobotDialogue(ctx);
    }
    if (!intro) intro = `Aha, ${playerName}! I have a challenge for you! Let's see how smart you are!`;

    startDialogue([{ speaker: npc.name, text: intro }], () => {
      startChallenge(challenge, (correct) => {
        if (correct) {
          startDialogue([{
            speaker: npc.name,
            text: `Incredible, ${playerName}! You got it!`,
          }]);
        } else {
          awardDumDum(time);
          startDialogue([
            { speaker: npc.name, text: `Oh no! Even Sparky got confused! Let's keep practicing!` },
            { speaker: 'Sparky', text: `Mmmmm Dum Dum! Thanks boss! *happy robot noises*` },
          ]);
        }
      });
    });
  } else {
    let line = null;
    if (API_KEY) {
      const ctx = `You are ${npc.name}. ${npc.dialogueContext} Say something fun to ${playerName}. 2-3 short sentences. Stay in character.`;
      line = await fetchRobotDialogue(ctx);
    }
    if (!line) line = getRandomFallbackLine(npc.id);
    startDialogue([{ speaker: npc.name, text: line }]);
  }
}

async function triggerChestInteraction(playerName, time) {
  const challenge = generateMathChallenge();

  startDialogue([{
    speaker: 'Sparky',
    text: `OOOOH a treasure chest! But it has a LOCK! We need to solve the puzzle to open it!`,
  }], () => {
    startChallenge(challenge, (correct) => {
      if (correct) {
        awardDumDum(time);
        startDialogue([
          { speaker: 'Sparky', text: `YOU OPENED IT! There's a Dum Dum inside! For ME?! You're the BEST BOSS EVER!!!` },
        ]);
      } else {
        startDialogue([{
          speaker: 'Sparky',
          text: `The chest didn't open this time... But we learned something! We'll get it next time, boss!`,
        }]);
      }
    });
  });
}

// ─── HELPERS ─────────────────────────────────────────────

function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
  ctx.closePath();
}

function wrapText(ctx, text, x, y, maxWidth, lineHeight) {
  const words = text.split(' ');
  let line = '';
  let ly = y;
  for (const word of words) {
    const test = line + word + ' ';
    if (ctx.measureText(test).width > maxWidth && line) {
      ctx.fillText(line.trim(), x, ly);
      line = word + ' ';
      ly += lineHeight;
    } else {
      line = test;
    }
  }
  ctx.fillText(line.trim(), x, ly);
}
