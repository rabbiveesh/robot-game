// sprites.js — All programmatic pixel art (no external images)
// Every tile and character is drawn directly onto the canvas.

const TILE_SIZE = 48;

// Seeded random for stable "random" details on tiles
function seededRandom(x, y, seed) {
  let h = (x * 374761393 + y * 668265263 + seed * 1274126177) | 0;
  h = ((h ^ (h >> 13)) * 1274126177) | 0;
  return (h & 0x7fffffff) / 0x7fffffff;
}

// ─── TILE DRAWING ────────────────────────────────────────

function drawGrass(ctx, x, y) {
  ctx.fillStyle = '#4CAF50';
  ctx.fillRect(x, y, TILE_SIZE, TILE_SIZE);
  // Little darker grass tufts
  for (let i = 0; i < 4; i++) {
    const rx = seededRandom(x, y, i * 3) * (TILE_SIZE - 6) + 3;
    const ry = seededRandom(x, y, i * 3 + 1) * (TILE_SIZE - 6) + 3;
    ctx.fillStyle = '#43A047';
    ctx.fillRect(x + rx, y + ry, 3, 3);
  }
}

function drawPath(ctx, x, y) {
  ctx.fillStyle = '#DEB887';
  ctx.fillRect(x, y, TILE_SIZE, TILE_SIZE);
  // Subtle pebbles
  for (let i = 0; i < 3; i++) {
    const rx = seededRandom(x, y, i * 7) * (TILE_SIZE - 8) + 4;
    const ry = seededRandom(x, y, i * 7 + 1) * (TILE_SIZE - 8) + 4;
    ctx.fillStyle = '#C8A96E';
    ctx.beginPath();
    ctx.arc(x + rx, y + ry, 2, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawWater(ctx, x, y, time) {
  ctx.fillStyle = '#42A5F5';
  ctx.fillRect(x, y, TILE_SIZE, TILE_SIZE);
  // Animated wave lines
  ctx.strokeStyle = '#64B5F6';
  ctx.lineWidth = 2;
  for (let row = 0; row < 3; row++) {
    ctx.beginPath();
    const baseY = y + 10 + row * 14;
    for (let px = 0; px <= TILE_SIZE; px += 4) {
      const wave = Math.sin((x + px) * 0.1 + time * 2 + row) * 3;
      if (px === 0) ctx.moveTo(x + px, baseY + wave);
      else ctx.lineTo(x + px, baseY + wave);
    }
    ctx.stroke();
  }
}

function drawWall(ctx, x, y) {
  ctx.fillStyle = '#8D6E63';
  ctx.fillRect(x, y, TILE_SIZE, TILE_SIZE);
  // Brick pattern
  ctx.strokeStyle = '#6D4C41';
  ctx.lineWidth = 1;
  for (let row = 0; row < 3; row++) {
    const by = y + row * 16;
    ctx.strokeRect(x, by, TILE_SIZE, 16);
    const offset = row % 2 === 0 ? 0 : TILE_SIZE / 2;
    ctx.beginPath();
    ctx.moveTo(x + TILE_SIZE / 2 + offset, by);
    ctx.lineTo(x + TILE_SIZE / 2 + offset, by + 16);
    ctx.stroke();
  }
}

function drawTree(ctx, x, y, time) {
  // Grass base
  drawGrass(ctx, x, y);
  // Trunk
  ctx.fillStyle = '#6D4C41';
  ctx.fillRect(x + 19, y + 28, 10, 18);
  // Canopy (slight sway)
  const sway = Math.sin(time * 1.5 + x * 0.3) * 1.5;
  ctx.fillStyle = '#2E7D32';
  ctx.beginPath();
  ctx.arc(x + 24 + sway, y + 20, 16, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#388E3C';
  ctx.beginPath();
  ctx.arc(x + 18 + sway, y + 24, 11, 0, Math.PI * 2);
  ctx.fill();
  ctx.beginPath();
  ctx.arc(x + 30 + sway, y + 24, 11, 0, Math.PI * 2);
  ctx.fill();
}

function drawFlower(ctx, x, y, time) {
  drawGrass(ctx, x, y);
  // 2-3 flowers
  const colors = ['#FF6B6B', '#FFD93D', '#E040FB'];
  for (let i = 0; i < 3; i++) {
    const fx = x + 8 + seededRandom(x, y, i * 5) * 28;
    const fy = y + 8 + seededRandom(x, y, i * 5 + 1) * 28;
    const sway = Math.sin(time * 2 + i * 2) * 1.5;
    // Stem
    ctx.strokeStyle = '#388E3C';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(fx, fy + 6);
    ctx.lineTo(fx + sway, fy - 2);
    ctx.stroke();
    // Petals
    ctx.fillStyle = colors[i % 3];
    ctx.beginPath();
    ctx.arc(fx + sway, fy - 4, 4, 0, Math.PI * 2);
    ctx.fill();
    // Center
    ctx.fillStyle = '#FFF9C4';
    ctx.beginPath();
    ctx.arc(fx + sway, fy - 4, 1.5, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawHouseWall(ctx, x, y) {
  ctx.fillStyle = '#FFCC80';
  ctx.fillRect(x, y, TILE_SIZE, TILE_SIZE);
  ctx.strokeStyle = '#EF6C00';
  ctx.lineWidth = 2;
  ctx.strokeRect(x + 1, y + 1, TILE_SIZE - 2, TILE_SIZE - 2);
}

function drawHouseRoof(ctx, x, y) {
  ctx.fillStyle = '#D32F2F';
  ctx.fillRect(x, y, TILE_SIZE, TILE_SIZE);
  // Shingle lines
  ctx.strokeStyle = '#B71C1C';
  ctx.lineWidth = 1;
  for (let i = 0; i < 3; i++) {
    ctx.beginPath();
    ctx.moveTo(x, y + 12 + i * 14);
    ctx.lineTo(x + TILE_SIZE, y + 12 + i * 14);
    ctx.stroke();
  }
}

function drawDoor(ctx, x, y) {
  drawHouseWall(ctx, x, y);
  // Door
  ctx.fillStyle = '#5D4037';
  ctx.fillRect(x + 14, y + 10, 20, 38);
  ctx.fillStyle = '#8D6E63';
  ctx.fillRect(x + 16, y + 12, 16, 34);
  // Doorknob
  ctx.fillStyle = '#FFD54F';
  ctx.beginPath();
  ctx.arc(x + 28, y + 30, 3, 0, Math.PI * 2);
  ctx.fill();
}

function drawWindow(ctx, x, y) {
  drawHouseWall(ctx, x, y);
  // Window
  ctx.fillStyle = '#81D4FA';
  ctx.fillRect(x + 12, y + 12, 24, 20);
  ctx.strokeStyle = '#EF6C00';
  ctx.lineWidth = 2;
  ctx.strokeRect(x + 12, y + 12, 24, 20);
  // Cross bar
  ctx.beginPath();
  ctx.moveTo(x + 24, y + 12);
  ctx.lineTo(x + 24, y + 32);
  ctx.moveTo(x + 12, y + 22);
  ctx.lineTo(x + 36, y + 22);
  ctx.stroke();
}

function drawFence(ctx, x, y) {
  drawGrass(ctx, x, y);
  ctx.fillStyle = '#A1887F';
  // Posts
  ctx.fillRect(x + 4, y + 12, 6, 30);
  ctx.fillRect(x + 38, y + 12, 6, 30);
  // Rails
  ctx.fillRect(x + 2, y + 16, 44, 5);
  ctx.fillRect(x + 2, y + 30, 44, 5);
  // Pointed tops
  ctx.fillStyle = '#8D6E63';
  ctx.beginPath();
  ctx.moveTo(x + 4, y + 12);
  ctx.lineTo(x + 7, y + 6);
  ctx.lineTo(x + 10, y + 12);
  ctx.fill();
  ctx.beginPath();
  ctx.moveTo(x + 38, y + 12);
  ctx.lineTo(x + 41, y + 6);
  ctx.lineTo(x + 44, y + 12);
  ctx.fill();
}

function drawSign(ctx, x, y) {
  drawGrass(ctx, x, y);
  // Post
  ctx.fillStyle = '#8D6E63';
  ctx.fillRect(x + 21, y + 22, 6, 24);
  // Sign board
  ctx.fillStyle = '#FFCC80';
  ctx.fillRect(x + 8, y + 8, 32, 18);
  ctx.strokeStyle = '#6D4C41';
  ctx.lineWidth = 2;
  ctx.strokeRect(x + 8, y + 8, 32, 18);
  // "!" on sign
  ctx.fillStyle = '#D32F2F';
  ctx.font = 'bold 14px sans-serif';
  ctx.textAlign = 'center';
  ctx.fillText('!', x + 24, y + 22);
}

function drawBridge(ctx, x, y) {
  ctx.fillStyle = '#42A5F5';
  ctx.fillRect(x, y, TILE_SIZE, TILE_SIZE);
  // Wooden planks
  ctx.fillStyle = '#A1887F';
  ctx.fillRect(x + 4, y, 40, TILE_SIZE);
  ctx.strokeStyle = '#8D6E63';
  ctx.lineWidth = 1;
  for (let i = 0; i < 4; i++) {
    ctx.beginPath();
    ctx.moveTo(x + 4, y + i * 12 + 12);
    ctx.lineTo(x + 44, y + i * 12 + 12);
    ctx.stroke();
  }
  // Rails
  ctx.fillStyle = '#6D4C41';
  ctx.fillRect(x + 2, y, 4, TILE_SIZE);
  ctx.fillRect(x + 42, y, 4, TILE_SIZE);
}

function drawChest(ctx, x, y, time) {
  drawGrass(ctx, x, y);
  // Chest body
  ctx.fillStyle = '#8D6E63';
  ctx.fillRect(x + 10, y + 20, 28, 20);
  // Chest lid
  ctx.fillStyle = '#A1887F';
  ctx.fillRect(x + 8, y + 14, 32, 12);
  // Metal band
  ctx.fillStyle = '#FFD54F';
  ctx.fillRect(x + 10, y + 18, 28, 3);
  // Lock
  ctx.fillStyle = '#FFD54F';
  ctx.beginPath();
  ctx.arc(x + 24, y + 28, 4, 0, Math.PI * 2);
  ctx.fill();
  // Sparkle
  const sparkle = Math.sin(time * 3) * 0.5 + 0.5;
  ctx.fillStyle = `rgba(255, 235, 59, ${sparkle})`;
  ctx.beginPath();
  ctx.arc(x + 32, y + 12, 3, 0, Math.PI * 2);
  ctx.fill();
}

// ─── INTERIOR TILES ──────────────────────────────────────

function drawFloor(ctx, x, y) {
  ctx.fillStyle = '#A1887F';
  ctx.fillRect(x, y, TILE_SIZE, TILE_SIZE);
  // Wood plank lines
  ctx.strokeStyle = '#8D6E63';
  ctx.lineWidth = 1;
  for (let i = 0; i < 3; i++) {
    ctx.beginPath();
    ctx.moveTo(x, y + 16 * i + 8);
    ctx.lineTo(x + TILE_SIZE, y + 16 * i + 8);
    ctx.stroke();
  }
  // Vertical seam
  const seam = seededRandom(x, y, 99) < 0.5 ? 20 : 28;
  ctx.beginPath();
  ctx.moveTo(x + seam, y);
  ctx.lineTo(x + seam, y + TILE_SIZE);
  ctx.stroke();
}

function drawRug(ctx, x, y) {
  drawFloor(ctx, x, y);
  ctx.fillStyle = '#C62828';
  ctx.fillRect(x + 2, y + 2, TILE_SIZE - 4, TILE_SIZE - 4);
  // Rug pattern border
  ctx.strokeStyle = '#FFD54F';
  ctx.lineWidth = 2;
  ctx.strokeRect(x + 6, y + 6, TILE_SIZE - 12, TILE_SIZE - 12);
  // Center diamond
  ctx.fillStyle = '#FFD54F';
  ctx.beginPath();
  ctx.moveTo(x + TILE_SIZE / 2, y + 12);
  ctx.lineTo(x + TILE_SIZE - 12, y + TILE_SIZE / 2);
  ctx.lineTo(x + TILE_SIZE / 2, y + TILE_SIZE - 12);
  ctx.lineTo(x + 12, y + TILE_SIZE / 2);
  ctx.fill();
}

function drawTable(ctx, x, y) {
  drawFloor(ctx, x, y);
  // Table top
  ctx.fillStyle = '#6D4C41';
  ctx.fillRect(x + 4, y + 8, TILE_SIZE - 8, TILE_SIZE - 16);
  ctx.strokeStyle = '#5D4037';
  ctx.lineWidth = 2;
  ctx.strokeRect(x + 4, y + 8, TILE_SIZE - 8, TILE_SIZE - 16);
  // Something on the table
  ctx.fillStyle = '#81D4FA';
  ctx.fillRect(x + 14, y + 14, 10, 8);
  ctx.fillStyle = '#E0E0E0';
  ctx.fillRect(x + 26, y + 16, 8, 6);
}

function drawBookshelf(ctx, x, y) {
  // Back wall
  ctx.fillStyle = '#5D4037';
  ctx.fillRect(x, y, TILE_SIZE, TILE_SIZE);
  // Shelf frame
  ctx.fillStyle = '#8D6E63';
  ctx.fillRect(x + 2, y + 2, TILE_SIZE - 4, TILE_SIZE - 4);
  // Shelves
  ctx.fillStyle = '#6D4C41';
  ctx.fillRect(x + 2, y + 20, TILE_SIZE - 4, 3);
  ctx.fillRect(x + 2, y + 38, TILE_SIZE - 4, 3);
  // Books (top shelf)
  const bookColors = ['#F44336', '#2196F3', '#4CAF50', '#FF9800', '#9C27B0', '#FFEB3B'];
  for (let i = 0; i < 5; i++) {
    const bw = 5 + seededRandom(x, y, i * 2) * 3;
    const bx = x + 5 + i * 8;
    ctx.fillStyle = bookColors[i % bookColors.length];
    ctx.fillRect(bx, y + 5, bw, 15);
  }
  // Books (bottom shelf)
  for (let i = 0; i < 4; i++) {
    const bw = 6 + seededRandom(x, y, i * 3 + 10) * 3;
    const bx = x + 6 + i * 9;
    ctx.fillStyle = bookColors[(i + 3) % bookColors.length];
    ctx.fillRect(bx, y + 24, bw, 13);
  }
}

// Tile type registry
const TILE_TYPES = {
  0: { name: 'grass',     draw: drawGrass,     solid: false },
  1: { name: 'path',      draw: drawPath,      solid: false },
  2: { name: 'water',     draw: drawWater,     solid: true  },
  3: { name: 'wall',      draw: drawWall,      solid: true  },
  4: { name: 'tree',      draw: drawTree,      solid: true  },
  5: { name: 'flower',    draw: drawFlower,    solid: false },
  6: { name: 'houseWall', draw: drawHouseWall, solid: true  },
  7: { name: 'houseRoof', draw: drawHouseRoof, solid: true  },
  8: { name: 'door',      draw: drawDoor,      solid: true  },
  9: { name: 'window',    draw: drawWindow,    solid: true  },
  10: { name: 'fence',    draw: drawFence,     solid: true  },
  11: { name: 'sign',     draw: drawSign,      solid: true  },
  12: { name: 'bridge',   draw: drawBridge,    solid: false },
  13: { name: 'chest',    draw: drawChest,     solid: true  },
  14: { name: 'floor',    draw: drawFloor,     solid: false },
  15: { name: 'rug',      draw: drawRug,       solid: false },
  16: { name: 'table',    draw: drawTable,     solid: true  },
  17: { name: 'bookshelf',draw: drawBookshelf, solid: true  },
};

function drawTile(ctx, tileId, px, py, time) {
  const tile = TILE_TYPES[tileId];
  if (!tile) return;
  if (tile.draw.length === 3) tile.draw(ctx, px, py);
  else tile.draw(ctx, px, py, time);
}

// ─── CHARACTER SPRITES ───────────────────────────────────

const DIR = { down: 0, left: 1, right: 2, up: 3 };

// PLAYER_GENDER is set at game start: 'boy' or 'girl'
let PLAYER_GENDER = 'boy';

function drawPlayer(ctx, x, y, dir, frame, time) {
  if (PLAYER_GENDER === 'girl') {
    drawPlayerGirl(ctx, x, y, dir, frame, time);
  } else {
    drawPlayerBoy(ctx, x, y, dir, frame, time);
  }
}

function drawPlayerBoy(ctx, x, y, dir, frame, time) {
  const cx = x + TILE_SIZE / 2;
  const cy = y + TILE_SIZE / 2 + 4;
  const walkBob = frame % 2 === 1 ? -2 : 0;

  // Shadow
  ctx.fillStyle = 'rgba(0,0,0,0.15)';
  ctx.beginPath();
  ctx.ellipse(cx, y + TILE_SIZE - 4, 12, 5, 0, 0, Math.PI * 2);
  ctx.fill();

  // Body
  ctx.fillStyle = '#42A5F5'; // blue shirt
  ctx.fillRect(cx - 8, cy - 2 + walkBob, 16, 14);

  // Legs
  ctx.fillStyle = '#5D4037';
  const legOffset = frame % 2 === 1 ? 3 : 0;
  ctx.fillRect(cx - 6, cy + 12 + walkBob, 5, 8 - legOffset);
  ctx.fillRect(cx + 1, cy + 12 + walkBob, 5, 8 - (frame % 2 === 0 ? 3 : 0));

  // Head
  ctx.fillStyle = '#FFCC80';
  ctx.beginPath();
  ctx.arc(cx, cy - 8 + walkBob, 10, 0, Math.PI * 2);
  ctx.fill();

  // Hair
  ctx.fillStyle = '#5D4037';
  ctx.beginPath();
  ctx.arc(cx, cy - 12 + walkBob, 10, Math.PI, Math.PI * 2);
  ctx.fill();

  // Eyes
  ctx.fillStyle = '#333';
  if (dir === DIR.left) {
    ctx.fillRect(cx - 6, cy - 10 + walkBob, 3, 3);
    ctx.fillRect(cx - 1, cy - 10 + walkBob, 3, 3);
  } else if (dir === DIR.right) {
    ctx.fillRect(cx - 1, cy - 10 + walkBob, 3, 3);
    ctx.fillRect(cx + 4, cy - 10 + walkBob, 3, 3);
  } else if (dir === DIR.up) {
    // Facing away — no eyes
  } else {
    ctx.fillRect(cx - 5, cy - 10 + walkBob, 3, 3);
    ctx.fillRect(cx + 2, cy - 10 + walkBob, 3, 3);
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(cx, cy - 4 + walkBob, 4, 0.1 * Math.PI, 0.9 * Math.PI);
    ctx.stroke();
  }
}

function drawPlayerGirl(ctx, x, y, dir, frame, time) {
  const cx = x + TILE_SIZE / 2;
  const cy = y + TILE_SIZE / 2 + 4;
  const walkBob = frame % 2 === 1 ? -2 : 0;

  // Shadow
  ctx.fillStyle = 'rgba(0,0,0,0.15)';
  ctx.beginPath();
  ctx.ellipse(cx, y + TILE_SIZE - 4, 12, 5, 0, 0, Math.PI * 2);
  ctx.fill();

  // Dress
  ctx.fillStyle = '#F48FB1'; // pink dress
  ctx.beginPath();
  ctx.moveTo(cx - 8, cy - 2 + walkBob);
  ctx.lineTo(cx + 8, cy - 2 + walkBob);
  ctx.lineTo(cx + 11, cy + 14 + walkBob);
  ctx.lineTo(cx - 11, cy + 14 + walkBob);
  ctx.fill();
  // Dress stripe
  ctx.fillStyle = '#EC407A';
  ctx.fillRect(cx - 9, cy + 6 + walkBob, 18, 3);

  // Legs
  ctx.fillStyle = '#FFCC80';
  const legOffset = frame % 2 === 1 ? 3 : 0;
  ctx.fillRect(cx - 5, cy + 14 + walkBob, 4, 6 - legOffset);
  ctx.fillRect(cx + 1, cy + 14 + walkBob, 4, 6 - (frame % 2 === 0 ? 3 : 0));

  // Head
  ctx.fillStyle = '#FFCC80';
  ctx.beginPath();
  ctx.arc(cx, cy - 8 + walkBob, 10, 0, Math.PI * 2);
  ctx.fill();

  // Hair (long, with pigtails)
  ctx.fillStyle = '#6D4C41';
  ctx.beginPath();
  ctx.arc(cx, cy - 11 + walkBob, 11, Math.PI * 0.8, Math.PI * 2.2);
  ctx.fill();
  // Pigtails
  ctx.fillRect(cx - 12, cy - 8 + walkBob, 5, 18);
  ctx.fillRect(cx + 7, cy - 8 + walkBob, 5, 18);
  // Pigtail ends (rounded)
  ctx.beginPath();
  ctx.arc(cx - 9.5, cy + 10 + walkBob, 3, 0, Math.PI * 2);
  ctx.fill();
  ctx.beginPath();
  ctx.arc(cx + 9.5, cy + 10 + walkBob, 3, 0, Math.PI * 2);
  ctx.fill();

  // Hair bow
  ctx.fillStyle = '#FF5252';
  ctx.beginPath();
  ctx.moveTo(cx + 2, cy - 18 + walkBob);
  ctx.lineTo(cx + 10, cy - 22 + walkBob);
  ctx.lineTo(cx + 4, cy - 16 + walkBob);
  ctx.lineTo(cx + 10, cy - 12 + walkBob);
  ctx.lineTo(cx + 2, cy - 16 + walkBob);
  ctx.fill();

  // Eyes (bigger, with lashes)
  ctx.fillStyle = '#333';
  if (dir === DIR.left) {
    ctx.fillRect(cx - 6, cy - 10 + walkBob, 3, 4);
    ctx.fillRect(cx - 1, cy - 10 + walkBob, 3, 4);
  } else if (dir === DIR.right) {
    ctx.fillRect(cx - 1, cy - 10 + walkBob, 3, 4);
    ctx.fillRect(cx + 4, cy - 10 + walkBob, 3, 4);
  } else if (dir === DIR.up) {
    // Facing away
  } else {
    ctx.fillRect(cx - 5, cy - 10 + walkBob, 3, 4);
    ctx.fillRect(cx + 2, cy - 10 + walkBob, 3, 4);
    // Eyelashes
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(cx - 6, cy - 10 + walkBob);
    ctx.lineTo(cx - 7, cy - 12 + walkBob);
    ctx.moveTo(cx + 5, cy - 10 + walkBob);
    ctx.lineTo(cx + 6, cy - 12 + walkBob);
    ctx.stroke();
    // Smile
    ctx.strokeStyle = '#E91E63';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(cx, cy - 4 + walkBob, 4, 0.1 * Math.PI, 0.9 * Math.PI);
    ctx.stroke();
  }
}

function drawRobot(ctx, x, y, dir, frame, time) {
  const cx = x + TILE_SIZE / 2;
  const cy = y + TILE_SIZE / 2 + 2;
  const bob = Math.sin(time * 3) * 2;
  const walkShift = frame % 2 === 1 ? 1 : -1;

  // Shadow
  ctx.fillStyle = 'rgba(0,0,0,0.15)';
  ctx.beginPath();
  ctx.ellipse(cx, y + TILE_SIZE - 3, 11, 4, 0, 0, Math.PI * 2);
  ctx.fill();

  // Antenna
  ctx.strokeStyle = '#78909C';
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(cx, cy - 16 + bob);
  ctx.lineTo(cx, cy - 26 + bob);
  ctx.stroke();
  // Antenna ball
  const antennaBob = Math.sin(time * 4) * 2;
  ctx.fillStyle = '#FF5252';
  ctx.beginPath();
  ctx.arc(cx, cy - 28 + bob + antennaBob, 4, 0, Math.PI * 2);
  ctx.fill();

  // Body (rounded rectangle)
  const bx = cx - 12, by = cy - 10 + bob, bw = 24, bh = 22;
  ctx.fillStyle = '#B0BEC5';
  ctx.beginPath();
  ctx.moveTo(bx + 4, by);
  ctx.lineTo(bx + bw - 4, by);
  ctx.quadraticCurveTo(bx + bw, by, bx + bw, by + 4);
  ctx.lineTo(bx + bw, by + bh - 4);
  ctx.quadraticCurveTo(bx + bw, by + bh, bx + bw - 4, by + bh);
  ctx.lineTo(bx + 4, by + bh);
  ctx.quadraticCurveTo(bx, by + bh, bx, by + bh - 4);
  ctx.lineTo(bx, by + 4);
  ctx.quadraticCurveTo(bx, by, bx + 4, by);
  ctx.fill();
  // Body outline
  ctx.strokeStyle = '#78909C';
  ctx.lineWidth = 2;
  ctx.stroke();

  // Head
  ctx.fillStyle = '#CFD8DC';
  const hx = cx - 10, hy = cy - 20 + bob;
  ctx.fillRect(hx, hy, 20, 14);
  ctx.strokeStyle = '#78909C';
  ctx.lineWidth = 1.5;
  ctx.strokeRect(hx, hy, 20, 14);

  // Eyes (change with direction)
  const blink = Math.sin(time * 5) > 0.95;
  ctx.fillStyle = '#00E676';
  if (blink) {
    ctx.fillRect(cx - 7, cy - 16 + bob, 6, 2);
    ctx.fillRect(cx + 1, cy - 16 + bob, 6, 2);
  } else {
    ctx.fillRect(cx - 7, cy - 18 + bob, 6, 6);
    ctx.fillRect(cx + 1, cy - 18 + bob, 6, 6);
    // Pupils
    ctx.fillStyle = '#1B5E20';
    const pupilOff = dir === DIR.left ? -1 : dir === DIR.right ? 1 : 0;
    const pupilOffY = dir === DIR.up ? -1 : dir === DIR.down ? 1 : 0;
    ctx.fillRect(cx - 6 + pupilOff, cy - 17 + bob + pupilOffY, 3, 3);
    ctx.fillRect(cx + 2 + pupilOff, cy - 17 + bob + pupilOffY, 3, 3);
  }

  // Smile
  ctx.strokeStyle = '#00E676';
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.arc(cx, cy - 9 + bob, 5, 0.1 * Math.PI, 0.9 * Math.PI);
  ctx.stroke();

  // Arms
  ctx.fillStyle = '#90A4AE';
  ctx.fillRect(cx - 16, cy - 6 + bob + walkShift, 5, 12);
  ctx.fillRect(cx + 11, cy - 6 + bob - walkShift, 5, 12);

  // Legs
  ctx.fillStyle = '#78909C';
  ctx.fillRect(cx - 8, cy + 12 + bob, 6, 8 + walkShift);
  ctx.fillRect(cx + 2, cy + 12 + bob, 6, 8 - walkShift);

  // Chest light
  const lightPulse = Math.sin(time * 2) * 0.3 + 0.7;
  ctx.fillStyle = `rgba(0, 230, 118, ${lightPulse})`;
  ctx.beginPath();
  ctx.arc(cx, cy + bob, 3, 0, Math.PI * 2);
  ctx.fill();
}

function drawMommy(ctx, x, y, dir, frame, time) {
  const cx = x + TILE_SIZE / 2;
  const cy = y + TILE_SIZE / 2 + 4;

  // Shadow
  ctx.fillStyle = 'rgba(0,0,0,0.15)';
  ctx.beginPath();
  ctx.ellipse(cx, y + TILE_SIZE - 4, 12, 5, 0, 0, Math.PI * 2);
  ctx.fill();

  // Dress
  ctx.fillStyle = '#E040FB';
  ctx.beginPath();
  ctx.moveTo(cx - 10, cy - 2);
  ctx.lineTo(cx + 10, cy - 2);
  ctx.lineTo(cx + 14, cy + 18);
  ctx.lineTo(cx - 14, cy + 18);
  ctx.fill();

  // Head
  ctx.fillStyle = '#FFCC80';
  ctx.beginPath();
  ctx.arc(cx, cy - 8, 10, 0, Math.PI * 2);
  ctx.fill();

  // Hair
  ctx.fillStyle = '#4E342E';
  ctx.beginPath();
  ctx.arc(cx, cy - 10, 11, Math.PI * 0.8, Math.PI * 2.2);
  ctx.fill();
  // Long hair sides
  ctx.fillRect(cx - 11, cy - 8, 4, 16);
  ctx.fillRect(cx + 7, cy - 8, 4, 16);

  // Eyes
  ctx.fillStyle = '#333';
  ctx.fillRect(cx - 5, cy - 10, 3, 3);
  ctx.fillRect(cx + 2, cy - 10, 3, 3);

  // Warm smile
  ctx.strokeStyle = '#E91E63';
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.arc(cx, cy - 4, 5, 0.1 * Math.PI, 0.9 * Math.PI);
  ctx.stroke();

  // Heart above head (gentle float)
  const heartBob = Math.sin(time * 2) * 2;
  drawHeart(ctx, cx, cy - 24 + heartBob, 5, '#E91E63');
}

function drawHeart(ctx, x, y, size, color) {
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.moveTo(x, y + size * 0.3);
  ctx.bezierCurveTo(x, y - size * 0.3, x - size, y - size * 0.3, x - size, y + size * 0.1);
  ctx.bezierCurveTo(x - size, y + size * 0.6, x, y + size, x, y + size);
  ctx.bezierCurveTo(x, y + size, x + size, y + size * 0.6, x + size, y + size * 0.1);
  ctx.bezierCurveTo(x + size, y - size * 0.3, x, y - size * 0.3, x, y + size * 0.3);
  ctx.fill();
}

// Generic NPC (wizard/sage type)
function drawSage(ctx, x, y, dir, frame, time) {
  const cx = x + TILE_SIZE / 2;
  const cy = y + TILE_SIZE / 2 + 4;

  // Shadow
  ctx.fillStyle = 'rgba(0,0,0,0.15)';
  ctx.beginPath();
  ctx.ellipse(cx, y + TILE_SIZE - 4, 12, 5, 0, 0, Math.PI * 2);
  ctx.fill();

  // Robe
  ctx.fillStyle = '#7E57C2';
  ctx.beginPath();
  ctx.moveTo(cx - 10, cy - 4);
  ctx.lineTo(cx + 10, cy - 4);
  ctx.lineTo(cx + 12, cy + 18);
  ctx.lineTo(cx - 12, cy + 18);
  ctx.fill();

  // Head
  ctx.fillStyle = '#FFCC80';
  ctx.beginPath();
  ctx.arc(cx, cy - 8, 9, 0, Math.PI * 2);
  ctx.fill();

  // Wizard hat
  ctx.fillStyle = '#7E57C2';
  ctx.beginPath();
  ctx.moveTo(cx - 12, cy - 12);
  ctx.lineTo(cx, cy - 32);
  ctx.lineTo(cx + 12, cy - 12);
  ctx.fill();
  // Hat brim
  ctx.fillRect(cx - 14, cy - 14, 28, 4);
  // Star on hat
  ctx.fillStyle = '#FFD54F';
  ctx.font = '10px sans-serif';
  ctx.textAlign = 'center';
  ctx.fillText('★', cx, cy - 19);

  // Eyes
  ctx.fillStyle = '#333';
  ctx.fillRect(cx - 5, cy - 10, 3, 3);
  ctx.fillRect(cx + 2, cy - 10, 3, 3);

  // Beard
  ctx.fillStyle = '#E0E0E0';
  ctx.beginPath();
  ctx.moveTo(cx - 5, cy - 2);
  ctx.lineTo(cx + 5, cy - 2);
  ctx.lineTo(cx, cy + 10);
  ctx.fill();
}

// ─── DOG NPC (glitch doghouse) ───────────────────────────

function drawDog(ctx, x, y, dir, frame, time) {
  const cx = x + TILE_SIZE / 2;
  const cy = y + TILE_SIZE / 2 + 6;

  // Glitch flicker effect
  const glitchOff = Math.sin(time * 7) > 0.9 ? (Math.random() * 4 - 2) : 0;

  // Shadow
  ctx.fillStyle = 'rgba(0,0,0,0.15)';
  ctx.beginPath();
  ctx.ellipse(cx, y + TILE_SIZE - 3, 14, 5, 0, 0, Math.PI * 2);
  ctx.fill();

  // Body
  ctx.fillStyle = '#8D6E63';
  ctx.beginPath();
  ctx.ellipse(cx + glitchOff, cy, 14, 10, 0, 0, Math.PI * 2);
  ctx.fill();

  // Head
  ctx.fillStyle = '#A1887F';
  ctx.beginPath();
  ctx.arc(cx - 10 + glitchOff, cy - 10, 11, 0, Math.PI * 2);
  ctx.fill();

  // Ears (floppy)
  ctx.fillStyle = '#6D4C41';
  ctx.beginPath();
  ctx.ellipse(cx - 18 + glitchOff, cy - 8, 5, 9, -0.3, 0, Math.PI * 2);
  ctx.fill();
  ctx.beginPath();
  ctx.ellipse(cx - 2 + glitchOff, cy - 14, 5, 8, 0.3, 0, Math.PI * 2);
  ctx.fill();

  // Eyes
  ctx.fillStyle = '#333';
  ctx.beginPath();
  ctx.arc(cx - 13 + glitchOff, cy - 12, 2.5, 0, Math.PI * 2);
  ctx.fill();
  ctx.beginPath();
  ctx.arc(cx - 7 + glitchOff, cy - 12, 2.5, 0, Math.PI * 2);
  ctx.fill();

  // Nose
  ctx.fillStyle = '#333';
  ctx.beginPath();
  ctx.arc(cx - 14 + glitchOff, cy - 8, 2, 0, Math.PI * 2);
  ctx.fill();

  // Tongue (panting)
  if (Math.sin(time * 4) > 0) {
    ctx.fillStyle = '#E57373';
    ctx.beginPath();
    ctx.ellipse(cx - 14 + glitchOff, cy - 4, 3, 5, 0, 0, Math.PI * 2);
    ctx.fill();
  }

  // Tail (wagging)
  const wagAngle = Math.sin(time * 8) * 0.5;
  ctx.strokeStyle = '#8D6E63';
  ctx.lineWidth = 4;
  ctx.beginPath();
  ctx.moveTo(cx + 14 + glitchOff, cy - 2);
  ctx.quadraticCurveTo(cx + 22 + glitchOff, cy - 14 + wagAngle * 10, cx + 18 + glitchOff, cy - 18 + wagAngle * 8);
  ctx.stroke();

  // Legs
  ctx.fillStyle = '#6D4C41';
  const legBob = frame % 2 === 1 ? 2 : 0;
  ctx.fillRect(cx - 8 + glitchOff, cy + 6, 4, 8 + legBob);
  ctx.fillRect(cx + 4 + glitchOff, cy + 6, 4, 8 + (2 - legBob));

  // Glitch artifact: random pixel block near the dog
  if (Math.sin(time * 11) > 0.8) {
    const gx = cx + (Math.sin(time * 17) * 20) | 0;
    const gy = cy + (Math.cos(time * 13) * 15) | 0;
    ctx.fillStyle = `hsl(${(time * 200) % 360}, 80%, 50%)`;
    ctx.fillRect(gx, gy, 6, 6);
  }
}

// ─── DUM DUM LOLLIPOP ICON ──────────────────────────────

function drawDumDum(ctx, x, y, size) {
  // Stick
  ctx.strokeStyle = '#E0E0E0';
  ctx.lineWidth = size * 0.15;
  ctx.beginPath();
  ctx.moveTo(x, y + size * 0.4);
  ctx.lineTo(x, y + size * 1.2);
  ctx.stroke();
  // Candy ball
  ctx.fillStyle = '#FF5252';
  ctx.beginPath();
  ctx.arc(x, y, size * 0.4, 0, Math.PI * 2);
  ctx.fill();
  // Swirl
  ctx.strokeStyle = '#FFCDD2';
  ctx.lineWidth = size * 0.08;
  ctx.beginPath();
  ctx.arc(x, y, size * 0.2, 0, Math.PI * 1.5);
  ctx.stroke();
}

// ─── PARTICLE / CELEBRATION EFFECTS ──────────────────────

function drawStarBurst(ctx, x, y, time, startTime, duration) {
  const elapsed = time - startTime;
  if (elapsed < 0 || elapsed > duration) return false;
  const progress = elapsed / duration;

  const numStars = 12;
  for (let i = 0; i < numStars; i++) {
    const angle = (i / numStars) * Math.PI * 2;
    const dist = progress * 80;
    const alpha = 1 - progress;
    const sx = x + Math.cos(angle) * dist;
    const sy = y + Math.sin(angle) * dist;

    ctx.fillStyle = `rgba(255, 235, 59, ${alpha})`;
    ctx.font = `${14 - progress * 8}px sans-serif`;
    ctx.textAlign = 'center';
    ctx.fillText('★', sx, sy);
  }
  return true; // still active
}
