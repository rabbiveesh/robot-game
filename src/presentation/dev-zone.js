// dev-zone.js — Developer debug page
// Accessed by naming a save file "justinbailey"
// Scrollable gallery of every visual component with live controls.

let _devZoneScroll = 0;
let _devZoneControls = {
  op: '+', a: 47, b: 28, band: 6,
  visualMethod: 'base10_blocks',
  phase: 'presented',
};

const DEV_OPS = ['+', '-', '\u00d7', '\u00f7'];
const DEV_OP_NAMES = { '+': 'add', '-': 'sub', '\u00d7': 'multiply', '\u00f7': 'divide' };

function devCompute(a, b, op) {
  switch (op) {
    case '+': return a + b;
    case '-': return a - b;
    case '\u00d7': return a * b;
    case '\u00f7': return Math.floor(a / b) || 1;
    default: return a + b;
  }
}

function initDevZone() {
  _devZoneScroll = 0;
  window.addEventListener('wheel', (e) => {
    if (GAME.state === 'DEV_ZONE') {
      _devZoneScroll = Math.max(0, _devZoneScroll + e.deltaY);
    }
  });
  window.addEventListener('keydown', (e) => {
    if (GAME.state !== 'DEV_ZONE') return;
    if (e.key === 'Escape') {
      GAME.state = 'TITLE';
      document.getElementById('titleScreen').style.display = '';
      document.getElementById('gameCanvas').style.display = 'none';
    }
    if (e.key === 'ArrowDown') _devZoneScroll += 60;
    if (e.key === 'ArrowUp') _devZoneScroll = Math.max(0, _devZoneScroll - 60);
  });
}

function renderDevZone(ctx, canvasW, canvasH, time) {
  ctx.fillStyle = '#0a0a1a';
  ctx.fillRect(0, 0, canvasW, canvasH);

  const scrollY = _devZoneScroll;
  const lx = 20;
  let y = 30 - scrollY;
  const c = _devZoneControls;

  // ─── HEADER ───────────────────────────────────────
  ctx.fillStyle = '#00E676';
  ctx.font = 'bold 28px monospace';
  ctx.textAlign = 'left';
  ctx.fillText('DEV ZONE', lx, y);
  ctx.fillStyle = '#546E7A';
  ctx.font = '14px monospace';
  ctx.fillText('ESC to exit  |  Scroll or Arrow keys to navigate', lx + 200, y);
  y += 40;

  // ─── VISUALIZATION PLAYGROUND ─────────────────────
  ctx.fillStyle = '#FFD54F';
  ctx.font = 'bold 20px monospace';
  ctx.fillText('VISUALIZATION PLAYGROUND', lx, y);
  y += 30;

  // Operation buttons
  ctx.fillStyle = '#78909C';
  ctx.font = '14px monospace';
  ctx.fillText('Operation:', lx, y);
  DEV_OPS.forEach((op, i) => {
    const bx = lx + 100 + i * 50;
    ctx.fillStyle = c.op === op ? '#00E676' : '#37474F';
    ctx.fillRect(bx, y - 14, 40, 20);
    ctx.fillStyle = c.op === op ? '#000' : '#AAA';
    ctx.font = 'bold 14px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(op, bx + 20, y);
    ctx.textAlign = 'left';
  });
  y += 28;

  // A and B
  ctx.fillStyle = '#78909C';
  ctx.font = '14px monospace';
  ctx.fillText(`A: ${c.a}   B: ${c.b}   Answer: ${devCompute(c.a, c.b, c.op)}`, lx, y);
  ctx.fillText('(click numbers on keyboard to change)', lx + 350, y);
  y += 28;

  // Band buttons
  ctx.fillStyle = '#78909C';
  ctx.font = '14px monospace';
  ctx.fillText('Band:', lx, y);
  for (let band = 1; band <= 10; band++) {
    const bx = lx + 60 + (band - 1) * 36;
    ctx.fillStyle = c.band === band ? '#00E676' : '#37474F';
    ctx.fillRect(bx, y - 14, 30, 20);
    ctx.fillStyle = c.band === band ? '#000' : '#AAA';
    ctx.font = '12px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(String(band), bx + 15, y);
    ctx.textAlign = 'left';
  }
  y += 28;

  // Visual method buttons
  ctx.fillStyle = '#78909C';
  ctx.font = '14px monospace';
  ctx.fillText('Visual:', lx, y);
  const allVisuals = typeof getAllVisuals === 'function' ? getAllVisuals() : [];
  allVisuals.forEach((vis, i) => {
    const bx = lx + 70 + i * 120;
    ctx.fillStyle = c.visualMethod === vis.id ? '#00E676' : '#37474F';
    ctx.fillRect(bx, y - 14, 110, 20);
    ctx.fillStyle = c.visualMethod === vis.id ? '#000' : '#AAA';
    ctx.font = '11px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(vis.name, bx + 55, y);
    ctx.textAlign = 'left';
  });
  y += 35;

  // Live visual render
  ctx.strokeStyle = '#37474F';
  ctx.lineWidth = 1;
  ctx.strokeRect(lx, y, canvasW - 40, 120);
  const answer = devCompute(c.a, c.b, c.op);
  const vis = typeof getVisual === 'function' ? getVisual(c.visualMethod) : null;
  if (vis) {
    vis.render(ctx, c.a, c.b, c.op, answer, canvasW / 2, y + 20, time);
  } else {
    ctx.fillStyle = '#546E7A';
    ctx.font = '16px monospace';
    ctx.textAlign = 'center';
    ctx.fillText('No visual registered for: ' + c.visualMethod, canvasW / 2, y + 60);
    ctx.textAlign = 'left';
  }
  y += 140;

  // ─── ALL VISUALS COMPARISON ───────────────────────
  ctx.fillStyle = '#FFD54F';
  ctx.font = 'bold 20px monospace';
  ctx.fillText(`ALL VISUALS for ${c.a} ${c.op} ${c.b}`, lx, y);
  y += 25;

  const cardW = Math.floor((canvasW - 60) / 3);
  const cardH = 130;
  allVisuals.forEach((vis, i) => {
    const col = i % 3;
    const row = Math.floor(i / 3);
    const cx = lx + col * (cardW + 10);
    const cy = y + row * (cardH + 10);

    ctx.strokeStyle = '#37474F';
    ctx.lineWidth = 1;
    ctx.strokeRect(cx, cy, cardW, cardH);

    ctx.fillStyle = '#90CAF9';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText(vis.name, cx + 5, cy + 14);

    const supportsOp = vis.operations.includes(DEV_OP_NAMES[c.op]);
    if (supportsOp) {
      vis.render(ctx, c.a, c.b, c.op, answer, cx + cardW / 2, cy + 35, time);
    } else {
      ctx.fillStyle = '#37474F';
      ctx.font = '14px monospace';
      ctx.textAlign = 'center';
      ctx.fillText('N/A for ' + c.op, cx + cardW / 2, cy + 70);
      ctx.textAlign = 'left';
    }
  });
  y += Math.ceil(allVisuals.length / 3) * (cardH + 10) + 20;

  // ─── SPRITE GALLERY ───────────────────────────────
  ctx.fillStyle = '#FFD54F';
  ctx.font = 'bold 20px monospace';
  ctx.fillText('SPRITE GALLERY', lx, y);
  y += 25;

  // Use SPRITE_FNS from characters.js (the game's own sprite map)
  const spriteFns = typeof SPRITE_FNS !== 'undefined' ? SPRITE_FNS : {};
  const spriteNames = Object.keys(spriteFns);
  const spriteScale = 2;

  spriteNames.forEach((name, i) => {
    const sx = lx + (i % 6) * 120;
    const sy = y + Math.floor(i / 6) * 90;

    ctx.save();
    ctx.translate(sx, sy);
    ctx.scale(spriteScale, spriteScale);
    try {
      spriteFns[name](ctx, 0, 0, DIR.down, 0, time);
    } catch (e) { /* skip broken sprites */ }
    ctx.restore();

    ctx.fillStyle = '#AAA';
    ctx.font = '11px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(name, sx + 24, sy + 75);
    ctx.textAlign = 'left';
  });
  y += Math.ceil(spriteNames.length / 6) * 90 + 20;

  // Also draw player sprites
  const playerSprites = [
    { name: 'player_boy', fn: typeof drawPlayer === 'function' ? drawPlayer : null, gender: 'boy' },
    { name: 'player_girl', fn: typeof drawPlayer === 'function' ? drawPlayer : null, gender: 'girl' },
    { name: 'robot', fn: typeof drawRobot === 'function' ? drawRobot : null },
  ];
  playerSprites.forEach((ps, i) => {
    if (!ps.fn) return;
    const sx = lx + (spriteNames.length % 6 + i) * 120;
    const sy = y - 90 - 20;
    ctx.save();
    ctx.translate(sx, sy);
    ctx.scale(spriteScale, spriteScale);
    try { ps.fn(ctx, 0, 0, DIR.down, 0, time); } catch (e) { }
    ctx.restore();
    ctx.fillStyle = '#AAA';
    ctx.font = '11px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(ps.name, sx + 24, sy + 75);
    ctx.textAlign = 'left';
  });

  // ─── TILE GALLERY ─────────────────────────────────
  ctx.fillStyle = '#FFD54F';
  ctx.font = 'bold 20px monospace';
  ctx.fillText('TILE GALLERY', lx, y);
  y += 25;

  const tileTypes = typeof TILE_TYPES !== 'undefined' ? TILE_TYPES : [];
  tileTypes.forEach((tile, i) => {
    const tx = lx + (i % 10) * 50;
    const ty = y + Math.floor(i / 10) * 60;
    if (typeof renderTile === 'function') {
      try { renderTile(ctx, tx, ty, i, time); } catch (e) { }
    } else {
      ctx.fillStyle = tile.color || '#333';
      ctx.fillRect(tx, ty, 32, 32);
    }
    ctx.fillStyle = '#546E7A';
    ctx.font = '9px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(String(i), tx + 16, ty + 44);
    ctx.textAlign = 'left';
  });
  y += Math.ceil(tileTypes.length / 10) * 60 + 20;

  // ─── TTS TEST ─────────────────────────────────────
  ctx.fillStyle = '#FFD54F';
  ctx.font = 'bold 20px monospace';
  ctx.fillText('TTS TEST', lx, y);
  y += 25;

  const speakers = [
    { name: 'Sparky', text: 'BEEP BOOP! What is 8 times 5?' },
    { name: 'Mommy', text: "You're doing great, sweetie!" },
    { name: 'Professor Gizmo', text: 'My formula needs the missing number!' },
    { name: 'Old Oak', text: 'The leaves whisper your name...' },
    { name: 'B0RK.exe', text: 'BORK BORK! sys.treat.exe loaded!' },
  ];
  speakers.forEach((s, i) => {
    const bx = lx;
    const by = y + i * 30;
    ctx.fillStyle = '#37474F';
    ctx.fillRect(bx, by, 30, 22);
    ctx.fillStyle = '#69F0AE';
    ctx.font = 'bold 14px monospace';
    ctx.textAlign = 'center';
    ctx.fillText('\u25B6', bx + 15, by + 16);
    ctx.textAlign = 'left';

    ctx.fillStyle = '#E0E0E0';
    ctx.font = '13px monospace';
    ctx.fillText(`${s.name}: "${s.text}"`, bx + 40, by + 16);

    // Store bounds for click
    s._bounds = { x: bx, y: by, w: 30, h: 22 };
  });
  window._devTTSSpeakers = speakers;
  y += speakers.length * 30 + 30;

  // Footer
  ctx.fillStyle = '#37474F';
  ctx.font = '12px monospace';
  ctx.fillText('End of Dev Zone. ESC to exit.', lx, y);
}

function handleDevZoneClick(mx, my) {
  const scrollY = _devZoneScroll;
  const adjY = my + scrollY;
  const c = _devZoneControls;

  // Operation buttons (y ≈ 70-scrollY + 14)
  DEV_OPS.forEach((op, i) => {
    const bx = 20 + 100 + i * 50;
    const by = 70;
    if (mx >= bx && mx <= bx + 40 && adjY >= by - 14 && adjY <= by + 6) {
      c.op = op;
    }
  });

  // Band buttons (y ≈ 126)
  for (let band = 1; band <= 10; band++) {
    const bx = 20 + 60 + (band - 1) * 36;
    const by = 126;
    if (mx >= bx && mx <= bx + 30 && adjY >= by - 14 && adjY <= by + 6) {
      c.band = band;
    }
  }

  // Visual method buttons (y ≈ 154)
  const allVisuals = typeof getAllVisuals === 'function' ? getAllVisuals() : [];
  allVisuals.forEach((vis, i) => {
    const bx = 20 + 70 + i * 120;
    const by = 154;
    if (mx >= bx && mx <= bx + 110 && adjY >= by - 14 && adjY <= by + 6) {
      c.visualMethod = vis.id;
    }
  });

  // TTS play buttons
  if (window._devTTSSpeakers) {
    window._devTTSSpeakers.forEach((s) => {
      const b = s._bounds;
      if (b && mx >= b.x && mx <= b.x + b.w && my >= b.y && my <= b.y + b.h) {
        if (typeof speakLine === 'function') speakLine(s.name, s.text);
      }
    });
  }
}

function handleDevZoneKey(key) {
  const c = _devZoneControls;
  // Adjust A and B with number keys + shift
  if (key >= '0' && key <= '9') {
    c.a = parseInt(String(c.a) + key) % 1000;
  }
  if (key === 'Backspace') {
    c.a = Math.floor(c.a / 10);
  }
  if (key === 'Tab') {
    // Swap focus between A and B
    const tmp = c.a;
    c.a = c.b;
    c.b = tmp;
  }
}
