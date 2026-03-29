// adapter.js — Bridge between the domain (WASM or JS fallback) and the legacy game.
// WASM loads async via wasm-bridge.js. Adapter picks it up when available.

(function () {
  // Domain is WASM — no fallback
  const W = window.WasmDomain;

  function createProfile(overrides) { return W.createProfile(overrides); }
  function learnerReducer(state, event) { return W.learnerReducer(state, event); }
  function generateChallenge(profile, rng) { return W.generateChallenge(profile, rng); }
  function detectFrustration(win, behaviors) { return W.detectFrustration(win, behaviors); }
  function generateIntakeQuestion(band, idx, rng) { return W.generateIntakeQuestion(band, idx, rng); }
  function processIntakeResults(answers, band) { return W.processIntakeResults(answers, band); }
  function nextIntakeBand(band, correct, ceiling) { return W.nextIntakeBand(band, correct, ceiling); }
  function accuracy(win) { return W.accuracy(win); }
  function createWindow(entries) { return W.createWindow(entries); }

  // Challenge lifecycle — createChallengeState builds from challenge + context
  function createChallengeState(challenge, context) {
    // Build the state struct that the Rust challenge_reducer expects
    return {
      phase: 'presented',
      correct_answer: challenge.correctAnswer ?? challenge.correct_answer,
      attempts: 0,
      max_attempts: 2,
      correct: null,
      question: {
        display: challenge.displayText ?? challenge.display_text ?? challenge.question,
        speech: challenge.speechText ?? challenge.speech_text ?? challenge.question,
      },
      feedback: null,
      reward: null,
      render_hint: context.renderHint ?? {
        cra_stage: 'abstract',
        answer_mode: 'choice',
        interaction_type: 'quiz',
      },
      hint_used: false,
      hint_level: 0,
      told_me: false,
      voice: { listening: false, confirming: false, confirm_number: null, retries: 0, text: null },
      // Keep the full challenge + context for the adapter to read
      challenge,
      context,
    };
  }
  function challengeReducer(state, action) {
    // Use WASM reducer
    const result = W.challengeReducer(state, action);
    // Preserve challenge + context (not part of Rust state)
    result.challenge = state.challenge;
    result.context = state.context;
    return result;
  }

  // ─── ADAPTIVE STATE ─────────────────────────────────────

  let profileState = createProfile();
  let eventLog = [];          // current session events
  let previousSessionLogs = []; // up to 5 prior sessions
  let recentBehaviors = [];
  let challengeShownAt = 0; // track when challenge was displayed
  const RESPONSE_TIME_CAP_MS = 30000; // 30 seconds — anything above is treated as "walked away"

  function capResponseTime(rawMs) {
    return rawMs > RESPONSE_TIME_CAP_MS ? null : rawMs;
  }
  let debugOverlayVisible = false;
  let voiceDebugVisible = false;
  let voiceDebugState = {}; // populated by dialogue.js voice handler

  // Expose for save/load and debugging
  window.ADAPTIVE = {
    getProfile() { return profileState; },
    getEventLog() { return eventLog; },
    setProfile(p) { profileState = p; },
    setEventLog(log) { eventLog = log; },
    exportSession() {
      const data = {
        exportDate: new Date().toISOString(),
        playerName: GAME.playerName,
        profile: { ...profileState },
        currentSession: eventLog,
        previousSessions: previousSessionLogs,
        operationStats: { ...profileState.operationStats },
        metadata: {
          gameVersion: '0.1.0',
          totalPlayTime: GAME.time,
          dumDums: DUM_DUMS,
          mapId: MAP.id,
        },
      };
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `robot-buddy-session-${new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)}.json`;
      a.click();
      URL.revokeObjectURL(url);
    },
  };

  // Expose voice debug state for dialogue.js to write into
  window._voiceDebug = voiceDebugState;

  // ─── CHALLENGE STATE MACHINE ────────────────────────────
  // Single source of truth for the challenge lifecycle.
  // Replaces all monkey-patches. Both click and voice go through here.

  let challengeState = null;
  window._challengeState = null; // exposed for renderChallenge to read

  // Generate a challenge and create the lifecycle state
  const _oldGenerateMath = window.generateMathChallenge;
  window.generateMathChallenge = function () {
    const challenge = generateChallenge(profileState, Math.random);
    return challenge; // return the full domain object, not legacy format
  };

  // Start a challenge via the state machine
  window._startChallengeFromDomain = function (challenge, context) {
    challengeShownAt = performance.now();

    // Inject CRA stage from learner profile for this operation
    const op = challenge.operation;
    const craStage = (op && profileState.craStages[op]) || 'concrete';
    const enrichedContext = {
      ...context,
      renderHint: {
        craStage,
        answerMode: 'choice',
        interactionType: context.renderHint?.interactionType || 'quiz',
        ...(context.renderHint || {}),
        craStage, // profile CRA takes precedence over context default
      },
    };

    challengeState = createChallengeState(challenge, enrichedContext);
    window._challengeState = challengeState;

  };

  // Handle an answer (from either button click or voice)
  window._onChallengeAnswer = function (answer, time, answerMode) {
    if (!challengeState || challengeState.phase === 'complete') return;

    const prevPhase = challengeState.phase;
    challengeState = challengeReducer(challengeState, { type: 'ANSWER_SUBMITTED', answer });
    window._challengeState = challengeState;

    // Speak feedback
    if (challengeState.feedback) {
      speakLine(challengeState.context.npcName || 'Sparky', challengeState.feedback.speech);
    }

    // Record EVERY answer to the learning domain — first-attempt wrongs are
    // critical signal ("this sub-skill is hard for this kid")
    const responseTimeMs = capResponseTime(performance.now() - challengeShownAt);
    const ch = challengeState.challenge;
    const voiceResult = challengeState.voice.lastResult;
    const correct = answer === ch.correctAnswer;

    const event = {
      type: 'PUZZLE_ATTEMPTED',
      correct,
      operation: ch.operation,
      subSkill: ch.subSkill || null,
      band: ch.sampledBand || ch.band,
      centerBand: profileState.mathBand,
      responseTimeMs,
      attemptNumber: challengeState.attempts,
      timestamp: Date.now(),
      features: ch.features || null,
      answerMode: answerMode || 'choice',
      hintUsed: challengeState.hintUsed,
      toldMe: challengeState.toldMe,
      craLevelShown: challengeState.renderHint?.craStage || null,
      voiceConfidence: voiceResult?.confidence ?? null,
      voiceHesitationMs: voiceResult?.hesitationMs ?? null,
      voiceSelfCorrected: voiceResult?.selfCorrected ?? null,
      voiceHadFillers: voiceResult?.hadFillerWords ?? null,
      voiceRetries: challengeState.voice.retries,
    };

    profileState = learnerReducer(profileState, event);
    eventLog.push(event);

    SKILL.math.band = profileState.mathBand;
    SKILL.math.streak = profileState.streak;

    // Apply reward on completion
    if (challengeState.reward) {
      DUM_DUMS += challengeState.reward.amount;
      DUM_DUM_FLASH = time;
    }

    checkFrustration();

    // Auto-dismiss after brief visual feedback
    if (challengeState.phase === 'complete' || challengeState.phase === 'teaching') {
      autoDismissChallenge(challengeState.correct, challengeState.phase === 'complete' && challengeState.correct ? 800 : 400);
    }
  };

  // Voice actions go through the challenge reducer
  window._onVoiceAction = function (action) {
    if (!challengeState) return;
    challengeState = challengeReducer(challengeState, action);
    window._challengeState = challengeState;

    // Speak voice text changes
    if (challengeState.voice.text?.speech) {
      speakLine('Sparky', challengeState.voice.text.speech);
    }
  };

  // Auto-dismiss: fire onComplete after brief visual feedback, no Space needed
  function autoDismissChallenge(wasCorrect, delayMs) {
    setTimeout(() => {
      if (CHALLENGE.onComplete) {
        const cb = CHALLENGE.onComplete;
        CHALLENGE.onComplete = null;
        CHALLENGE.active = false;
        challengeState = null;
        window._challengeState = null;
        if (window._activeRenderer) {
          window._activeRenderer.dispose();
          window._activeRenderer = null;
        }
        // Fire callback first — it may start a new dialogue/challenge
        cb(wasCorrect);
        // Set state based on what the callback started
        if (DIALOGUE.active) {
          GAME.state = 'DIALOGUE';
        } else if (CHALLENGE.active) {
          GAME.state = 'CHALLENGE';
        } else {
          GAME.state = 'PLAYING';
        }
      }
    }, delayMs);
  }

  // Teaching complete
  window._onTeachingComplete = function () {
    if (!challengeState) return;
    challengeState = challengeReducer(challengeState, { type: 'TEACHING_COMPLETE' });
    window._challengeState = challengeState;
    autoDismissChallenge(false, 400);
  };

  // Show-me scaffold
  window._onShowMe = function () {
    if (!challengeState) return;
    challengeState = challengeReducer(challengeState, { type: 'SHOW_ME' });
    window._challengeState = challengeState;
  };

  // Tell-me scaffold
  window._onTellMe = function () {
    if (!challengeState) return;
    challengeState = challengeReducer(challengeState, { type: 'TELL_ME' });
    window._challengeState = challengeState;
    if (challengeState.feedback?.speech) {
      speakLine('Sparky', challengeState.feedback.speech);
    }
  };

  function mapOpToOperation(op) {
    if (!op) return 'add';
    if (op === '+') return 'add';
    if (op === '-') return 'sub';
    if (op === '\u00d7' || op === '*') return 'multiply';
    if (op === '\u00f7' || op === '/') return 'divide';
    return 'add';
  }

  // ─── FRUSTRATION DETECTION ──────────────────────────────

  function checkFrustration() {
    const result = detectFrustration(profileState.rollingWindow, recentBehaviors);
    if (result.level === 'high') {
      profileState = learnerReducer(profileState, {
        type: 'FRUSTRATION_DETECTED',
        level: 'high',
      });
      SKILL.math.band = profileState.mathBand;
      SKILL.math.streak = profileState.streak;
    }
  }

  // ─── EXPLORATION EVENTS ────────────────────────────────

  window._onAreaEntered = function (mapId) {
    eventLog.push({ type: 'AREA_ENTERED', mapId, timestamp: Date.now() });
  };

  window._onNpcTalked = function (npcId) {
    eventLog.push({ type: 'NPC_TALKED', npcId, timestamp: Date.now() });
  };

  // ─── BEHAVIOR TRACKING ─────────────────────────────────

  // Detect text skipping (space pressed before text finishes)
  const _oldAdvanceDialogue = window.advanceDialogue;
  if (_oldAdvanceDialogue) {
    window.advanceDialogue = function () {
      // If text is still typing and user advances, that's a skip
      if (DIALOGUE.active && DIALOGUE.charIndex < (DIALOGUE.lines[DIALOGUE.currentLine]?.text?.length || 0)) {
        const behavior = { signal: 'text_skipped', timestamp: Date.now() };
        recentBehaviors.push(behavior);
        profileState = learnerReducer(profileState, { type: 'BEHAVIOR', signal: 'text_skipped' });
      }
      _oldAdvanceDialogue();
    };
  }

  // Detect rapid clicking during challenges
  let lastClickTime = 0;
  const _oldHandleChallengeClick = window.handleChallengeClick;
  if (_oldHandleChallengeClick) {
    window.handleChallengeClick = function (mx, my, time) {
      const now = performance.now();
      if (now - lastClickTime < 300) { // clicks within 300ms = rapid
        const behavior = { signal: 'rapid_clicking', timestamp: Date.now() };
        recentBehaviors.push(behavior);
        profileState = learnerReducer(profileState, { type: 'BEHAVIOR', signal: 'rapid_clicking' });
      }
      lastClickTime = now;
      _oldHandleChallengeClick(mx, my, time);
    };
  }

  // ─── INTAKE QUIZ ────────────────────────────────────────

  window.runIntakeQuiz = function (onComplete) {
    const INTAKE_QUESTIONS = 4;
    // Use the parent's configured band as an anchor for intake
    const configuredBand = profileState.mathBand;
    const startBand = Math.min(configuredBand + 1, 3); // start near configured, cap at 3
    const ceiling = configuredBand + 2; // don't wildly exceed what parent expects
    let currentBand = startBand;
    let questionIndex = 0;
    const answers = [];

    function askNext() {
      if (questionIndex >= INTAKE_QUESTIONS) {
        // Process results — pass configured band so placement is anchored
        const results = processIntakeResults(answers, configuredBand);
        profileState = learnerReducer(profileState, {
          type: 'INTAKE_COMPLETED',
          ...results,
        });
        SKILL.math.band = profileState.mathBand;
        SKILL.math.streak = profileState.streak;

        startDialogue([
          { speaker: 'Sparky', text: 'BZZZT! Calibration complete! I know just how to help you now!' },
          { speaker: 'Sparky', text: "Let's go explore! I heard there are TREASURE CHESTS nearby!" },
        ], onComplete);
        return;
      }

      const challenge = generateIntakeQuestion(currentBand, questionIndex, Math.random);
      const intakeCtx = { source: 'intake', npcName: 'Sparky' };

      const shownAt = performance.now();
      startChallenge(challenge, intakeCtx, function (wasCorrect) {
        const responseTimeMs = performance.now() - shownAt;
        answers.push({
          band: currentBand,
          correct: wasCorrect,
          responseTimeMs,
          skippedText: false,
        });
        currentBand = nextIntakeBand(currentBand, wasCorrect, ceiling);
        questionIndex++;
        // Next question — auto-dismiss provides the delay
        askNext();
      });
      GAME.state = 'CHALLENGE';
    }

    // Intro dialogue
    startDialogue([
      { speaker: 'Sparky', text: "BEEP BOOP! Before we start, let me calibrate my circuits!" },
      { speaker: 'Sparky', text: "I'll ask you a few quick questions. Just do your best!" },
    ], () => {
      askNext();
    });
    GAME.state = 'DIALOGUE';
  };

  // ─── SAVE/LOAD INTEGRATION ──────────────────────────────

  const _oldGatherSave = window.gatherSaveData;
  window.gatherSaveData = function () {
    const data = _oldGatherSave();
    data.learnerProfile = {
      mathBand: profileState.mathBand,
      streak: profileState.streak,
      pace: profileState.pace,
      scaffolding: profileState.scaffolding,
      challengeFreq: profileState.challengeFreq,
      spreadWidth: profileState.spreadWidth,
      promoteThreshold: profileState.promoteThreshold,
      stretchThreshold: profileState.stretchThreshold,
      wrongsBeforeTeach: profileState.wrongsBeforeTeach,
      hintVisibility: profileState.hintVisibility,
      textSpeed: profileState.textSpeed,
      framingStyle: profileState.framingStyle,
      representationStyle: profileState.representationStyle,
      craStages: { ...profileState.craStages },
      intakeCompleted: profileState.intakeCompleted,
      operationStats: JSON.parse(JSON.stringify(profileState.operationStats)),
      rollingWindowEntries: profileState.rollingWindow.entries.map(e => ({ ...e })),
    };
    data.totalGiftsGiven = typeof TOTAL_GIFTS_GIVEN !== 'undefined' ? { ...TOTAL_GIFTS_GIVEN } : {};
    // Store current session (capped at 200 events) + last 5 prior session logs
    data.sessionLogs = [
      ...previousSessionLogs.slice(-5),
      eventLog.slice(-200),
    ].slice(-6); // keep at most 6 (5 prior + current)
    return data;
  };

  const _oldLoadFromSlot = window.loadFromSlot;
  window.loadFromSlot = function (slotIndex) {
    const result = _oldLoadFromSlot(slotIndex);
    // Restore profile from save data
    const slots = getSaveSlots();
    const data = slots[slotIndex];
    if (data && data.learnerProfile) {
      const lp = data.learnerProfile;
      profileState = createProfile({
        ...lp,
        rollingWindow: createWindow(lp.rollingWindowEntries || []),
      });
      SKILL.math.band = profileState.mathBand;
      SKILL.math.streak = profileState.streak;
    } else {
      // Old save without profile — use defaults
      profileState = createProfile({
        mathBand: SKILL.math.band,
      });
    }
    // Restore session logs: previous sessions become history, start fresh current session
    const savedLogs = (data && data.sessionLogs) || [];
    previousSessionLogs = savedLogs.slice(-5);
    eventLog = [];
    recentBehaviors = [];
    // Restore gift tracking
    if (typeof TOTAL_GIFTS_GIVEN !== 'undefined') {
      TOTAL_GIFTS_GIVEN = (data && data.totalGiftsGiven) || {};
    }
    return result;
  };

  // ─── PARENT DEBUG OVERLAY (P KEY) ───────────────────────

  window.addEventListener('keydown', (e) => {
    if (e.key === 'p' || e.key === 'P') {
      if (GAME.state !== 'TITLE') {
        debugOverlayVisible = !debugOverlayVisible;
        if (!debugOverlayVisible) voiceDebugVisible = false;
      }
    }
    if (e.key === 'v' || e.key === 'V') {
      if (debugOverlayVisible) {
        voiceDebugVisible = !voiceDebugVisible;
      }
    }
  });

  // Export button click handler — listen on canvas
  if (typeof document !== 'undefined') {
    document.addEventListener('click', (e) => {
      if (!debugOverlayVisible || !window._exportBtnBounds) return;
      const canvas = GAME.canvas;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const scaleX = GAME.canvasW / rect.width;
      const scaleY = GAME.canvasH / rect.height;
      const mx = (e.clientX - rect.left) * scaleX;
      const my = (e.clientY - rect.top) * scaleY;
      const b = window._exportBtnBounds;
      if (mx >= b.x && mx <= b.x + b.w && my >= b.y && my <= b.y + b.h) {
        ADAPTIVE.exportSession();
      }
    });
  }

  // Inject into render cycle
  const _oldRender = window.render;
  if (_oldRender) {
    window.render = function () {
      _oldRender();
      if (debugOverlayVisible) {
        renderDebugOverlay();
      }
    };
  }

  function renderDebugOverlay() {
    const ctx = GAME.ctx;
    if (!ctx) return;

    const x = GAME.canvasW - 320;
    const y = 10;
    const w = 310;
    const h = 280;

    // Background
    ctx.fillStyle = 'rgba(0, 0, 0, 0.85)';
    ctx.fillRect(x, y, w, h);
    ctx.strokeStyle = '#00E676';
    ctx.lineWidth = 2;
    ctx.strokeRect(x, y, w, h);

    ctx.font = '12px monospace';
    ctx.textAlign = 'left';
    let ly = y + 20;
    const lx = x + 10;
    const lineH = 16;

    function line(text, color = '#E0E0E0') {
      ctx.fillStyle = color;
      ctx.fillText(text, lx, ly);
      ly += lineH;
    }

    const BAND_NAMES = ['', 'Add <5', '+/- <10', '+/- <15', '+/- <20', 'x1 x2',
      '+/- <50', '+/- <100', 'x1-5', 'x1-12', 'Divide'];
    const p = profileState;
    const win = p.rollingWindow;
    const acc = win.entries.length > 0 ? accuracy(win) : 0;
    const correctCount = win.entries.filter(e => e.correct).length;

    line('-- Learner Profile --', '#00E676');
    line(`Band: ${p.mathBand} (${BAND_NAMES[p.mathBand] || '?'})   Streak: ${p.streak}/${p.streakToPromote}`, '#FFD54F');
    line(`Pace: ${p.pace.toFixed(2)}   Scaffolding: ${p.scaffolding.toFixed(2)}`);

    // Frustration status
    const frust = detectFrustration(win, recentBehaviors);
    const frustColor = frust.level === 'high' ? '#F44336' : frust.level === 'mild' ? '#FFC107' : '#4CAF50';
    line(`Frustration: ${frust.level}`, frustColor);

    line(`Rolling accuracy: ${(acc * 100).toFixed(0)}% (${correctCount}/${win.entries.length})`);
    line(`Intake: ${p.intakeCompleted ? 'completed' : 'pending'}`, p.intakeCompleted ? '#4CAF50' : '#78909C');
    line('');

    // Per-operation stats
    const ops = ['add', 'sub', 'multiply', 'divide', 'number_bond'];
    const opLabels = { add: 'add', sub: 'sub', multiply: 'mult', divide: 'div', number_bond: 'bond' };
    for (const op of ops) {
      const s = p.operationStats[op];
      const opAcc = s.attempts > 0 ? `${((s.correct / s.attempts) * 100).toFixed(0)}%` : '--';
      const opDetail = s.attempts > 0 ? `(${s.correct}/${s.attempts})` : '';
      const cra = p.craStages[op] || 'concrete';
      line(`${(opLabels[op] + ':').padEnd(6)} ${opAcc.padStart(4)} ${opDetail.padStart(7)}  CRA: ${cra}`);
    }
    line('');

    // Last 5 events
    line('Last events:', '#90CAF9');
    const recent = eventLog.slice(-5);
    for (const evt of recent) {
      const mark = evt.correct ? '\u2713' : '\u2717';
      const color = evt.correct ? '#4CAF50' : '#F44336';
      const time = evt.responseTimeMs ? `${(evt.responseTimeMs / 1000).toFixed(1)}s` : '?';
      const mode = evt.answerMode === 'voice' ? ' [voice]' : '';
      line(`  ${mark} ${evt.operation || '?'}  band:${evt.band || '?'}  ${time}${mode}`, color);
    }

    // Export button
    const exportBtnX = x + 5;
    const exportBtnY = ly + 5;
    const exportBtnW = 110;
    const exportBtnH = 22;
    ctx.fillStyle = '#37474F';
    ctx.fillRect(exportBtnX, exportBtnY, exportBtnW, exportBtnH);
    ctx.fillStyle = '#90CAF9';
    ctx.font = '11px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('[Export Session]', exportBtnX + 5, exportBtnY + 15);
    // Store bounds for click detection
    window._exportBtnBounds = { x: exportBtnX, y: exportBtnY, w: exportBtnW, h: exportBtnH };

    // Voice debug panel (V key, only when P is active)
    if (voiceDebugVisible) {
      const vx = x;
      const vy = exportBtnY + exportBtnH + 10;
      const vw = w;
      const vh = 160;

      ctx.fillStyle = 'rgba(0, 0, 0, 0.85)';
      ctx.fillRect(vx, vy, vw, vh);
      ctx.strokeStyle = '#7E57C2';
      ctx.lineWidth = 2;
      ctx.strokeRect(vx, vy, vw, vh);

      let vly = vy + 16;
      const vlx = vx + 10;
      function vline(text, color = '#E0E0E0') {
        ctx.fillStyle = color;
        ctx.font = '12px monospace';
        ctx.textAlign = 'left';
        ctx.fillText(text, vlx, vly);
        vly += 15;
      }

      const vd = voiceDebugState;
      vline('-- Voice Debug --', '#7E57C2');
      vline(`Status: ${vd.status || 'idle'}`);
      vline(`Interim: "${vd.interim || ''}"`);
      vline(`Final: "${vd.final || ''}"`);
      vline(`Confidence: ${vd.confidence != null ? vd.confidence.toFixed(2) : '--'}`);
      vline(`Parsed: ${vd.parsed != null ? vd.parsed : '--'}`);
      vline(`Expected: ${vd.expected != null ? vd.expected : '--'}`);
      const matchColor = vd.match === true ? '#4CAF50' : vd.match === false ? '#F44336' : '#78909C';
      vline(`Match: ${vd.match != null ? (vd.match ? 'YES' : 'NO') : '--'}`, matchColor);
      vline(`Hesitation: ${vd.hesitationMs != null ? (vd.hesitationMs / 1000).toFixed(1) + 's' : '--'}`);
      vline(`Fillers: ${vd.fillers ?? '--'}  Self-corrected: ${vd.selfCorrected ?? '--'}`);
    }
  }

  // ─── HOOK INTO initGame FOR INTAKE ──────────────────────

  const _oldInitGame = window.initGame;
  window.initGame = function (playerName, apiKey, slotIndex, isLoad, opts) {
    // Reset adaptive state for new games
    if (!isLoad) {
      const startBand = (opts && opts.mathBand) || 1;
      profileState = createProfile({ mathBand: startBand });
      eventLog = [];
      recentBehaviors = [];
    }

    // For new games, intercept startDialogue BEFORE _oldInitGame fires
    // so we can wrap the welcome dialogue's onComplete with the intake quiz.
    // This avoids a fragile polling loop that could race with fast text.
    if (!isLoad) {
      const _realStartDialogue = window.startDialogue;
      let intercepted = false;
      window.startDialogue = function (lines, onComplete) {
        if (!intercepted) {
          intercepted = true;
          // Restore original immediately so intake and future dialogues work normally
          window.startDialogue = _realStartDialogue;
          // Wrap onComplete: after welcome finishes, run intake
          _realStartDialogue(lines, function () {
            if (onComplete) onComplete();
            runIntakeQuiz(() => {
              GAME.state = 'PLAYING';
            });
          });
        } else {
          _realStartDialogue(lines, onComplete);
        }
      };
    }

    _oldInitGame(playerName, apiKey, slotIndex, isLoad, opts);
  };

  console.log('[Adaptive Engine] Loaded. Press P for parent debug overlay.');
})();
