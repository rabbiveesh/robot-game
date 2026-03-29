# NPC Interaction Menu + Dum Dum Economy Phase 1 + Kid NPCs

**Status: IMPLEMENTED.** Shipped in PR #7. Reward logic now in Rust domain.

## Goal

Replace the auto-trigger "Space to talk" with an interaction menu that shows options per NPC. Add the give mechanic. Add kid NPCs at Mommy's house. Extract reward logic into the domain.

## 1. Domain: Economy (`src/domain/economy/`)

### rewards.js

```js
export function determineReward(interactionType, correct) {
  // interactionType: 'challenge' | 'chest' | 'quest'
  if (!correct) return null;
  return Object.freeze({ type: 'dum_dum', amount: 1 });
}
```

Immediately usable by the existing challenge reward paths (replace hardcoded `awardDumDum` calls in dialogue.js) and by the future challenge lifecycle reducer.

### give.js

```js
export function canGive(dumDums) {
  return dumDums > 0;
}

export function processGive(dumDums, recipientId, totalGiftsGiven) {
  // Returns: { newDumDums, newTotalGifts, milestone }
  // milestone is null or a milestone object when the kid crosses a threshold
  if (dumDums <= 0) return null;

  const newTotal = (totalGiftsGiven[recipientId] || 0) + 1;
  const milestone = checkMilestone(recipientId, newTotal);

  return Object.freeze({
    newDumDums: dumDums - 1,
    newTotalGifts: { ...totalGiftsGiven, [recipientId]: newTotal },
    milestone,
  });
}

const MILESTONES = [
  { count: 1,  reaction: 'first' },
  { count: 5,  reaction: 'spin' },
  { count: 10, reaction: 'accessory' },
  { count: 20, reaction: 'color_change' },
  { count: 50, reaction: 'ultimate' },
];

function checkMilestone(recipientId, total) {
  // Find the highest milestone reached
  for (let i = MILESTONES.length - 1; i >= 0; i--) {
    if (total === MILESTONES[i].count) {
      return Object.freeze({ recipientId, total, reaction: MILESTONES[i].reaction });
    }
  }
  return null;
}
```

### interaction-options.js

```js
export function getInteractionOptions(npc, playerState) {
  const options = [];

  // Everyone can be talked to
  options.push(Object.freeze({ type: 'talk', label: 'Talk', key: '1' }));

  // Can give Dum Dums to any character NPC (not chests)
  if (npc.canReceiveGifts !== false && playerState.dumDums > 0) {
    options.push(Object.freeze({ type: 'give', label: 'Give Dum Dum', key: '2' }));
  }

  // NPC-specific options
  if (npc.hasShop) {
    options.push(Object.freeze({ type: 'shop', label: 'Buy', key: '3' }));
  }

  return Object.freeze(options);
}
```

### Domain events

```js
DUM_DUM_EARNED {
  type: 'DUM_DUM_EARNED',
  amount: 1,
  source: 'challenge' | 'chest' | 'quest' | 'found',
  timestamp: number,
}

DUM_DUM_SPENT {
  type: 'DUM_DUM_SPENT',
  amount: 1,
  recipient: string,  // NPC id
  totalGiftsToRecipient: number,
  milestone: object | null,
  balanceBefore: number,
  balanceAfter: number,
  timestamp: number,
}
```

These go into the event log for the parent dashboard and session export.

### Tests

```
test/domain/economy/
  rewards.test.js
  give.test.js
  interaction-options.test.js

rewards:
  - 'correct challenge returns dum_dum reward'
  - 'wrong challenge returns null'
  - 'reward object is frozen'

give:
  - 'canGive returns true when dumDums > 0'
  - 'canGive returns false when dumDums === 0'
  - 'processGive decrements dumDums by 1'
  - 'processGive increments recipient total'
  - 'processGive returns milestone at count 1, 5, 10, 20, 50'
  - 'processGive returns null milestone between thresholds'
  - 'processGive returns null when dumDums === 0'
  - 'processGive tracks per-recipient totals independently'

interaction-options:
  - 'always includes talk option'
  - 'includes give when dumDums > 0'
  - 'excludes give when dumDums === 0'
  - 'excludes give when npc.canReceiveGifts is false'
  - 'includes shop when npc.hasShop is true'
  - 'options are frozen'
  - 'options have key assignments (1, 2, 3)'
```

## 2. Interaction Menu (Presentation)

### How it works

When player presses Space facing an NPC:

1. Call `getInteractionOptions(npc, playerState)` → get available options
2. If only one option (e.g., chest = just "Open"), auto-trigger it (no menu)
3. If multiple options, show a menu panel above the NPC:

```
┌─────────────────────────┐
│  [1] Talk  [2] Give     │
└─────────────────────────┘
```

Kid presses 1, 2, 3 or clicks to select. Menu dismisses and the selected action runs.

### Game state

New state: `GAME.state = 'INTERACTION_MENU'`

In this state:
- Player can't move
- Number keys 1-3 select an option
- Click on option buttons selects
- Space dismisses (cancel)
- ESC dismisses (cancel)

### Rendering

Small panel rendered above the NPC's head (or at the bottom of the screen like the dialogue box). Each option is a button with its key number. Style matches the challenge choice buttons but smaller.

```js
function renderInteractionMenu(ctx, options, npcScreenX, npcScreenY) {
  const panelW = options.length * 120 + 20;
  const panelH = 50;
  const panelX = npcScreenX - panelW / 2;
  const panelY = npcScreenY - 70; // above NPC head

  // Background
  ctx.fillStyle = 'rgba(20, 20, 40, 0.9)';
  roundRect(ctx, panelX, panelY, panelW, panelH, 10);
  ctx.fill();

  // Buttons
  options.forEach((opt, i) => {
    const btnX = panelX + 10 + i * 120;
    const btnY = panelY + 8;
    ctx.fillStyle = '#37474F';
    roundRect(ctx, btnX, btnY, 110, 34, 6);
    ctx.fill();
    ctx.fillStyle = '#E0E0E0';
    ctx.font = '14px "Segoe UI", system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(`[${opt.key}] ${opt.label}`, btnX + 55, btnY + 22);
    opt._bounds = { x: btnX, y: btnY, w: 110, h: 34 };
  });
}
```

### Files to change

- `game.js` — add INTERACTION_MENU state, handle number keys and click in that state, call `getInteractionOptions` on Space press
- `dialogue.js` or new `interaction-menu.js` — render the menu panel, handle option selection, dispatch to the right handler (talk → existing triggerNPCChat/triggerRobotChat, give → new give handler)

## 3. Give Mechanic (Presentation)

### What happens when kid selects "Give Dum Dum"

1. Call `processGive(DUM_DUMS, npcId, totalGiftsGiven)` → get result
2. Update `DUM_DUMS = result.newDumDums`
3. Update `totalGiftsGiven = result.newTotalGifts`
4. Log `DUM_DUM_SPENT` event
5. Play NPC reaction dialogue based on milestone and NPC personality

### NPC-specific reactions

Each NPC has a reaction table. The give handler looks up the reaction:

```js
const GIVE_REACTIONS = {
  robot: {
    normal: [
      "MMMMM! *crunch* BEST BOSS EVER! My circuits are tingling!",
      "Dum Dum Dum Dum! I love Dum Dums! Thank you, boss!",
      "BZZZT! Sugar rush! My antenna is spinning!",
    ],
    first: "My FIRST Dum Dum?! This is the BEST DAY of my robot LIFE!",
    spin: "FIFTY DUM DUMS! Watch me spin! *spins around* WHEEEEE!",
    accessory: "TEN?! I'm wearing a bow tie now! Look how FANCY I am!",
    color_change: "TWENTY! My chest light is changing color! LOOK LOOK LOOK!",
    ultimate: "FIFTY DUM DUMS. Boss. I... I don't have words. *happy robot tears*",
  },
  mommy: {
    normal: [
      "Oh sweetie, thank you! You're so thoughtful!",
      "A Dum Dum for me? You're the best!",
      "Mmm, cherry! My favorite! Thank you, honey!",
    ],
    first: "My very first Dum Dum! I'll treasure it forever!",
  },
  kid_1: {
    normal: [
      "WOW, thanks! You're the coolest!",
      "Yay! Dum Dum! You're my best friend!",
      "Mmmmm yummy! Wanna play?",
    ],
    first: "For ME?! Wow! No one ever gave me a Dum Dum before!",
  },
  kid_2: {
    normal: [
      "Hehe, thanks! *nom nom*",
      "Dum Dum! You're so nice!",
      "Ooh, what flavor? I love grape!",
    ],
    first: "A Dum Dum?! You're the nicest person EVER!",
  },
  // Gizmo, Old Oak, B0RK etc. get their own reaction tables
};

function getGiveReaction(npcId, milestone, rng) {
  const reactions = GIVE_REACTIONS[npcId] || GIVE_REACTIONS.robot;
  if (milestone) {
    return reactions[milestone.reaction] || reactions.normal[0];
  }
  const normals = reactions.normal;
  return normals[Math.floor(rng() * normals.length)];
}
```

### Sparky cosmetic milestones

At milestone thresholds, Sparky gets a visible change:

| Gifts | Cosmetic |
|-------|----------|
| 5 | Antenna spins briefly after each Dum Dum |
| 10 | Gains a bow tie (drawn in sprite) |
| 20 | Chest light changes color |
| 50 | Permanent sparkle trail |

These are flags on the save data: `sparkyCosmetics: { bowTie: false, colorChange: false, sparkleTrail: false }`. The sprite renderer checks these flags.

Kid NPCs don't get cosmetics (keeping it simple). They just have unique dialogue.

### Save data additions

```js
// Added to save data
totalGiftsGiven: { robot: 12, mommy: 3, kid_1: 5, kid_2: 2 },
sparkyCosmetics: { bowTie: false, colorChange: false, sparkleTrail: false },
```

## 4. Kid NPCs

### Definitions

Two kid NPCs placed in Mommy's house (the home interior map, near Mommy):

```js
// In NPC_DEFS_BY_MAP.home (or wherever Mommy is defined)
{
  id: 'kid_1',
  name: 'Tali',
  tileX: 6,
  tileY: 5,
  canReceiveGifts: true,
  dialogueContext: 'A playful kid who loves games and gets excited about everything.',
  draw: drawKidNPC1,
},
{
  id: 'kid_2',
  name: 'Noa',
  tileX: 8,
  tileY: 5,
  canReceiveGifts: true,
  dialogueContext: 'A shy but curious kid who asks lots of questions.',
  draw: drawKidNPC2,
},
```

Names are configurable — Veesh can change them to whatever he wants. They're just defaults.

### Sprites

Two small character sprites (shorter than adults, bigger heads proportionally):

```js
function drawKidNPC1(ctx, x, y, dir, frame, time) {
  // Similar to player sprite but:
  // - Shorter (12px body instead of 14px)
  // - Bigger head relative to body
  // - Different hair color (e.g., orange pigtails)
  // - Different shirt color (yellow)
  // - No walking animation if standing still (just idle bob)
}

function drawKidNPC2(ctx, x, y, dir, frame, time) {
  // Similar but:
  // - Different hair (brown, short)
  // - Different shirt (green)
  // - Slightly different proportions
}
```

Keep them simple — programmatic like all other sprites, no image assets.

### Dialogue

Kid NPCs have chat lines (no challenges — they're kids, not teachers):

```js
const KID_FALLBACK_LINES = {
  kid_1: [
    "Wanna see me do a cartwheel? Watch! ...okay I can't actually do one yet.",
    "Sparky is SO COOL! I wish I had a robot friend!",
    "Did you know frogs can jump SUPER far? Like, really far!",
    "I drew a picture of you and Sparky! It's on the fridge!",
    "Mom said we're having pizza later! PIZZA!",
  ],
  kid_2: [
    "Hi... um... do you like bugs? I found a really cool one.",
    "Sparky beeped at me and I think that means he likes me!",
    "I'm building a tower with blocks. Wanna help?",
    "Do you think clouds are soft? I think they're soft.",
    "Mom says I ask too many questions. Is that a lot of questions?",
  ],
};
```

If AI provider is configured, kid NPCs use it with their personality context. If not, fallback lines.

### Interaction flow for kid NPCs

Kid selects "Talk" → random chat line (no challenge, no coin flip)
Kid selects "Give Dum Dum" → give mechanic with kid-specific reaction

Kid NPCs NEVER trigger challenges. They're for relatedness (SDT) — making the game world feel alive with people the player cares about.

## Files

```
NEW:
  src/domain/economy/rewards.js
  src/domain/economy/give.js
  src/domain/economy/interaction-options.js
  src/domain/economy/index.js
  test/domain/economy/rewards.test.js
  test/domain/economy/give.test.js
  test/domain/economy/interaction-options.test.js

MODIFIED:
  sprites.js      — add drawKidNPC1, drawKidNPC2
  world.js         — add kid NPC definitions to home map
  characters.js    — kid NPCs in NPC interaction detection
  game.js          — INTERACTION_MENU state, ESC/number key handling
  dialogue.js      — give handler, reaction dialogues, replace awardDumDum with determineReward
  adapter.js       — log DUM_DUM_SPENT/EARNED events, save/load totalGiftsGiven + cosmetics
  rollup.config.js — add economy bundle (or fold into learning-domain bundle)
  index.html       — load economy bundle
```

## Acceptance Criteria

1. Space near any NPC shows interaction menu with available options
2. Menu shows "Talk" always, "Give Dum Dum" when DUM_DUMS > 0
3. Number keys (1, 2, 3) or click selects option
4. Giving a Dum Dum decrements counter and plays NPC reaction
5. Sparky gets cosmetic changes at milestone thresholds (visible in sprite)
6. Kid NPCs appear in Mommy's house, have unique dialogue, accept Dum Dums
7. Kid NPCs never trigger challenges
8. DUM_DUM_SPENT and DUM_DUM_EARNED events appear in session export
9. totalGiftsGiven and sparkyCosmetics persist across save/load
10. Existing challenge rewards use determineReward from domain (correct = reward, wrong = nothing)
11. All domain tests pass (rewards, give, interaction-options)
