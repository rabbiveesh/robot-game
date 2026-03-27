// game.js — Game loop, state machine, input handling

// ─── GAME STATE ──────────────────────────────────────────

const GAME = {
  state: 'TITLE',   // TITLE | PLAYING | DIALOGUE | CHALLENGE
  playerName: '',
  canvas: null,
  ctx: null,
  canvasW: 960,
  canvasH: 720,
  keys: {},
  lastTime: 0,
  time: 0,           // accumulated game time in seconds
  effects: [],       // active visual effects
};

// ─── INPUT ───────────────────────────────────────────────

function initInput() {
  window.addEventListener('keydown', (e) => {
    GAME.keys[e.key] = true;

    if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault();
      if (GAME.state === 'INTERACTION_MENU') {
        dismissMenu();
      } else if (GAME.state === 'PLAYING') {
        handleInteract();
      } else if (GAME.state === 'CHALLENGE') {
        if (CHALLENGE.showTeaching) {
          dismissTeaching(GAME.time);
        } else if (CHALLENGE.answered) {
          dismissChallenge();
          if (DIALOGUE.active) {
            GAME.state = 'DIALOGUE';
          } else if (CHALLENGE.active) {
            GAME.state = 'CHALLENGE';
          } else {
            GAME.state = 'PLAYING';
          }
        }
      } else if (GAME.state === 'DIALOGUE') {
        advanceDialogue();
        // If dialogue ended, onComplete may have started challenge or new dialogue
        if (!DIALOGUE.active) {
          if (CHALLENGE.active) {
            GAME.state = 'CHALLENGE';
          } else if (DIALOGUE.active) {
            // onComplete restarted dialogue
            GAME.state = 'DIALOGUE';
          } else {
            GAME.state = 'PLAYING';
          }
        }
      }
    }

    // Toggle TTS at runtime
    if (e.key === 't' || e.key === 'T') {
      if (GAME.state !== 'TITLE' && GAME.state !== 'SETTINGS') {
        TTS_ENABLED = !TTS_ENABLED;
        if (!TTS_ENABLED) stopSpeech();
      }
    }

    // Number keys for interaction menu
    if (GAME.state === 'INTERACTION_MENU') {
      const num = parseInt(e.key);
      if (num >= 1 && num <= 3) {
        selectMenuOption(num - 1);
      }
    }

    // ESC dismisses interaction menu (settings handled by index.html overlay)
    if (e.key === 'Escape') {
      if (GAME.state === 'INTERACTION_MENU') {
        dismissMenu();
      }
    }

    // Number keys for challenge choices
    if (GAME.state === 'CHALLENGE' && !CHALLENGE.answered && !CHALLENGE.showTeaching) {
      const num = parseInt(e.key);
      if (num >= 1 && num <= 3) {
        selectChallengeChoice(num - 1, GAME.time);
      }
    }
  });

  window.addEventListener('keyup', (e) => {
    GAME.keys[e.key] = false;
  });

  // Mouse/touch for challenge buttons and interaction menu
  function handlePointer(e) {
    const rect = GAME.canvas.getBoundingClientRect();
    const scaleX = GAME.canvasW / rect.width;
    const scaleY = GAME.canvasH / rect.height;
    const px = (e.clientX - rect.left) * scaleX;
    const py = (e.clientY - rect.top) * scaleY;

    // Interaction menu clicks
    if (GAME.state === 'INTERACTION_MENU' && INTERACTION_MENU.active) {
      for (let i = 0; i < INTERACTION_MENU.options.length; i++) {
        const b = INTERACTION_MENU.options[i]._bounds;
        if (b && px >= b.x && px <= b.x + b.w && py >= b.y && py <= b.y + b.h) {
          selectMenuOption(i);
          return;
        }
      }
      dismissMenu();
      return;
    }

    if (GAME.state !== 'CHALLENGE') return;
    const mx = px;
    const my = py;

    if (CHALLENGE.showTeaching) {
      dismissTeaching(GAME.time);
      return;
    }

    // Scaffold buttons (show-me / tell-me)
    if (window._showMeBounds) {
      const b = window._showMeBounds;
      if (mx >= b.x && mx <= b.x + b.w && my >= b.y && my <= b.y + b.h) {
        if (typeof window._onShowMe === 'function') window._onShowMe();
        return;
      }
    }
    if (window._tellMeBounds) {
      const b = window._tellMeBounds;
      if (mx >= b.x && mx <= b.x + b.w && my >= b.y && my <= b.y + b.h) {
        if (typeof window._onTellMe === 'function') window._onTellMe();
        return;
      }
    }

    if (CHALLENGE.answered) {
      dismissChallenge();
      if (DIALOGUE.active) {
        GAME.state = 'DIALOGUE';
      } else if (CHALLENGE.active) {
        GAME.state = 'CHALLENGE';
      } else {
        GAME.state = 'PLAYING';
      }
      return;
    }

    handleChallengeClick(mx, my, GAME.time);
  }

  window.addEventListener('click', handlePointer);
  window.addEventListener('touchend', (e) => {
    if (e.changedTouches.length > 0) {
      handlePointer({ clientX: e.changedTouches[0].clientX, clientY: e.changedTouches[0].clientY });
    }
  });
}

// ─── INTERACTION HANDLER ─────────────────────────────────

let interacting = false;

async function handleInteract() {
  if (interacting) return;
  const target = getInteractTarget();
  if (!target) return;

  interacting = true;
  GAME.state = 'DIALOGUE';

  await triggerInteraction(target, GAME.playerName, GAME.time);

  // triggerInteraction starts dialogue/challenge — state transitions
  // are handled in the keydown handler when dialogue/challenge ends.
  // Just reset the interaction lock after a tick so new interactions can happen.
  setTimeout(() => { interacting = false; }, 200);
}

// ─── GAME LOOP ───────────────────────────────────────────

function gameLoop(timestamp) {
  const dt = Math.min((timestamp - GAME.lastTime) / 1000, 0.1); // cap delta
  GAME.lastTime = timestamp;
  GAME.time += dt;

  update(dt);
  render();

  requestAnimationFrame(gameLoop);
}

function update(dt) {
  if (GAME.state === 'PLAYING') {
    updatePlayer(dt, GAME.keys);
    updateRobot(dt);
    updateCamera(PLAYER.pixelX, PLAYER.pixelY, GAME.canvasW, GAME.canvasH);
  }

  if (GAME.state === 'DIALOGUE' || GAME.state === 'CHALLENGE') {
    updateDialogue(dt);
    // Keep updating visuals
    updateRobot(dt);
  }

  // Clean up effects
  GAME.effects = GAME.effects.filter(e => GAME.time - e.start < e.duration);
}

function render() {
  const ctx = GAME.ctx;
  const w = GAME.canvasW;
  const h = GAME.canvasH;

  // Clear
  ctx.fillStyle = '#1a1a2e';
  ctx.fillRect(0, 0, w, h);

  if (GAME.state === 'TITLE') {
    renderTitle(ctx, w, h);
    return;
  }

  // Game world
  renderMap(ctx, w, h, GAME.time);

  // Characters — render in Y-order for overlap
  const renderables = [
    { y: PLAYER.pixelY, render: () => renderPlayer(ctx, GAME.time) },
    { y: ROBOT.pixelY,  render: () => renderRobot(ctx, GAME.time) },
    ...NPCS.map(npc => ({
      y: npc.tileY * TILE_SIZE,
      render: () => {
        const screenX = npc.tileX * TILE_SIZE - CAMERA.x;
        const screenY = npc.tileY * TILE_SIZE - CAMERA.y;
        const fn = SPRITE_FNS[npc.spriteFn];
        if (fn) fn(ctx, screenX, screenY, npc.dir, npc.frame, GAME.time);
      }
    })),
  ];
  renderables.sort((a, b) => a.y - b.y);
  renderables.forEach(r => r.render());

  // Area name indicator
  renderAreaName(ctx, w);

  // Dum Dum counter
  renderDumDumCounter(ctx, w, GAME.time);

  // Skill level badges
  renderSkillBadges(ctx, w);

  // Effects
  for (const effect of GAME.effects) {
    drawStarBurst(ctx, effect.x, effect.y, GAME.time, effect.start, effect.duration);
  }

  // UI overlays
  if (DIALOGUE.active) {
    renderDialogue(ctx, w, h, GAME.time);
  }

  if (CHALLENGE.active) {
    renderChallenge(ctx, w, h, GAME.time);
  }

  if (INTERACTION_MENU.active) {
    renderInteractionMenu(ctx, w, h);
  }
}

// ─── TITLE SCREEN ────────────────────────────────────────

function renderTitle(ctx, w, h) {
  // Background gradient
  const grad = ctx.createLinearGradient(0, 0, 0, h);
  grad.addColorStop(0, '#1a1a2e');
  grad.addColorStop(1, '#16213e');
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, w, h);

  // Stars
  for (let i = 0; i < 50; i++) {
    const sx = seededRandom(i, 0, 42) * w;
    const sy = seededRandom(i, 1, 42) * h * 0.6;
    const twinkle = Math.sin(GAME.time * 2 + i) * 0.3 + 0.7;
    ctx.fillStyle = `rgba(255, 255, 255, ${twinkle})`;
    ctx.beginPath();
    ctx.arc(sx, sy, 1.5, 0, Math.PI * 2);
    ctx.fill();
  }

  // Robot mascot
  drawRobot(ctx, w / 2 - TILE_SIZE / 2, 100, DIR.down, 0, GAME.time);

  // Title
  ctx.fillStyle = '#00E676';
  ctx.font = 'bold 56px "Segoe UI", system-ui, sans-serif';
  ctx.textAlign = 'center';
  ctx.fillText('ROBOT BUDDY', w / 2, 230);

  ctx.fillStyle = '#FFD54F';
  ctx.font = 'bold 28px "Segoe UI", system-ui, sans-serif';
  ctx.fillText('ADVENTURE', w / 2, 268);

  // Subtitle
  ctx.fillStyle = '#90CAF9';
  ctx.font = '18px "Segoe UI", system-ui, sans-serif';
  ctx.fillText('A Math RPG', w / 2, 300);
}

function renderAreaName(ctx, canvasW) {
  const area = getAreaName(PLAYER.tileX, PLAYER.tileY);
  ctx.fillStyle = 'rgba(20, 20, 40, 0.7)';
  roundRect(ctx, 10, 10, 180, 30, 8);
  ctx.fill();
  ctx.fillStyle = '#90CAF9';
  ctx.font = '16px "Segoe UI", system-ui, sans-serif';
  ctx.textAlign = 'left';
  ctx.fillText(`📍 ${area}`, 20, 30);
}

// ─── SAVE SYSTEM (3 NES-style slots) ─────────────────────

const SAVE_KEY = 'robotBuddySaves';

function getSaveSlots() {
  try {
    const raw = localStorage.getItem(SAVE_KEY);
    if (raw) return JSON.parse(raw);
  } catch (e) {}
  return [null, null, null];
}

function writeSaveSlots(slots) {
  localStorage.setItem(SAVE_KEY, JSON.stringify(slots));
}

function gatherSaveData() {
  return {
    version: 1,
    name: GAME.playerName,
    apiKey: API_KEY,
    timestamp: Date.now(),
    mapId: MAP.id,
    playerX: PLAYER.tileX,
    playerY: PLAYER.tileY,
    playerDir: PLAYER.dir,
    robotX: ROBOT.tileX,
    robotY: ROBOT.tileY,
    gender: PLAYER_GENDER,
    dumDums: DUM_DUMS,
    skill: {
      math: { band: SKILL.math.band, streak: SKILL.math.streak, totalCorrect: SKILL.math.totalCorrect, totalAttempts: SKILL.math.totalAttempts },
    },
    playTime: GAME.time,
  };
}

function saveToSlot(slotIndex) {
  const slots = getSaveSlots();
  slots[slotIndex] = gatherSaveData();
  writeSaveSlots(slots);
}

function loadFromSlot(slotIndex) {
  const slots = getSaveSlots();
  const data = slots[slotIndex];
  if (!data) return false;

  GAME.playerName = data.name;
  API_KEY = data.apiKey || '';

  // Restore map
  loadMap(data.mapId || 'overworld');
  NPC_DEFS = NPC_DEFS_BY_MAP[data.mapId || 'overworld'] || [];
  initNPCs();

  // Restore player
  PLAYER.tileX = data.playerX;
  PLAYER.tileY = data.playerY;
  PLAYER.pixelX = data.playerX * TILE_SIZE;
  PLAYER.pixelY = data.playerY * TILE_SIZE;
  PLAYER.dir = data.playerDir ?? DIR.down;
  PLAYER.moving = false;

  // Restore robot
  ROBOT.tileX = data.robotX ?? data.playerX;
  ROBOT.tileY = data.robotY ?? data.playerY + 1;
  ROBOT.pixelX = ROBOT.tileX * TILE_SIZE;
  ROBOT.pixelY = ROBOT.tileY * TILE_SIZE;
  ROBOT.followQueue = [];
  ROBOT.moving = false;

  // Restore progress
  PLAYER_GENDER = data.gender || 'boy';
  DUM_DUMS = data.dumDums || 0;
  if (data.skill && data.skill.math) {
    Object.assign(SKILL.math, data.skill.math);
  }
  // Old saves may have data.skill.phonics — safely ignored
  GAME.time = data.playTime || 0;

  return true;
}

function deleteSlot(slotIndex) {
  const slots = getSaveSlots();
  slots[slotIndex] = null;
  writeSaveSlots(slots);
}

function formatPlayTime(seconds) {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatSaveDate(timestamp) {
  const d = new Date(timestamp);
  const mon = d.toLocaleString('en-US', { month: 'short' });
  const day = d.getDate();
  const hour = d.getHours();
  const min = String(d.getMinutes()).padStart(2, '0');
  const ampm = hour >= 12 ? 'PM' : 'AM';
  return `${mon} ${day} ${hour % 12 || 12}:${min}${ampm}`;
}

// Auto-save every 30 seconds during play
let autoSaveInterval = null;
let activeSlot = -1;

function startAutoSave(slotIndex) {
  activeSlot = slotIndex;
  if (autoSaveInterval) clearInterval(autoSaveInterval);
  autoSaveInterval = setInterval(() => {
    if (GAME.state === 'PLAYING' && activeSlot >= 0) {
      saveToSlot(activeSlot);
    }
  }, 30000);
}

// ─── INIT ────────────────────────────────────────────────

function initGame(playerName, apiKey, slotIndex, isLoad, opts) {
  GAME.canvas = document.getElementById('gameCanvas');
  GAME.ctx = GAME.canvas.getContext('2d');
  GAME.canvas.width = GAME.canvasW;
  GAME.canvas.height = GAME.canvasH;
  GAME.ctx.imageSmoothingEnabled = false;

  if (isLoad) {
    loadFromSlot(slotIndex);
  } else {
    GAME.playerName = playerName;
    API_KEY = apiKey || '';
    PLAYER_GENDER = (opts && opts.gender) || 'boy';
    // Set starting levels
    if (opts && opts.mathBand) SKILL.math.band = opts.mathBand;
    loadMap('overworld');
    NPC_DEFS = NPC_DEFS_BY_MAP.overworld;
    initNPCs();
  }

  initInput();

  GAME.state = 'PLAYING';
  GAME.lastTime = performance.now();

  // Start auto-save
  startAutoSave(slotIndex);

  // Save immediately for new games
  if (!isLoad) {
    saveToSlot(slotIndex);
  }

  if (API_KEY) {
    prefetchDialogue(GAME.playerName);
  }

  // Welcome (new game) or welcome back (load)
  setTimeout(() => {
    GAME.state = 'DIALOGUE';
    if (isLoad) {
      startDialogue([
        { speaker: 'Sparky', text: `BEEP BOOP! Welcome back, ${GAME.playerName}! I missed you SO much!` },
        { speaker: 'Sparky', text: `We have ${DUM_DUMS} Dum Dum${DUM_DUMS !== 1 ? 's' : ''}! Let's go find more!` },
      ], () => { GAME.state = 'PLAYING'; });
    } else {
      startDialogue([
        { speaker: 'Sparky', text: `BEEP BOOP! Hi ${GAME.playerName}! I'm Sparky, your robot buddy!` },
        { speaker: 'Sparky', text: `Use the ARROW KEYS to walk around! Press SPACE to talk to people!` },
        { speaker: 'Sparky', text: `Let's go explore Robot Village! I heard there are TREASURE CHESTS! And maybe even Dum Dums!` },
      ], () => { GAME.state = 'PLAYING'; });
    }
  }, 500);

  requestAnimationFrame(gameLoop);
}
