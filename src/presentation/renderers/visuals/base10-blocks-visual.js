// base10-blocks-visual.js — Tens rods + ones cubes
// For numbers 1-200+. Shows place value visually.
// A long bar = 10, a small square = 1.

// Registration is at end of file (after function definition).

function renderBase10Blocks(ctx, a, b, op, answer, cx, cy, time) {
  const ROD_W = 10;
  const ROD_H = 44;
  const FIVE_H = 22;   // 5-bar: half the rod height
  const CUBE = 10;
  const GAP = 3;
  const COLOR_A = { rod: '#42A5F5', cube: '#64B5F6', five: '#66BB6A' };
  const COLOR_B = { rod: '#FFD54F', cube: '#FFE082', five: '#81C784' };

  // Measure width needed for a number's blocks
  function measureNum(num) {
    const tens = Math.floor(num / 10);
    const ones = num % 10;
    const fives = Math.floor(ones / 5);
    const remainder = ones % 5;
    const rodsW = tens > 0 ? tens * (ROD_W + GAP) : 0;
    const onesW = fives * (ROD_W + GAP) + remainder * (CUBE + GAP);
    return Math.max(rodsW, onesW, 20);
  }

  // Draw one number as blocks: tens rods on top, 5-bars + cubes below
  function drawNum(x, y, num, colors) {
    const tens = Math.floor(num / 10);
    const ones = num % 10;
    const fives = Math.floor(ones / 5);
    const remainder = ones % 5;
    const totalW = measureNum(num);

    // Label centered above
    ctx.fillStyle = '#E0E0E0';
    ctx.font = 'bold 16px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(String(num), x + totalW / 2, y - 6);

    // Tens rods
    for (let i = 0; i < Math.min(tens, 15); i++) {
      const rx = x + i * (ROD_W + GAP);
      ctx.fillStyle = colors.rod;
      ctx.fillRect(rx, y, ROD_W, ROD_H);
      ctx.strokeStyle = 'rgba(0,0,0,0.3)';
      ctx.lineWidth = 1;
      ctx.strokeRect(rx, y, ROD_W, ROD_H);
    }

    // Ones row: 5-bars (green half-rods) + remainder cubes
    const onesY = tens > 0 ? y + ROD_H + 5 : y;
    let onesX = x;

    // 5-bars
    for (let i = 0; i < fives; i++) {
      ctx.fillStyle = colors.five;
      ctx.fillRect(onesX, onesY, ROD_W, FIVE_H);
      ctx.strokeStyle = 'rgba(0,0,0,0.3)';
      ctx.lineWidth = 1;
      ctx.strokeRect(onesX, onesY, ROD_W, FIVE_H);
      onesX += ROD_W + GAP;
    }

    // Remainder cubes
    for (let i = 0; i < remainder; i++) {
      ctx.fillStyle = colors.cube;
      ctx.fillRect(onesX, onesY + (fives > 0 ? (FIVE_H - CUBE) / 2 : 0), CUBE, CUBE);
      ctx.strokeStyle = 'rgba(0,0,0,0.3)';
      ctx.lineWidth = 1;
      ctx.strokeRect(onesX, onesY + (fives > 0 ? (FIVE_H - CUBE) / 2 : 0), CUBE, CUBE);
      onesX += CUBE + GAP;
    }
  }

  // Height of the block area for a number
  function contentHeight(num) {
    const tens = Math.floor(num / 10);
    const ones = num % 10;
    const fives = Math.floor(ones / 5);
    const onesH = fives > 0 ? FIVE_H : (ones > 0 ? CUBE : 0);
    if (tens > 0) return ROD_H + 5 + onesH;
    return onesH || CUBE;
  }

  // Draw operator symbol centered vertically between the taller group
  function drawOp(x, y, symbol, numA, numB) {
    const h = Math.max(contentHeight(numA), contentHeight(numB));
    ctx.fillStyle = '#FFF';
    ctx.font = 'bold 28px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(symbol, x, y + h / 2 + 8);
  }

  if (op === '+') {
    const wA = measureNum(a);
    const wB = measureNum(b);
    const opGap = 40;
    const totalW = wA + opGap + wB;
    const startX = cx - totalW / 2;

    drawNum(startX, cy, a, COLOR_A);
    drawOp(startX + wA + opGap / 2, cy, '+', a, b);
    drawNum(startX + wA + opGap, cy, b, COLOR_B);

  } else if (op === '-' || op === '\u2212') {
    const wA = measureNum(a);
    const wB = measureNum(b);
    const opGap = 40;
    const totalW = wA + opGap + wB;
    const startX = cx - totalW / 2;

    drawNum(startX, cy, a, COLOR_A);
    drawOp(startX + wA + opGap / 2, cy, '\u2212', a, b);
    // Draw B in red to show "take away"
    drawNum(startX + wA + opGap, cy, b, { rod: '#EF5350', cube: '#EF9A9A', five: '#E57373' });

  } else if (op === '\u00d7' || op === '*') {
    // Array: a rows of b dots
    const rows = Math.min(a, b) <= 12 ? Math.min(a, b) : Math.min(a, 6);
    const cols = Math.max(a, b) <= 12 ? Math.max(a, b) : Math.min(Math.max(a, b), 12);
    const dotR = 5;
    const dotGap = 4;
    const gridW = cols * (dotR * 2 + dotGap);
    const gridH = rows * (dotR * 2 + dotGap);
    const startX = cx - gridW / 2;
    const startY = cy;

    ctx.fillStyle = '#AAA';
    ctx.font = '14px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(`${Math.min(a, b)} rows of ${Math.max(a, b)}`, cx, cy - 8);

    for (let r = 0; r < rows; r++) {
      for (let c2 = 0; c2 < cols; c2++) {
        ctx.fillStyle = r % 2 === 0 ? COLOR_A.rod : COLOR_B.rod;
        ctx.beginPath();
        ctx.arc(startX + c2 * (dotR * 2 + dotGap) + dotR,
          startY + 5 + r * (dotR * 2 + dotGap) + dotR,
          dotR, 0, Math.PI * 2);
        ctx.fill();
      }
    }

  } else if (op === '\u00f7' || op === '/') {
    // Division: show as "a split into b groups of answer"
    const groups = Math.min(b, 6);
    const perGroup = Math.min(answer, 12);
    const dotR = 5;
    const dotGap = 3;
    const groupW = perGroup * (dotR * 2 + dotGap) + 10;
    const totalW = groups * groupW;
    const startX = cx - Math.min(totalW, 500) / 2;

    ctx.fillStyle = '#AAA';
    ctx.font = '14px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(`${a} split into ${b} groups`, cx, cy - 8);

    for (let g = 0; g < groups; g++) {
      const gx = startX + g * groupW;
      // Group outline
      ctx.strokeStyle = '#546E7A';
      ctx.lineWidth = 1;
      ctx.strokeRect(gx, cy + 2, groupW - 8, dotR * 2 + 8);
      // Dots in group
      for (let d = 0; d < perGroup; d++) {
        ctx.fillStyle = g % 2 === 0 ? COLOR_A.rod : COLOR_B.rod;
        ctx.beginPath();
        ctx.arc(gx + 5 + d * (dotR * 2 + dotGap) + dotR, cy + 7 + dotR, dotR, 0, Math.PI * 2);
        ctx.fill();
      }
      // Group count
      ctx.fillStyle = '#78909C';
      ctx.font = '11px "Segoe UI", system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(String(perGroup), gx + (groupW - 8) / 2, cy + dotR * 2 + 22);
    }
    ctx.textAlign = 'left';
  }
}

// Register with visual registry
if (typeof registerVisual === "function") {
  registerVisual("base10_blocks", {
    name: "Base-10 Blocks",
    description: "Tens rods + ones cubes. Shows place value.",
    operations: ["add", "sub", "multiply", "divide"],
    bandRange: [5, 10],
    craStage: "concrete",
  }, renderBase10Blocks);
}
