// world.js — Multi-map system with overworld + house interiors

// Map tile legend:
// 0=grass, 1=path, 2=water, 3=wall, 4=tree, 5=flower, 6=houseWall,
// 7=houseRoof, 8=door, 9=window, 10=fence, 11=sign, 12=bridge, 13=chest
// 14=floor (interior), 15=rug, 16=table, 17=bookshelf

// ─── NEW INTERIOR TILES ──────────────────────────────────
// We register these in sprites.js via TILE_TYPES, but define IDs here:
// 14=wood floor, 15=rug, 16=table, 17=bookshelf

// ─── MAP DEFINITIONS ─────────────────────────────────────

const MAPS = {
  overworld: {
    id: 'overworld',
    width: 30,
    height: 25,
    tiles: [
      [4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4],
      [4,4,4,0,0,5,0,0,4,4,4,0,0,0,1,1,0,0,4,4,4,0,0,5,0,0,0,4,4,4],
      [4,0,0,0,5,0,0,0,0,4,0,0,5,0,1,1,0,5,0,4,0,0,0,0,5,0,0,0,0,4],
      [4,0,5,0,0,0,0,5,0,0,0,0,0,0,1,1,0,0,0,0,0,7,7,7,7,0,0,5,0,4],
      [4,0,0,0,0,0,0,0,0,0,5,0,0,0,1,1,0,0,0,0,0,6,9,6,6,0,0,0,0,4],
      [4,0,0,0,7,7,7,0,0,0,0,0,0,1,1,1,1,0,0,5,0,6,8,6,6,0,0,0,0,4],
      [4,0,0,0,6,9,6,0,0,0,0,0,0,1,0,0,1,0,0,0,0,0,1,0,0,0,0,0,0,4],
      [4,0,0,0,6,8,6,0,0,0,0,0,0,1,0,0,1,1,1,1,1,1,1,0,0,0,5,0,0,4],
      [4,0,5,0,0,1,0,0,0,0,0,5,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4],
      [4,0,0,0,0,1,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,13,0,0,0,4],
      [4,0,0,0,0,1,1,1,1,1,1,1,1,1,0,0,0,0,0,10,10,10,0,0,0,0,0,0,0,4],
      [4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,2,2,0,0,0,0,5,0,0,0,0,4],
      [4,0,5,0,0,11,0,0,0,5,0,0,0,0,0,2,2,2,2,2,2,0,0,0,0,0,0,5,0,4],
      [4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,2,2,2,2,0,0,0,0,0,0,0,0,4],
      [4,0,0,0,0,0,0,5,0,0,0,0,1,1,12,12,2,2,2,2,0,0,0,0,0,0,5,0,0,4],
      [4,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,2,2,0,0,0,0,7,7,7,0,0,0,4],
      [4,0,0,5,0,0,0,0,0,0,0,0,1,0,0,5,0,0,0,0,0,0,0,6,9,6,0,0,0,4],
      [4,0,0,0,0,0,0,0,5,0,0,0,1,0,0,0,0,0,0,0,5,0,0,6,8,6,0,0,0,4],
      [4,0,0,0,0,5,0,0,0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,5,0,4],
      [4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4],
      [4,0,5,0,0,0,0,0,5,0,0,0,0,5,0,0,0,5,0,0,0,0,5,0,0,0,0,5,0,4],
      [4,0,0,0,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,5,0,0,0,0,0,0,0,0,4],
      [4,0,0,0,0,0,13,0,0,0,5,0,0,0,0,0,0,0,0,0,0,0,0,0,5,0,0,0,0,4],
      [4,4,0,0,0,4,4,4,0,0,4,4,0,0,0,0,0,0,4,4,0,0,4,4,4,0,0,0,4,4],
      [4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4],
    ],
  },

  // ─── HOME (left house) ──────────────────────────────────
  home: {
    id: 'home',
    width: 10,
    height: 8,
    tiles: [
      [3,3,3,3,3,3,3,3,3,3],
      [3,14,14,14,14,14,14,14,14,3],
      [3,14,15,15,15,14,14,17,14,3],
      [3,14,15,16,15,14,14,17,14,3],
      [3,14,15,15,15,14,14,14,14,3],
      [3,14,14,14,14,14,14,14,14,3],
      [3,14,14,14,8,14,14,14,14,3],
      [3,3,3,3,3,3,3,3,3,3],
    ],
  },

  // ─── GIZMO'S LAB (east house) ───────────────────────────
  lab: {
    id: 'lab',
    width: 12,
    height: 9,
    tiles: [
      [3,3,3,3,3,3,3,3,3,3,3,3],
      [3,14,14,17,17,14,14,17,17,14,14,3],
      [3,14,14,14,14,14,14,14,14,14,14,3],
      [3,14,16,14,14,14,14,14,14,16,14,3],
      [3,14,14,14,15,15,15,15,14,14,14,3],
      [3,14,14,14,15,13,13,15,14,14,14,3],
      [3,14,14,14,14,14,14,14,14,14,14,3],
      [3,14,14,14,14,8,14,14,14,14,14,3],
      [3,3,3,3,3,3,3,3,3,3,3,3],
    ],
  },

  // ─── SHOP (south house) ─────────────────────────────────
  shop: {
    id: 'shop',
    width: 10,
    height: 8,
    tiles: [
      [3,3,3,3,3,3,3,3,3,3],
      [3,14,17,17,14,14,17,17,14,3],
      [3,14,14,14,14,14,14,14,14,3],
      [3,14,14,16,16,16,16,14,14,3],
      [3,14,14,14,14,14,14,14,14,3],
      [3,14,14,15,15,15,15,14,14,3],
      [3,14,14,14,8,14,14,14,14,3],
      [3,3,3,3,3,3,3,3,3,3],
    ],
  },

  // ─── DREAM WORLD (palette-swapped overworld) ────────────
  dream: {
    id: 'dream',
    width: 30,
    height: 25,
    // Same layout as overworld — palette swap handled in rendering
    tiles: null, // loaded dynamically from overworld
    renderMode: 'dream',
  },

  // ─── DOGHOUSE LAND (glitch zone) ───────────────────────
  // Small map with intentionally broken tile indices for glitch aesthetic
  // Tile IDs > 17 get rendered as "glitch tiles" by the renderer
  doghouse: {
    id: 'doghouse',
    width: 16,
    height: 12,
    renderMode: 'glitch',
    tiles: [
      [99,99,99,99,99,99,99,99,99,99,99,99,99,99,99,99],
      [99,14,98,98,97,14,96,96,14,95,95,14,98,14,14,99],
      [99,98,14,14,14,97,14,14,96,14,14,95,14,14,98,99],
      [99,14,14,14,14,14,14,14,14,14,14,14,14,14,14,99],
      [99,97,14,14,16,14,14,14,14,14,14,16,14,14,97,99],
      [99,14,14,14,14,14,15,15,15,14,14,14,14,14,14,99],
      [99,96,14,14,14,14,15,13,15,14,14,14,14,14,96,99],
      [99,14,14,14,14,14,15,15,15,14,14,14,14,14,14,99],
      [99,95,14,14,14,14,14,14,14,14,14,14,14,14,95,99],
      [99,14,14,14,14,14,14,14,14,14,14,14,14,14,14,99],
      [99,14,98,14,14,97,14,8,14,96,14,14,95,14,14,99],
      [99,99,99,99,99,99,99,99,99,99,99,99,99,99,99,99],
    ],
  },

  // ─── HIDDEN GROVE (secret area over the trees) ──────────
  grove: {
    id: 'grove',
    width: 12,
    height: 10,
    tiles: [
      [4,4,4,4,4,4,4,4,4,4,4,4],
      [4,5,0,0,5,0,0,5,0,0,5,4],
      [4,0,0,5,0,0,0,0,5,0,0,4],
      [4,0,5,0,0,13,13,0,0,5,0,4],
      [4,5,0,0,0,5,5,0,0,0,5,4],
      [4,0,0,0,5,0,0,5,0,0,0,4],
      [4,0,5,0,0,0,0,0,0,5,0,4],
      [4,5,0,0,0,0,0,0,0,0,5,4],
      [4,0,0,5,0,1,0,5,0,0,0,4],
      [4,4,4,4,4,4,4,4,4,4,4,4],
    ],
  },
};

// ─── DOOR/PORTAL CONNECTIONS ─────────────────────────────
// When player steps on a door tile, check this list for transitions
const PORTALS = [
  // Overworld doors → interiors
  { fromMap: 'overworld', fromX: 5,  fromY: 7,  toMap: 'home', toX: 4, toY: 5, dir: DIR.up },
  { fromMap: 'overworld', fromX: 22, fromY: 5,  toMap: 'lab',  toX: 5, toY: 6, dir: DIR.up },
  { fromMap: 'overworld', fromX: 24, fromY: 17, toMap: 'shop', toX: 4, toY: 5, dir: DIR.up },
  // Interior doors → overworld
  { fromMap: 'home', fromX: 4, fromY: 6, toMap: 'overworld', toX: 5,  toY: 8,  dir: DIR.down },
  { fromMap: 'lab',  fromX: 5, fromY: 7, toMap: 'overworld', toX: 22, toY: 6,  dir: DIR.down },
  { fromMap: 'shop', fromX: 4, fromY: 6, toMap: 'overworld', toX: 24, toY: 18, dir: DIR.down },

  // ─── SECRET PORTALS ──────────────────────────────────────

  // Dream World: water tile just past the bridge — walk off the bridge into the water
  { fromMap: 'overworld', fromX: 16, fromY: 14, toMap: 'dream', toX: 14, toY: 14, dir: DIR.down },
  // Dream World exit: walk back to same bridge spot
  { fromMap: 'dream', fromX: 14, fromY: 14, toMap: 'overworld', toX: 15, toY: 14, dir: DIR.up },

  // Doghouse Land: back of home roof — walk into the roof tile at (5,5) from above
  { fromMap: 'overworld', fromX: 5, fromY: 5, toMap: 'doghouse', toX: 7, toY: 1, dir: DIR.down },
  // Doghouse exit: spit back out above the house
  { fromMap: 'doghouse', fromX: 7, fromY: 10, toMap: 'overworld', toX: 5, toY: 4, dir: DIR.down },

  // Hidden Grove: top tree border at col 15 — walk up off the map
  { fromMap: 'overworld', fromX: 15, fromY: 0, toMap: 'grove', toX: 5, toY: 8, dir: DIR.up },
  // Grove exit: the path tile at bottom of grove
  { fromMap: 'grove', fromX: 5, fromY: 8, toMap: 'overworld', toX: 15, toY: 1, dir: DIR.down },
];

// ─── ACTIVE MAP STATE ────────────────────────────────────
// The game reads from MAP (mutable reference to current map data)

const MAP = {
  id: 'overworld',
  width: 30,
  height: 25,
  tiles: [],
  renderMode: null,
};

function loadMap(mapId) {
  const src = MAPS[mapId];
  if (!src) return;
  MAP.id = src.id;
  MAP.width = src.width;
  MAP.height = src.height;
  MAP.renderMode = src.renderMode || null;

  // Dream world uses overworld tiles
  if (mapId === 'dream') {
    MAP.tiles = MAPS.overworld.tiles.map(row => [...row]);
    MAP.width = MAPS.overworld.width;
    MAP.height = MAPS.overworld.height;
  } else {
    MAP.tiles = src.tiles.map(row => [...row]);
  }
}

// ─── NPC DEFINITIONS (per map) ───────────────────────────

const NPC_DEFS_BY_MAP = {
  overworld: [
    {
      id: 'sage',
      name: 'Professor Gizmo',
      tileX: 12,
      tileY: 12,
      spriteFn: 'sage',
      dialogueContext: 'Professor Gizmo is a wise and silly wizard who stands by the sign near the pond. He loves math puzzles and always has a tricky (but fair) math question. He speaks dramatically.',
    },
  ],
  home: [
    {
      id: 'mommy',
      name: 'Mommy',
      tileX: 3,
      tileY: 3,
      spriteFn: 'mommy',
      dialogueContext: 'Mommy is at home. She loves the player and says encouraging things. She might ask a gentle math question or just give a hug.',
    },
    {
      id: 'kid_1',
      name: 'Tali',
      tileX: 6,
      tileY: 5,
      spriteFn: 'kid1',
      canReceiveGifts: true,
      neverChallenge: true,
      dialogueContext: 'A playful kid who loves games and gets excited about everything.',
    },
    {
      id: 'kid_2',
      name: 'Noa',
      tileX: 8,
      tileY: 5,
      spriteFn: 'kid2',
      canReceiveGifts: true,
      neverChallenge: true,
      dialogueContext: 'A shy but curious kid who asks lots of questions.',
    },
  ],
  lab: [
    {
      id: 'sage_lab',
      name: 'Professor Gizmo',
      tileX: 5,
      tileY: 3,
      spriteFn: 'sage',
      dialogueContext: 'Professor Gizmo is in his lab surrounded by books and potions. He has extra-hard challenges here! He is very dramatic about his experiments.',
    },
  ],
  shop: [
    {
      id: 'shopkeeper',
      name: 'Bolt the Shopkeeper',
      tileX: 5,
      tileY: 2,
      spriteFn: 'sage',
      dialogueContext: 'Bolt runs the Dum Dum shop. He sells silly things for Dum Dums. He talks like a friendly merchant.',
    },
  ],
  dream: [
    {
      id: 'dream_sage',
      name: '???',
      tileX: 15,
      tileY: 8,
      spriteFn: 'sage',
      dialogueContext: 'A mysterious dream version of Professor Gizmo. He speaks in riddles and says cryptic, poetic things about numbers and letters. Everything he says sounds like a dream.',
    },
  ],
  doghouse: [
    {
      id: 'glitch_dog',
      name: 'B0RK.exe',
      tileX: 7,
      tileY: 5,
      spriteFn: 'dog',
      dialogueContext: 'A glitchy dog NPC in the corrupted doghouse dimension. Speaks in a mix of normal dog talk and corrupted text. Says things like "BORK BORK... sys.treat.exe... good boy OVERFLOW". Very friendly despite being glitched.',
    },
  ],
  grove: [
    {
      id: 'grove_spirit',
      name: 'Old Oak',
      tileX: 6,
      tileY: 4,
      spriteFn: 'sage',
      dialogueContext: 'A wise ancient tree spirit in the Hidden Grove. Speaks slowly and kindly. Amazed that someone found this secret place. Gives extra-rewarding challenges.',
    },
  ],
};

// Keep backward compat — NPC_DEFS is referenced elsewhere
let NPC_DEFS = NPC_DEFS_BY_MAP.overworld;

// ─── AREAS (per map, for AI context) ─────────────────────

const AREAS_BY_MAP = {
  overworld: [
    { name: 'Home',          x1: 3, y1: 3, x2: 8, y2: 9 },
    { name: 'Main Path',     x1: 9, y1: 1, x2: 18, y2: 10 },
    { name: 'Pond',          x1: 14, y1: 11, x2: 22, y2: 15 },
    { name: 'East House',    x1: 20, y1: 3, x2: 26, y2: 7 },
    { name: 'South House',   x1: 22, y1: 15, x2: 27, y2: 19 },
    { name: 'Forest Edge',   x1: 1, y1: 10, x2: 5, y2: 22 },
    { name: 'South Meadow',  x1: 6, y1: 15, x2: 14, y2: 22 },
    { name: 'Treasure Woods', x1: 22, y1: 8, x2: 28, y2: 14 },
  ],
  home: [{ name: 'Home (Inside)', x1: 0, y1: 0, x2: 10, y2: 8 }],
  lab:  [{ name: "Gizmo's Lab", x1: 0, y1: 0, x2: 12, y2: 9 }],
  shop:     [{ name: 'Dum Dum Shop', x1: 0, y1: 0, x2: 10, y2: 8 }],
  dream:    [{ name: 'The Dream', x1: 0, y1: 0, x2: 30, y2: 25 }],
  doghouse: [{ name: 'D0GH0USE.exe', x1: 0, y1: 0, x2: 16, y2: 12 }],
  grove:    [{ name: 'Hidden Grove', x1: 0, y1: 0, x2: 12, y2: 10 }],
};

function getAreaName(tileX, tileY) {
  const areas = AREAS_BY_MAP[MAP.id] || [];
  for (const area of areas) {
    if (tileX >= area.x1 && tileX <= area.x2 && tileY >= area.y1 && tileY <= area.y2) {
      return area.name;
    }
  }
  return MAP.id === 'overworld' ? 'The Wild' : MAP.id;
}

// ─── PORTAL CHECK ────────────────────────────────────────

function checkPortal(tileX, tileY) {
  return PORTALS.find(p => p.fromMap === MAP.id && p.fromX === tileX && p.fromY === tileY) || null;
}

function executePortal(portal) {
  loadMap(portal.toMap);
  NPC_DEFS = NPC_DEFS_BY_MAP[portal.toMap] || [];
  initNPCs();

  PLAYER.tileX = portal.toX;
  PLAYER.tileY = portal.toY;
  PLAYER.pixelX = portal.toX * TILE_SIZE;
  PLAYER.pixelY = portal.toY * TILE_SIZE;
  PLAYER.dir = portal.dir;
  PLAYER.moving = false;

  // Place robot next to player
  const robotOffsets = [
    { dx: 0, dy: 1 }, { dx: -1, dy: 0 }, { dx: 1, dy: 0 }, { dx: 0, dy: -1 }
  ];
  for (const off of robotOffsets) {
    const rx = portal.toX + off.dx;
    const ry = portal.toY + off.dy;
    if (!isTileSolid(rx, ry) && !NPCS.some(n => n.tileX === rx && n.tileY === ry)) {
      ROBOT.tileX = rx;
      ROBOT.tileY = ry;
      ROBOT.pixelX = rx * TILE_SIZE;
      ROBOT.pixelY = ry * TILE_SIZE;
      break;
    }
  }
  ROBOT.followQueue = [];
  ROBOT.moving = false;

  // Secret area entry dialogue — lock movement immediately to prevent
  // the player from walking out before the dialogue box appears
  if (portal.toMap === 'dream' || portal.toMap === 'doghouse' || portal.toMap === 'grove') {
    if (typeof GAME !== 'undefined') GAME.state = 'DIALOGUE';
  }
  if (portal.toMap === 'dream') {
    setTimeout(() => {
      startDialogue([
        { speaker: 'Sparky', text: 'Bzzzzt... where... are we? Everything looks... DREAMY...' },
        { speaker: 'Sparky', text: 'My circuits feel all tingly! Is this a dream?! BEEP BOOP DREAM MODE!' },
      ], () => { if (typeof GAME !== 'undefined') GAME.state = 'PLAYING'; });
    }, 300);
  } else if (portal.toMap === 'doghouse') {
    setTimeout(() => {
      startDialogue([
        { speaker: 'Sparky', text: 'W-WHAT?! ERROR ERROR! My v-v-visual sensors are GLITCHING!' },
        { speaker: 'Sparky', text: 'Boss... I think we walked through a WALL! This place is... b r o k e n...' },
        { speaker: 'Sparky', text: 'I hear barking... and my code is full of BORK!' },
      ], () => { if (typeof GAME !== 'undefined') GAME.state = 'PLAYING'; });
    }, 300);
  } else if (portal.toMap === 'grove') {
    setTimeout(() => {
      startDialogue([
        { speaker: 'Sparky', text: 'WAIT... how did we get here?! My GPS is TOTALLY BROKEN!' },
        { speaker: 'Sparky', text: 'We walked OVER the trees?! Boss, you are a GENIUS!' },
        { speaker: 'Sparky', text: 'Ooooh... this place is beautiful... and SECRET!' },
      ], () => { if (typeof GAME !== 'undefined') GAME.state = 'PLAYING'; });
    }, 300);
  }
}

// ─── TILE COLLISION ──────────────────────────────────────

// Secret walkable spots — tiles that are normally solid but act as secret portals
const SECRET_WALKABLE = [
  // Dream World entrance: water tile just past the bridge end
  { map: 'overworld', x: 16, y: 14 },
  // Doghouse Land entrance: middle roof tile on back of home house
  { map: 'overworld', x: 5, y: 5 },
  // Hidden Grove entrance: tree at top border
  { map: 'overworld', x: 15, y: 0 },
];

function isTileSolid(tileX, tileY) {
  if (tileX < 0 || tileY < 0 || tileX >= MAP.width || tileY >= MAP.height) return true;
  const tileId = MAP.tiles[tileY][tileX];
  const tileType = TILE_TYPES[tileId];
  // Door tiles are always walkable (portals)
  if (tileId === 8) return false;
  // Glitch tiles (IDs >= 95) in doghouse — border ones are solid, inner are walkable
  if (tileId >= 95) return tileId === 99;
  if (!tileType) return true;
  // Check secret walkable overrides
  if (SECRET_WALKABLE.some(s => s.map === MAP.id && s.x === tileX && s.y === tileY)) return false;
  return tileType.solid;
}

// ─── CAMERA ──────────────────────────────────────────────

const CAMERA = {
  x: 0,
  y: 0,
  viewW: 20,
  viewH: 15,
};

function updateCamera(playerPixelX, playerPixelY, canvasW, canvasH) {
  const targetX = playerPixelX - canvasW / 2 + TILE_SIZE / 2;
  const targetY = playerPixelY - canvasH / 2 + TILE_SIZE / 2;

  const maxX = MAP.width * TILE_SIZE - canvasW;
  const maxY = MAP.height * TILE_SIZE - canvasH;

  CAMERA.x = Math.max(0, Math.min(maxX, targetX));
  CAMERA.y = Math.max(0, Math.min(maxY, targetY));
}

function renderMap(ctx, canvasW, canvasH, time) {
  const startCol = Math.floor(CAMERA.x / TILE_SIZE);
  const startRow = Math.floor(CAMERA.y / TILE_SIZE);
  const endCol = Math.min(MAP.width - 1, startCol + Math.ceil(canvasW / TILE_SIZE) + 1);
  const endRow = Math.min(MAP.height - 1, startRow + Math.ceil(canvasH / TILE_SIZE) + 1);

  // Dream world: apply palette swap via compositing
  if (MAP.renderMode === 'dream') {
    // Draw normally first
    for (let row = startRow; row <= endRow; row++) {
      for (let col = startCol; col <= endCol; col++) {
        const px = col * TILE_SIZE - CAMERA.x;
        const py = row * TILE_SIZE - CAMERA.y;
        const tileId = MAP.tiles[row]?.[col] ?? 0;
        drawTile(ctx, tileId, px, py, time);
      }
    }
    // Apply dream color overlay via hue rotation
    ctx.save();
    ctx.globalCompositeOperation = 'hue';
    ctx.fillStyle = `hsl(${(time * 15) % 360}, 70%, 50%)`;
    ctx.fillRect(0, 0, canvasW, canvasH);
    ctx.globalCompositeOperation = 'source-over';
    // Dreamy sparkle overlay
    ctx.globalAlpha = 0.15 + Math.sin(time * 0.5) * 0.1;
    for (let i = 0; i < 30; i++) {
      const sx = seededRandom(i, 0, 777) * canvasW;
      const sy = seededRandom(i, 1, 777) * canvasH;
      const twinkle = Math.sin(time * 3 + i * 1.7) * 0.5 + 0.5;
      ctx.fillStyle = `rgba(255, 200, 255, ${twinkle})`;
      ctx.beginPath();
      ctx.arc(sx + Math.sin(time + i) * 10, sy + Math.cos(time * 0.7 + i) * 10, 3, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
    ctx.restore();
    return;
  }

  // Glitch world: corrupted tile rendering
  if (MAP.renderMode === 'glitch') {
    renderGlitchMap(ctx, canvasW, canvasH, time, startCol, startRow, endCol, endRow);
    return;
  }

  // Normal rendering
  for (let row = startRow; row <= endRow; row++) {
    for (let col = startCol; col <= endCol; col++) {
      const px = col * TILE_SIZE - CAMERA.x;
      const py = row * TILE_SIZE - CAMERA.y;
      const tileId = MAP.tiles[row]?.[col] ?? 0;
      drawTile(ctx, tileId, px, py, time);
    }
  }
}

// ─── GLITCH MAP RENDERER (Doghouse Land) ─────────────────

function renderGlitchMap(ctx, canvasW, canvasH, time, startCol, startRow, endCol, endRow) {
  for (let row = startRow; row <= endRow; row++) {
    for (let col = startCol; col <= endCol; col++) {
      const px = col * TILE_SIZE - CAMERA.x;
      const py = row * TILE_SIZE - CAMERA.y;
      const tileId = MAP.tiles[row]?.[col] ?? 0;

      if (tileId >= 95) {
        // Glitch tile — draw corrupted visual
        drawGlitchTile(ctx, px, py, col, row, tileId, time);
      } else {
        // Normal tile (floor, rug, table etc inside doghouse)
        drawTile(ctx, tileId, px, py, time);
        // But with a slight color shift
        ctx.save();
        ctx.globalCompositeOperation = 'color';
        ctx.fillStyle = `hsla(${(row * 37 + col * 53) % 360}, 50%, 40%, 0.25)`;
        ctx.fillRect(px, py, TILE_SIZE, TILE_SIZE);
        ctx.restore();
      }
    }
  }

  // Scanline overlay for CRT glitch feel
  ctx.save();
  ctx.globalAlpha = 0.06;
  for (let y = 0; y < canvasH; y += 3) {
    ctx.fillStyle = '#000';
    ctx.fillRect(0, y, canvasW, 1);
  }
  ctx.globalAlpha = 1;

  // Occasional screen tear — horizontal offset for a few rows
  const tearY = Math.floor((time * 60) % canvasH);
  const tearH = 4 + Math.floor(Math.sin(time * 7) * 3);
  const tearOffset = Math.sin(time * 13) * 8;
  if (Math.sin(time * 2.3) > 0.7) {
    ctx.drawImage(ctx.canvas, 0, tearY, canvasW, tearH, tearOffset, tearY, canvasW, tearH);
  }
  ctx.restore();
}

function drawGlitchTile(ctx, px, py, col, row, tileId, time) {
  // Each glitch tile ID maps to a different corruption style
  // Use position-seeded randomness for stable but broken-looking tiles
  const seed = col * 31 + row * 97 + tileId;
  const variant = seededRandom(col, row, seed) * 6 | 0;

  // Flicker: occasionally swap what's drawn
  const flicker = Math.sin(time * 5 + seed) > 0.85;

  switch (flicker ? (variant + 3) % 6 : variant) {
    case 0: // Wrong-palette tree fragment
      ctx.fillStyle = `hsl(${(seed * 47) % 360}, 60%, 30%)`;
      ctx.fillRect(px, py, TILE_SIZE, TILE_SIZE);
      // Half a tree canopy in wrong color
      ctx.fillStyle = `hsl(${(seed * 47 + 120) % 360}, 70%, 45%)`;
      ctx.beginPath();
      ctx.arc(px + TILE_SIZE * 0.7, py + TILE_SIZE * 0.4, 14, 0, Math.PI * 2);
      ctx.fill();
      break;

    case 1: // Corrupted text/number fragments
      ctx.fillStyle = `hsl(${(seed * 23) % 360}, 40%, 15%)`;
      ctx.fillRect(px, py, TILE_SIZE, TILE_SIZE);
      ctx.fillStyle = `hsl(${(seed * 23 + 180) % 360}, 80%, 60%)`;
      ctx.font = `${12 + (seed % 16)}px monospace`;
      ctx.textAlign = 'center';
      const glitchChars = '!@#$%^&*BORK01█▓░▒';
      const c1 = glitchChars[(seed * 3) % glitchChars.length];
      const c2 = glitchChars[(seed * 7 + 1) % glitchChars.length];
      ctx.fillText(c1 + c2, px + TILE_SIZE / 2, py + TILE_SIZE / 2 + 4);
      break;

    case 2: // Horizontal stripe corruption (repeating tile band)
      for (let stripe = 0; stripe < 4; stripe++) {
        const hue = ((row * 60 + stripe * 90 + seed * 11) % 360);
        ctx.fillStyle = `hsl(${hue}, ${40 + stripe * 10}%, ${20 + stripe * 8}%)`;
        ctx.fillRect(px, py + stripe * 12, TILE_SIZE, 12);
      }
      break;

    case 3: // Wrong-palette water/house mashup
      ctx.fillStyle = '#1a0030';
      ctx.fillRect(px, py, TILE_SIZE, TILE_SIZE);
      // Brick pattern in wrong color
      ctx.strokeStyle = `hsl(${(seed * 71) % 360}, 70%, 50%)`;
      ctx.lineWidth = 1;
      for (let r = 0; r < 3; r++) {
        ctx.strokeRect(px + 2, py + r * 16 + 2, TILE_SIZE - 4, 14);
      }
      break;

    case 4: // Paw print corruption (dog theme!)
      ctx.fillStyle = `hsl(${(seed * 19) % 360}, 30%, 18%)`;
      ctx.fillRect(px, py, TILE_SIZE, TILE_SIZE);
      ctx.fillStyle = `hsl(${(seed * 19 + 40) % 360}, 50%, 40%)`;
      // Paw pad
      ctx.beginPath();
      ctx.ellipse(px + 24, py + 28, 8, 10, 0, 0, Math.PI * 2);
      ctx.fill();
      // Toes
      for (let t = 0; t < 3; t++) {
        ctx.beginPath();
        ctx.arc(px + 14 + t * 10, py + 16, 5, 0, Math.PI * 2);
        ctx.fill();
      }
      break;

    case 5: // Static noise block
      for (let ny = 0; ny < TILE_SIZE; ny += 4) {
        for (let nx = 0; nx < TILE_SIZE; nx += 4) {
          const bright = seededRandom(px + nx, py + ny, Math.floor(time * 8)) * 60;
          ctx.fillStyle = `hsl(${(seed * 31) % 360}, 20%, ${bright}%)`;
          ctx.fillRect(px + nx, py + ny, 4, 4);
        }
      }
      break;
  }
}
