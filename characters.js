// characters.js — Player, Robot companion, NPC management

// ─── PLAYER ──────────────────────────────────────────────

const PLAYER = {
  tileX: 5,
  tileY: 8,
  pixelX: 5 * TILE_SIZE,
  pixelY: 8 * TILE_SIZE,
  dir: DIR.down,
  frame: 0,
  moving: false,
  moveProgress: 0,
  moveFromX: 0,
  moveFromY: 0,
  moveToX: 0,
  moveToY: 0,
  moveSpeed: 5, // tiles per second (controls how fast the slide is)
  stepTimer: 0,
  stepHeld: false,
};

const MOVE_DIRS = {
  ArrowUp:    { dx: 0,  dy: -1, dir: DIR.up },
  ArrowDown:  { dx: 0,  dy: 1,  dir: DIR.down },
  ArrowLeft:  { dx: -1, dy: 0,  dir: DIR.left },
  ArrowRight: { dx: 1,  dy: 0,  dir: DIR.right },
  w: { dx: 0,  dy: -1, dir: DIR.up },
  s: { dx: 0,  dy: 1,  dir: DIR.down },
  a: { dx: -1, dy: 0,  dir: DIR.left },
  d: { dx: 1,  dy: 0,  dir: DIR.right },
};

function tryMovePlayer(key) {
  if (PLAYER.moving) return false;
  const move = MOVE_DIRS[key];
  if (!move) return false;

  PLAYER.dir = move.dir;

  const newX = PLAYER.tileX + move.dx;
  const newY = PLAYER.tileY + move.dy;

  // Check tile collision
  if (isTileSolid(newX, newY)) return false;

  // Check NPC collision
  if (NPCS.some(n => n.tileX === newX && n.tileY === newY)) return false;

  // Start movement
  PLAYER.moving = true;
  PLAYER.moveProgress = 0;
  PLAYER.moveFromX = PLAYER.tileX * TILE_SIZE;
  PLAYER.moveFromY = PLAYER.tileY * TILE_SIZE;
  PLAYER.moveToX = newX * TILE_SIZE;
  PLAYER.moveToY = newY * TILE_SIZE;
  PLAYER.tileX = newX;
  PLAYER.tileY = newY;
  PLAYER.frame = (PLAYER.frame + 1) % 4;

  // Push position to robot's follow queue
  pushRobotTarget(PLAYER.moveFromX / TILE_SIZE, PLAYER.moveFromY / TILE_SIZE);

  return true;
}

function updatePlayer(dt, keys) {
  if (PLAYER.moving) {
    PLAYER.moveProgress += dt * PLAYER.moveSpeed;
    if (PLAYER.moveProgress >= 1) {
      PLAYER.moveProgress = 1;
      PLAYER.moving = false;
      PLAYER.pixelX = PLAYER.moveToX;
      PLAYER.pixelY = PLAYER.moveToY;

      // Check for portal transition
      const portal = checkPortal(PLAYER.tileX, PLAYER.tileY);
      if (portal) {
        executePortal(portal);
        return;
      }
    } else {
      PLAYER.pixelX = PLAYER.moveFromX + (PLAYER.moveToX - PLAYER.moveFromX) * PLAYER.moveProgress;
      PLAYER.pixelY = PLAYER.moveFromY + (PLAYER.moveToY - PLAYER.moveFromY) * PLAYER.moveProgress;
    }
  }

  // Handle held keys for continuous movement
  if (!PLAYER.moving) {
    for (const key of ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'w', 'a', 's', 'd']) {
      if (keys[key]) {
        tryMovePlayer(key);
        break;
      }
    }
  }
}

function renderPlayer(ctx, time) {
  const screenX = PLAYER.pixelX - CAMERA.x;
  const screenY = PLAYER.pixelY - CAMERA.y;
  drawPlayer(ctx, screenX, screenY, PLAYER.dir, PLAYER.frame, time);
}

// ─── ROBOT COMPANION ─────────────────────────────────────

const ROBOT = {
  tileX: 5,
  tileY: 9,
  pixelX: 5 * TILE_SIZE,
  pixelY: 9 * TILE_SIZE,
  dir: DIR.up,
  frame: 0,
  moving: false,
  moveProgress: 0,
  moveFromX: 0,
  moveFromY: 0,
  moveToX: 0,
  moveToY: 0,
  moveSpeed: 5,
  followQueue: [], // queue of {x, y} tile positions the player has been
};

function pushRobotTarget(tileX, tileY) {
  ROBOT.followQueue.push({ x: tileX, y: tileY });
  // Keep queue reasonable
  if (ROBOT.followQueue.length > 20) {
    ROBOT.followQueue.shift();
  }
}

function updateRobot(dt) {
  if (ROBOT.moving) {
    ROBOT.moveProgress += dt * ROBOT.moveSpeed;
    if (ROBOT.moveProgress >= 1) {
      ROBOT.moveProgress = 1;
      ROBOT.moving = false;
      ROBOT.pixelX = ROBOT.moveToX;
      ROBOT.pixelY = ROBOT.moveToY;
    } else {
      ROBOT.pixelX = ROBOT.moveFromX + (ROBOT.moveToX - ROBOT.moveFromX) * ROBOT.moveProgress;
      ROBOT.pixelY = ROBOT.moveFromY + (ROBOT.moveToY - ROBOT.moveFromY) * ROBOT.moveProgress;
    }
  }

  // If not moving and there's somewhere to go
  if (!ROBOT.moving && ROBOT.followQueue.length > 0) {
    // Only follow if player is far enough away
    const distX = Math.abs(PLAYER.tileX - ROBOT.tileX);
    const distY = Math.abs(PLAYER.tileY - ROBOT.tileY);
    if (distX + distY >= 2 || ROBOT.followQueue.length > 3) {
      const target = ROBOT.followQueue.shift();

      // Determine direction
      const dx = target.x - ROBOT.tileX;
      const dy = target.y - ROBOT.tileY;
      if (dx < 0) ROBOT.dir = DIR.left;
      else if (dx > 0) ROBOT.dir = DIR.right;
      else if (dy < 0) ROBOT.dir = DIR.up;
      else if (dy > 0) ROBOT.dir = DIR.down;

      // Check if we can move there (skip if blocked)
      if (!isTileSolid(target.x, target.y) && !NPCS.some(n => n.tileX === target.x && n.tileY === target.y)) {
        ROBOT.moving = true;
        ROBOT.moveProgress = 0;
        ROBOT.moveFromX = ROBOT.tileX * TILE_SIZE;
        ROBOT.moveFromY = ROBOT.tileY * TILE_SIZE;
        ROBOT.moveToX = target.x * TILE_SIZE;
        ROBOT.moveToY = target.y * TILE_SIZE;
        ROBOT.tileX = target.x;
        ROBOT.tileY = target.y;
        ROBOT.frame = (ROBOT.frame + 1) % 4;
      }
    }
  }
}

function renderRobot(ctx, time) {
  const screenX = ROBOT.pixelX - CAMERA.x;
  const screenY = ROBOT.pixelY - CAMERA.y;
  drawRobot(ctx, screenX, screenY, ROBOT.dir, ROBOT.frame, time);
}

// ─── NPCs ────────────────────────────────────────────────

const SPRITE_FNS = {
  mommy: drawMommy,
  sage: drawSage,
  dog: drawDog,
};

let NPCS = [];

function initNPCs() {
  NPCS = NPC_DEFS.map(def => ({
    ...def,
    dir: DIR.down,
    frame: 0,
  }));
}

function renderNPCs(ctx, time) {
  for (const npc of NPCS) {
    const screenX = npc.tileX * TILE_SIZE - CAMERA.x;
    const screenY = npc.tileY * TILE_SIZE - CAMERA.y;
    const drawFn = SPRITE_FNS[npc.spriteFn];
    if (drawFn) {
      drawFn(ctx, screenX, screenY, npc.dir, npc.frame, time);
    }
  }
}

// ─── INTERACTION ─────────────────────────────────────────

function getFacingTile() {
  switch (PLAYER.dir) {
    case DIR.up:    return { x: PLAYER.tileX, y: PLAYER.tileY - 1 };
    case DIR.down:  return { x: PLAYER.tileX, y: PLAYER.tileY + 1 };
    case DIR.left:  return { x: PLAYER.tileX - 1, y: PLAYER.tileY };
    case DIR.right: return { x: PLAYER.tileX + 1, y: PLAYER.tileY };
  }
}

function getNPCAtTile(tx, ty) {
  return NPCS.find(n => n.tileX === tx && n.tileY === ty) || null;
}

function getInteractTarget() {
  const facing = getFacingTile();

  // Check for NPC
  const npc = getNPCAtTile(facing.x, facing.y);
  if (npc) return { type: 'npc', npc };

  // Check for sign
  const tileId = MAP.tiles[facing.y]?.[facing.x];
  if (tileId === 11) return { type: 'sign', x: facing.x, y: facing.y };

  // Check for chest
  if (tileId === 13) return { type: 'chest', x: facing.x, y: facing.y };

  // Check if facing the robot
  if (ROBOT.tileX === facing.x && ROBOT.tileY === facing.y) {
    return { type: 'robot' };
  }

  return null;
}
