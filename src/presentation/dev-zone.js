// dev-zone.js — Developer debug page
// Accessed by naming a save file "justinbailey"
// Scrollable gallery of every visual component with live controls.

let _devZoneScroll = 0;
let _devZoneControls = {
  op: '+', a: 47, b: 28, band: 6,
  visualMethod: 'base10_blocks',
  phase: 'presented',
};
// All clickable regions stored during render, checked on click
let _devZoneButtons = [];

const DEV_OPS = ['+', '-', '\u00d7', '\u00f7'];
const DEV_OP_NAMES = { '+': 'add', '-': 'sub', '\u00d7': 'multiply', '\u00f7': 'divide' };

function devCompute(a, b, op) {
  switch (op) {
    case '+': return a + b;
    case '-': return a - b;
    case '\u00d7': return a * b;
    case '\u00f7': return b !== 0 ? Math.floor(a / b) : 1;
    default: return a + b;
  }
}

function initDevZone() {
  _devZoneScroll = 0;
  _devZoneButtons = [];
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
    if (typeof handleDevZoneKey === 'function') handleDevZoneKey(e.key);
  });
}

// Helper: register a clickable button during render
function devBtn(ctx, x, y, w, h, label, active, onClick) {
  ctx.fillStyle = active ? '#00E676' : '#37474F';
  ctx.fillRect(x, y, w, h);
  ctx.fillStyle = active ? '#000' : '#AAA';
  ctx.font = 'bold 13px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(label, x + w / 2, y + h / 2 + 4);
  ctx.textAlign = 'left';
  _devZoneButtons.push({ x, y, w, h, onClick });
}

function renderDevZone(ctx, canvasW, canvasH, time) {
  _devZoneButtons = []; // reset clickables each frame
  ctx.fillStyle = '#0a0a1a';
  ctx.fillRect(0, 0, canvasW, canvasH);

  ctx.save();
  ctx.translate(0, -_devZoneScroll);

  const lx = 20;
  let y = 30;
  const c = _devZoneControls;

  // ─── HEADER ───────────────────────────────────────
  ctx.fillStyle = '#00E676';
  ctx.font = 'bold 28px monospace';
  ctx.textAlign = 'left';
  ctx.fillText('DEV ZONE', lx, y);
  ctx.fillStyle = '#546E7A';
  ctx.font = '14px monospace';
  ctx.fillText('ESC exit  |  Scroll/Arrows  |  Click controls', lx + 200, y);
  y += 40;

  // ─── VISUALIZATION PLAYGROUND ─────────────────────
  ctx.fillStyle = '#FFD54F';
  ctx.font = 'bold 20px monospace';
  ctx.fillText('VISUALIZATION PLAYGROUND', lx, y);
  y += 30;

  // Operation buttons
  ctx.fillStyle = '#78909C';
  ctx.font = '14px monospace';
  ctx.fillText('Op:', lx, y + 12);
  DEV_OPS.forEach((op, i) => {
    const bx = lx + 40 + i * 50;
    devBtn(ctx, bx, y, 40, 22, op, c.op === op, () => { c.op = op; });
  });
  y += 32;

  // A and B display + increment/decrement buttons
  ctx.fillStyle = '#78909C';
  ctx.font = '14px monospace';
  ctx.fillText('A:', lx, y + 12);
  devBtn(ctx, lx + 25, y, 30, 22, '-', false, () => { c.a = Math.max(1, c.a - 1); });
  ctx.fillStyle = '#E0E0E0'; ctx.font = 'bold 16px monospace'; ctx.textAlign = 'center';
  ctx.fillText(String(c.a), lx + 75, y + 15); ctx.textAlign = 'left';
  devBtn(ctx, lx + 95, y, 30, 22, '+', false, () => { c.a = Math.min(200, c.a + 1); });

  ctx.fillStyle = '#78909C'; ctx.font = '14px monospace';
  ctx.fillText('B:', lx + 150, y + 12);
  devBtn(ctx, lx + 175, y, 30, 22, '-', false, () => { c.b = Math.max(1, c.b - 1); });
  ctx.fillStyle = '#E0E0E0'; ctx.font = 'bold 16px monospace'; ctx.textAlign = 'center';
  ctx.fillText(String(c.b), lx + 225, y + 15); ctx.textAlign = 'left';
  devBtn(ctx, lx + 245, y, 30, 22, '+', false, () => { c.b = Math.min(200, c.b + 1); });

  const answer = devCompute(c.a, c.b, c.op);
  ctx.fillStyle = '#69F0AE'; ctx.font = 'bold 16px monospace';
  ctx.fillText(`= ${answer}`, lx + 300, y + 15);
  y += 32;

  // Band buttons
  ctx.fillStyle = '#78909C'; ctx.font = '14px monospace';
  ctx.fillText('Band:', lx, y + 12);
  for (let band = 1; band <= 10; band++) {
    devBtn(ctx, lx + 55 + (band - 1) * 36, y, 30, 22, String(band), c.band === band, () => { c.band = band; });
  }
  y += 32;

  // Visual method buttons
  ctx.fillStyle = '#78909C'; ctx.font = '14px monospace';
  ctx.fillText('Visual:', lx, y + 12);
  const allVisuals = typeof getAllVisuals === 'function' ? getAllVisuals() : [];
  allVisuals.forEach((vis, i) => {
    devBtn(ctx, lx + 70 + i * 130, y, 120, 22, vis.name, c.visualMethod === vis.id, () => { c.visualMethod = vis.id; });
  });
  y += 38;

  // Live visual render
  ctx.strokeStyle = '#37474F'; ctx.lineWidth = 1;
  ctx.strokeRect(lx, y, canvasW - 40, 130);
  const vis = typeof getVisual === 'function' ? getVisual(c.visualMethod) : null;
  if (vis) {
    vis.render(ctx, c.a, c.b, c.op, answer, canvasW / 2, y + 20, time);
  } else {
    ctx.fillStyle = '#546E7A'; ctx.font = '16px monospace'; ctx.textAlign = 'center';
    ctx.fillText('No visual: ' + c.visualMethod, canvasW / 2, y + 65);
    ctx.textAlign = 'left';
  }
  y += 145;

  // ─── ALL VISUALS COMPARISON ───────────────────────
  ctx.fillStyle = '#FFD54F'; ctx.font = 'bold 20px monospace';
  ctx.fillText(`ALL VISUALS for ${c.a} ${c.op} ${c.b}`, lx, y);
  y += 25;

  const cardW = Math.floor((canvasW - 60) / Math.min(allVisuals.length, 3));
  const cardH = 130;
  allVisuals.forEach((v, i) => {
    const col = i % 3;
    const row = Math.floor(i / 3);
    const cx = lx + col * (cardW + 10);
    const cy = y + row * (cardH + 10);
    ctx.strokeStyle = '#37474F'; ctx.lineWidth = 1;
    ctx.strokeRect(cx, cy, cardW, cardH);
    ctx.fillStyle = '#90CAF9'; ctx.font = 'bold 12px monospace'; ctx.textAlign = 'left';
    ctx.fillText(v.name + (v.bandRange ? ` (${v.bandRange[0]}-${v.bandRange[1]})` : ''), cx + 5, cy + 14);
    if (v.operations.includes(DEV_OP_NAMES[c.op])) {
      v.render(ctx, c.a, c.b, c.op, answer, cx + cardW / 2, cy + 35, time);
    } else {
      ctx.fillStyle = '#37474F'; ctx.font = '14px monospace'; ctx.textAlign = 'center';
      ctx.fillText('N/A for ' + c.op, cx + cardW / 2, cy + 70);
      ctx.textAlign = 'left';
    }
  });
  y += Math.ceil(allVisuals.length / 3) * (cardH + 10) + 20;

  // ─── SPRITE GALLERY ───────────────────────────────
  ctx.fillStyle = '#FFD54F'; ctx.font = 'bold 20px monospace';
  ctx.fillText('SPRITE GALLERY', lx, y);
  y += 30;

  const spriteFns = typeof SPRITE_FNS !== 'undefined' ? SPRITE_FNS : {};
  const spriteNames = Object.keys(spriteFns);
  spriteNames.forEach((name, i) => {
    const sx = lx + (i % 6) * 130;
    const sy = y + Math.floor(i / 6) * 90;
    ctx.save();
    ctx.translate(sx + 10, sy);
    ctx.scale(2, 2);
    try { spriteFns[name](ctx, 0, 0, DIR.down, 0, time); } catch (e) { }
    ctx.restore();
    ctx.fillStyle = '#AAA'; ctx.font = '11px monospace'; ctx.textAlign = 'center';
    ctx.fillText(name, sx + 35, sy + 78);
    ctx.textAlign = 'left';
  });
  y += Math.ceil(spriteNames.length / 6) * 90 + 20;

  // ─── TTS TEST ─────────────────────────────────────
  ctx.fillStyle = '#FFD54F'; ctx.font = 'bold 20px monospace';
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
    const by = y + i * 30;
    devBtn(ctx, lx, by, 30, 22, '\u25B6', false, () => {
      if (typeof speakLine === 'function') speakLine(s.name, s.text);
    });
    ctx.fillStyle = '#E0E0E0'; ctx.font = '13px monospace'; ctx.textAlign = 'left';
    ctx.fillText(`${s.name}: "${s.text}"`, lx + 40, by + 16);
  });
  y += speakers.length * 30 + 30;

  ctx.fillStyle = '#37474F'; ctx.font = '12px monospace';
  ctx.fillText('End of Dev Zone. ESC to exit.', lx, y);

  ctx.restore(); // undo scroll translate
}

function handleDevZoneClick(mx, my) {
  // Adjust for scroll — buttons were rendered in translated coordinates
  const adjX = mx;
  const adjY = my + _devZoneScroll;

  console.log('[DevZone Click]', {
    raw: { mx: Math.round(mx), my: Math.round(my) },
    adjusted: { adjX: Math.round(adjX), adjY: Math.round(adjY) },
    scroll: Math.round(_devZoneScroll),
    buttonCount: _devZoneButtons.length,
    firstFewButtons: _devZoneButtons.slice(0, 5).map(b => ({
      x: Math.round(b.x), y: Math.round(b.y), w: b.w, h: b.h,
    })),
  });

  for (const btn of _devZoneButtons) {
    if (adjX >= btn.x && adjX <= btn.x + btn.w && adjY >= btn.y && adjY <= btn.y + btn.h) {
      console.log('[DevZone Hit]', { x: btn.x, y: btn.y, w: btn.w, h: btn.h });
      btn.onClick();
      return;
    }
  }
  console.log('[DevZone Miss] no button hit');
}

function handleDevZoneKey(key) {
  // Reserved for future keyboard shortcuts
}
