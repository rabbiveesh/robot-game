# Dev Zone — Developer Debug Page

## Access

Name a save file `justinbailey`. On load, the game enters the Dev Zone instead of the normal game. The Dev Zone is a scrollable gallery of every visual component in the game, rendered with sample data.

```js
// In loadFromSlot or initGame:
if (playerName.toLowerCase().replace(/\s/g, '') === 'justinbailey') {
  GAME.state = 'DEV_ZONE';
  return;
}
```

## The "Can't Forget" Pattern: Visual Registry

The Dev Zone doesn't maintain its own list of things to render. It reads from registries that the GAME ITSELF uses. If a renderer isn't in the registry, it doesn't work in the game. So the Dev Zone is always complete by construction.

### Visualization Registry

Every visual method registers itself:

```js
// src/presentation/renderers/visual-registry.js

const VISUAL_REGISTRY = {};

function registerVisual(id, meta, renderFn) {
  VISUAL_REGISTRY[id] = Object.freeze({
    id,
    name: meta.name,              // human-readable: "Base-10 Blocks"
    description: meta.description, // "Tens rods + ones cubes"
    operations: meta.operations,   // which ops it supports: ['add', 'sub', 'multiply', 'divide']
    bandRange: meta.bandRange,     // [minBand, maxBand] where it makes sense
    craStage: meta.craStage,       // 'concrete' | 'representational'
    render: renderFn,              // the actual render function
  });
}

function getVisual(id) {
  return VISUAL_REGISTRY[id];
}

function getAllVisuals() {
  return Object.values(VISUAL_REGISTRY);
}
```

Each visual file registers on load:

```js
// base10-blocks-visual.js
registerVisual('base10_blocks', {
  name: 'Base-10 Blocks',
  description: 'Tens rods + ones cubes. Shows place value.',
  operations: ['add', 'sub', 'multiply', 'divide'],
  bandRange: [5, 10],
  craStage: 'concrete',
}, renderBase10Blocks);

// dots-visual.js (the existing renderDotVisual)
registerVisual('dots', {
  name: 'Counting Dots',
  description: 'Individual dots. Count them up.',
  operations: ['add', 'sub'],
  bandRange: [1, 4],
  craStage: 'concrete',
}, renderDotVisual);
```

The QuizRenderer uses the registry to pick a visual:

```js
// Instead of hardcoded band check:
const visual = getVisual(cs.renderHint.visualMethod) || getVisual('dots');
visual.render(ctx, a, b, op, answer, cx, cy, time);
```

The Dev Zone iterates `getAllVisuals()`. If someone adds a new visual file but doesn't call `registerVisual`, it won't work in the game — the bug is immediately obvious because the QuizRenderer can't find it. No way to forget.

### Renderer Registry

Same pattern for challenge renderers:

```js
// src/presentation/renderers/renderer-registry.js

const RENDERER_REGISTRY = {};

function registerRenderer(id, meta, createFn) {
  RENDERER_REGISTRY[id] = Object.freeze({
    id,
    name: meta.name,
    description: meta.description,
    createFn,  // factory function: () => renderer instance
  });
}

// quiz-renderer.js
registerRenderer('quiz', {
  name: 'Quiz (Multiple Choice)',
  description: 'Standard multiple choice with Show-me/Tell-me buttons.',
}, createQuizRenderer);

// Future:
registerRenderer('puzzle', { ... }, createPuzzleRenderer);
registerRenderer('shop', { ... }, createShopRenderer);
```

### Sprite Registry

Same pattern for character sprites:

```js
const SPRITE_REGISTRY = {};

function registerSprite(id, meta, drawFn) {
  SPRITE_REGISTRY[id] = { id, name: meta.name, drawFn };
}

// sprites.js
registerSprite('player_boy', { name: 'Player (Boy)' }, drawPlayerBoy);
registerSprite('player_girl', { name: 'Player (Girl)' }, drawPlayerGirl);
registerSprite('robot', { name: 'Sparky' }, drawRobot);
registerSprite('mommy', { name: 'Mommy' }, drawMommy);
registerSprite('kid1', { name: 'Tali' }, drawKidNPC1);
registerSprite('kid2', { name: 'Noa' }, drawKidNPC2);
// ...
```

## Dev Zone Sections

The Dev Zone is a scrollable canvas-rendered page (or DOM overlay) with sections:

### 1. Visualization Playground

An interactive sandbox where you control EVERY input and see the rendered result live.

```
┌─────────────────────────────────────────────────────────┐
│  VISUALIZATION PLAYGROUND                                │
│                                                          │
│  Operation:  [+ ] [- ] [×✓] [÷ ]                        │
│  A: [__7__]  B: [__8__]  (Answer: 56)                   │
│  Band:       [1] [2] [3] [4] [5] [6✓] [7] [8] [9] [10]│
│                                                          │
│  Visual method:                                          │
│  [dots] [ten_frame] [base10✓] [array] [number_line]     │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │                                                   │    │
│  │     [LIVE RENDER of base10 blocks for 7 × 8]     │    │
│  │                                                   │    │
│  └─────────────────────────────────────────────────┘    │
│                                                          │
│  Challenge phase:                                        │
│  [presented✓] [feedback] [teaching] [complete]           │
│                                                          │
│  Hint state:                                             │
│  [no hint✓] [show-me ×1] [show-me ×2] [told-me]        │
│                                                          │
│  Answer mode:                                            │
│  [choice✓] [eliminate] [free_input] [voice]              │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │                                                   │    │
│  │     [LIVE RENDER of full QuizRenderer              │    │
│  │      at selected phase/hint/mode]                  │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

Controls are clickable buttons on canvas (same button rendering as the game). Changing any input immediately re-renders both panels:
- **Top panel**: just the visualization method in isolation (the visual render function with your A, B, op)
- **Bottom panel**: the full QuizRenderer with a mock `challengeState` built from your selected phase, hint state, answer mode, and visual method

This lets you answer questions like:
- "What does base-10 blocks look like for 144 ÷ 12?" → set op=÷, A=144, B=12, visual=base10
- "What does the quiz panel look like after show-me twice?" → set hint=show-me×2, see the visual alongside the choices
- "What does teaching phase look like with tell-me?" → set phase=teaching, hint=told-me
- "What if the band is 1 but we force base10 blocks?" → set band=1, visual=base10, see if it still makes sense

The mock `challengeState` is constructed from the controls:

```js
function buildMockChallengeState(controls) {
  const { a, b, op, band, phase, hintLevel, toldMe, answerMode, visualMethod } = controls;

  // Generate a real challenge with the specified numbers
  const challenge = {
    correctAnswer: compute(a, b, op),
    displayText: `What is ${a} ${DISPLAY_OP[op]} ${b}?`,
    speechText: `What is ${a} ${SPEECH_OP[op]} ${b}?`,
    question: `What is ${a} ${DISPLAY_OP[op]} ${b}?`,
    operation: opToOperation(op),
    numbers: { a, b, op },
    choices: makeChoicesForAnswer(compute(a, b, op)),
    sampledBand: band,
    band,
  };

  return createChallengeState(challenge, {
    source: 'dev_zone',
    npcName: 'Sparky',
    renderHint: {
      craStage: METHOD_CRA[visualMethod] || 'concrete',
      visualMethod,
      answerMode,
      interactionType: 'quiz',
    },
  });
  // Then apply hint/phase actions to get desired state:
  // for (let i = 0; i < hintLevel; i++) state = challengeReducer(state, { type: 'SHOW_ME' });
  // if (toldMe) state = challengeReducer(state, { type: 'TELL_ME' });
  // etc.
}
```

### All Visuals At Once

Below the playground, a static comparison grid showing the SAME problem rendered by every registered visual side-by-side:

```
┌─────────────────────────────────────────────────────────┐
│  ALL VISUALS for 47 + 28  (from playground A, B, op)    │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │  dots    │  │ ten_frame│  │ base10   │              │
│  │  (47     │  │ (4 full  │  │ (4 rods  │              │
│  │  dots!!) │  │  frames) │  │  7 cubes │              │
│  └──────────┘  └──────────┘  └──────────┘              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │  array  │  │ num_line │  │ bar_model│              │
│  │  (N/A   │  │  (jump   │  │  (parts  │              │
│  │  for +) │  │  47→75)  │  │  /whole) │              │
│  └──────────┘  └──────────┘  └──────────┘              │
└─────────────────────────────────────────────────────────┘
```

Visuals that don't support the selected operation show "N/A" greyed out. Change A, B, or op in the playground → the comparison grid updates too.

### 2. Renderer Gallery

For each registered renderer, show a mock challenge at each phase:

```
┌─────────────────────────────────────────────────────────┐
│  Quiz (Multiple Choice)                                  │
│                                                          │
│  Phase: presented    [rendered]                           │
│  Phase: feedback     [rendered with "Hmm, not quite!"]   │
│  Phase: teaching     [rendered with visual + answer]      │
│  Phase: complete     [rendered with celebration]          │
│  With show-me used   [rendered with hint visual]          │
│  With voice active   [rendered with mic UI]               │
└─────────────────────────────────────────────────────────┘
```

### 3. Sprite Gallery

All registered sprites, rendered at 2x scale with animation:

```
┌─────────────────────────────────────────────────────────┐
│  Sprites                                                 │
│                                                          │
│  [Boy ↓] [Boy ←] [Boy →] [Boy ↑]   Player (Boy)        │
│  [Girl ↓] [Girl ←] [Girl →] [Girl ↑]  Player (Girl)    │
│  [Robot ↓ animated]                    Sparky            │
│  [Mommy]                               Mommy             │
│  [Kid1]                                Tali              │
│  [Kid2]                                Noa               │
│  [Sage]                                Old Oak           │
│  [Dog]                                 B0RK.exe          │
└─────────────────────────────────────────────────────────┘
```

### 4. Tile Gallery

All tile types at 2x scale:

```
┌─────────────────────────────────────────────────────────┐
│  Tiles                                                   │
│                                                          │
│  [grass] [path] [water] [tree] [flower] [house]  ...    │
│  Labels below each                                       │
└─────────────────────────────────────────────────────────┘
```

### 5. Adaptive Profile Inspector

Live view of a test profile with sliders to adjust dials and see how challenge generation changes:

```
┌─────────────────────────────────────────────────────────┐
│  Profile Inspector                                       │
│                                                          │
│  Band:       [====|====] 6                               │
│  Spread:     [===|=====] 0.4                             │
│  Scaffolding:[=|=======] 0.2                             │
│  Pace:       [========|] 0.9                             │
│                                                          │
│  Generated challenges (seed 42):                         │
│  #1  add_carry    band:6  28 + 15 = 43                  │
│  #2  sub_borrow   band:5  42 - 17 = 25                  │
│  #3  mul_trivial  band:7  2 × 9 = 18                    │
│  #4  bond_large   band:6  ? + 8 = 15                    │
│  #5  add_no_carry band:4  23 + 14 = 37                  │
│  [Regenerate]                                            │
└─────────────────────────────────────────────────────────┘
```

### 6. TTS Test

Buttons to test each speaker voice with sample text:

```
┌─────────────────────────────────────────────────────────┐
│  TTS Test                                                │
│                                                          │
│  [▶ Sparky]  "BEEP BOOP! What is 8 times 5?"           │
│  [▶ Mommy]   "You're doing great, sweetie!"             │
│  [▶ Gizmo]   "My formula needs the missing number!"     │
└─────────────────────────────────────────────────────────┘
```

## Navigation

Arrow keys or scroll to navigate sections. ESC exits to title screen. Each section has a header. Sections auto-size based on registered content — no manual layout to maintain.

## Implementation

### File: `src/presentation/dev-zone.js`

Reads from all registries. Generates sample data. Renders each section.

```js
function renderDevZone(ctx, canvasW, canvasH, time, scrollY) {
  let y = 20 - scrollY;

  // Section 1: Visuals
  y = renderSection(ctx, 'Visualization Gallery', y, canvasW);
  for (const visual of getAllVisuals()) {
    y = renderVisualCard(ctx, visual, y, canvasW, time);
  }

  // Section 2: Renderers
  y = renderSection(ctx, 'Renderer Gallery', y, canvasW);
  for (const renderer of getAllRenderers()) {
    y = renderRendererCard(ctx, renderer, y, canvasW, time);
  }

  // Section 3: Sprites
  y = renderSection(ctx, 'Sprite Gallery', y, canvasW);
  for (const sprite of getAllSprites()) {
    y = renderSpriteCard(ctx, sprite, y, canvasW, time);
  }

  // ... etc
}
```

### Files to create

```
src/presentation/renderers/visual-registry.js   # registerVisual, getAllVisuals
src/presentation/renderers/renderer-registry.js  # registerRenderer, getAllRenderers
src/presentation/sprite-registry.js              # registerSprite, getAllSprites
src/presentation/dev-zone.js                     # the Dev Zone renderer
```

### Files to modify

```
sprites.js                                       # register each sprite
src/presentation/renderers/visuals/*.js           # register each visual
src/presentation/renderers/quiz-renderer.js       # register, use registry for visual lookup
dialogue.js                                       # justinbailey check on game start
game.js                                           # DEV_ZONE state, scroll handling, ESC to exit
```

### The Guarantee

The pattern makes forgetting impossible:

1. **Visual not registered** → QuizRenderer can't find it → show-me breaks for that visual → immediately noticed
2. **Renderer not registered** → can't be selected by the application layer → challenge rendering breaks → immediately noticed
3. **Sprite not registered** → Dev Zone won't show it → but game still works (sprites are called directly). This is the weak link — sprite registration is only for the Dev Zone. Mitigate: add a build-time check that counts sprite draw functions vs registry entries.

For sprites, alternatively: the Dev Zone can scan the existing `SPRITE_FNS` map in `characters.js` instead of a separate registry. That map already lists all sprites and is required for NPC rendering. No extra registration step.

## Acceptance Criteria

1. Save file named "justinbailey" (case-insensitive, spaces ignored) opens Dev Zone
2. Dev Zone shows every registered visual with sample problems
3. Dev Zone shows every registered renderer at each phase
4. Dev Zone shows all sprites with animation
5. Adding a new visual via `registerVisual` automatically appears in Dev Zone
6. Arrow keys scroll, ESC exits to title screen
7. Dev Zone works without an API key (no AI-dependent content)
