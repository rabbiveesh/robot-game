// base10-blocks-visual.js — Tens rods + ones cubes
// For numbers 1-200+. Shows place value visually.
// A long bar = 10, a small square = 1.

// Registration is at end of file (after function definition).

function renderBase10Blocks(ctx, a, b, op, answer, cx, cy, time) {
  const rodW = 8;
  const rodH = 40;
  const cubeSize = 8;
  const gap = 3;
  const groupGap = 30;
  const rodColor1 = '#42A5F5';
  const rodColor2 = '#FFD54F';
  const cubeColor1 = '#64B5F6';
  const cubeColor2 = '#FFE082';
  const ansColor = '#69F0AE';

  function drawNumber(x, y, num, rodColor, cubeColor, label) {
    const tens = Math.floor(num / 10);
    const ones = num % 10;

    // Label
    ctx.fillStyle = '#AAA';
    ctx.font = 'bold 14px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(String(num), x + Math.max((tens * (rodW + gap)) / 2, 20), y - 8);

    let drawX = x;

    // Tens rods (vertical bars)
    for (let i = 0; i < Math.min(tens, 12); i++) {
      ctx.fillStyle = rodColor;
      ctx.fillRect(drawX, y, rodW, rodH);
      ctx.strokeStyle = 'rgba(0,0,0,0.2)';
      ctx.lineWidth = 1;
      ctx.strokeRect(drawX, y, rodW, rodH);
      drawX += rodW + gap;
    }
    if (tens > 12) {
      ctx.fillStyle = '#AAA';
      ctx.font = '12px "Segoe UI", system-ui, sans-serif';
      ctx.textAlign = 'left';
      ctx.fillText(`+${tens - 12} more`, drawX, y + rodH / 2 + 4);
    }

    // Ones cubes (below the rods)
    const onesY = y + rodH + 6;
    let onesX = x;
    for (let i = 0; i < ones; i++) {
      ctx.fillStyle = cubeColor;
      ctx.fillRect(onesX, onesY, cubeSize, cubeSize);
      ctx.strokeStyle = 'rgba(0,0,0,0.2)';
      ctx.lineWidth = 1;
      ctx.strokeRect(onesX, onesY, cubeSize, cubeSize);
      onesX += cubeSize + gap;
    }
  }

  if (op === '+') {
    // Show A blocks + B blocks → Answer blocks
    const aWidth = Math.max(Math.floor(a / 10) * (rodW + gap), 30);
    const bWidth = Math.max(Math.floor(b / 10) * (rodW + gap), 30);

    const totalW = aWidth + groupGap + 30 + groupGap + bWidth;
    const startX = cx - totalW / 2;

    drawNumber(startX, cy, a, rodColor1, cubeColor1);

    // Plus sign
    ctx.fillStyle = '#FFF';
    ctx.font = 'bold 24px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('+', startX + aWidth + groupGap / 2 + 5, cy + 20);

    drawNumber(startX + aWidth + groupGap + 20, cy, b, rodColor2, cubeColor2);

  } else if (op === '-' || op === '\u2212') {
    // Show A blocks, then cross out B worth
    const aWidth = Math.max(Math.floor(a / 10) * (rodW + gap), 30);
    const startX = cx - aWidth / 2;

    drawNumber(startX, cy, a, rodColor1, cubeColor1);

    // Show B blocks to remove (in red, to the right)
    ctx.fillStyle = '#F44336';
    ctx.font = 'bold 18px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(`take away`, cx, cy + rodH + cubeSize + 25);

    const bWidth = Math.max(Math.floor(b / 10) * (rodW + gap), 30);
    drawNumber(cx - bWidth / 2, cy + rodH + cubeSize + 32, b, '#EF5350', '#EF9A9A');

  } else if (op === '\u00d7' || op === '*') {
    // Show a × b as 'a groups of b'
    const groups = Math.min(a, 12);
    const perGroup = Math.min(b, 12);
    const dotSize = 6;
    const dotGap = 2;
    const groupW = perGroup * (dotSize + dotGap);
    const totalW = groups * (groupW + 10);
    let startX = cx - Math.min(totalW, 300) / 2;

    ctx.fillStyle = '#AAA';
    ctx.font = '14px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(`${a} groups of ${b}`, cx, cy - 8);

    for (let g = 0; g < groups && g < 6; g++) {
      for (let d = 0; d < perGroup; d++) {
        ctx.fillStyle = g % 2 === 0 ? rodColor1 : rodColor2;
        ctx.beginPath();
        ctx.arc(startX + d * (dotSize + dotGap) + dotSize / 2,
          cy + 10 + g * (dotSize + dotGap + 2) + dotSize / 2,
          dotSize / 2, 0, Math.PI * 2);
        ctx.fill();
      }
    }
    if (groups > 6) {
      ctx.fillStyle = '#AAA';
      ctx.font = '12px "Segoe UI", system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(`... and ${groups - 6} more groups`, cx, cy + 10 + 6 * (dotSize + dotGap + 2) + 10);
    }

  } else if (op === '\u00f7' || op === '/') {
    // Division: show dividend as blocks, partition into divisor groups
    ctx.fillStyle = '#AAA';
    ctx.font = '14px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(`${a} split into ${b} equal groups`, cx, cy - 8);
    ctx.fillText(`= ${answer} in each group`, cx, cy + 12);

    // Draw answer-sized groups
    const groups = Math.min(b, 6);
    const perGroup = Math.min(answer, 12);
    const dotSize = 6;
    const dotGap = 2;
    let startX = cx - (groups * (perGroup * (dotSize + dotGap) + 15)) / 2;

    for (let g = 0; g < groups; g++) {
      const gx = startX + g * (perGroup * (dotSize + dotGap) + 15);
      // Group bracket
      ctx.strokeStyle = '#546E7A';
      ctx.lineWidth = 1;
      ctx.strokeRect(gx - 2, cy + 22, perGroup * (dotSize + dotGap) + 2, dotSize + 6);
      for (let d = 0; d < perGroup; d++) {
        ctx.fillStyle = g % 2 === 0 ? rodColor1 : rodColor2;
        ctx.beginPath();
        ctx.arc(gx + d * (dotSize + dotGap) + dotSize / 2, cy + 25 + dotSize / 2, dotSize / 2, 0, Math.PI * 2);
        ctx.fill();
      }
    }
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
