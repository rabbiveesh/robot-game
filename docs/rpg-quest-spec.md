# RPG Quest System — Design Spec

## Vision

Math isn't a pop quiz that interrupts gameplay. Math IS the gameplay. Every puzzle, obstacle, and decision in the game world requires mathematical reasoning — and the kid is motivated to solve it because they have a goal they care about.

The current system (walk → NPC → flash card → reward) is a classroom with a Zelda skin. The target system is: **quest gives you a goal → math is the obstacle → solving it advances the story.**

## The Broccoli Test

Every feature we build must pass this test: **Is the math the mechanic, or is it chocolate on broccoli?**

- **Broccoli**: "Solve 3×4 to shoot the laser." The math is an obstacle BETWEEN the kid and the fun. The kid wishes the math wasn't there.
- **Not broccoli**: "You need to tile a 3×4 floor. How many tiles?" The math IS the fun. Removing the math removes the game.

If a kid would have more fun with the math removed, we've failed. If the math is what makes the puzzle satisfying to solve, we've succeeded.

## Scientific Frameworks

These aren't just theory — they're concrete design constraints we can test against.

### MDA (Mechanics, Dynamics, Aesthetics)

The learning must live in the **Mechanics** — the core rules of the game. Not in a side panel. Not in a quiz that interrupts play.

| Layer | What it means for us |
|-------|---------------------|
| **Mechanics** | Math operations are the verbs of the game. You add to combine inventory. You divide to share loot. You multiply to calculate costs. These aren't mini-games — they're how the world works. |
| **Dynamics** | The emergent gameplay that arises: kids develop strategies (stockpile cheap items, plan efficient routes, budget for big purchases). The math creates the strategy space. |
| **Aesthetics** | The feelings: pride when you figure out a hard trade, surprise when a clever calculation opens a shortcut, satisfaction when you have exactly enough resources. |

**Design test**: For every puzzle, ask "What would this interaction be if the kid already knew the answer?" If it would be boring (just press a button), the math is broccoli. If it would still be interesting (strategic choice, resource tradeoff), the math is a mechanic.

### Flow Theory

The game must keep kids in the flow channel — challenged but not overwhelmed.

```
                Anxiety
               /
              /   ← FLOW CHANNEL →
             /                      \
  Challenge /                        \ Boredom
           /                          \
          ─────────────────────────────
                   Skill →
```

This is exactly what the adaptive learning system's dials do. But the quest system adds a crucial dimension: **the kid controls difficulty by choosing quests.** A kid who's bored can take on the harder quest. A kid who's anxious can do a micro-quest to build confidence. Autonomy in difficulty selection is more effective than pure algorithmic adjustment.

The adaptive system sets the RANGE. The kid's choices within that range provide flow.

### Self-Determination Theory (SDT)

Three needs that drive intrinsic motivation:

| Need | How our game satisfies it |
|------|--------------------------|
| **Autonomy** | Choose which quests to do. Choose your path through puzzles. Choose to confront or sneak. The kid is the boss (literally — Sparky works for them). |
| **Competence** | Problems are in the sweet spot (adaptive system). Success is celebrated. Failure has natural consequences, not punishment. The kid sees themselves leveling up. |
| **Relatedness** | Sparky is a companion who cares. Mommy is proud. NPCs remember what you did for them. The kid belongs to Robot Village. |

**Design test**: Does the kid feel like they're making real choices? Or just following instructions? Every quest should have at least one meaningful decision.

### Cognitive Load Theory

Introduce one concept at a time. Let the kid master it through play before layering complexity.

| Principle | Application |
|-----------|------------|
| **One variable at a time** | New quest introduces ONE new math concept. Don't mix new addition strategies with a new game mechanic simultaneously. |
| **Worked examples first** | When introducing a new puzzle type, Sparky can "try it first" and get it wrong in a funny way — showing the kid the structure of the problem before they attempt it. |
| **Progressive complexity** | First shop quest: one item, one price. Second: two items. Third: two items + "do you have enough?" Fourth: comparison shopping. Each builds on the last. |
| **Offload to the environment** | Show the numbers IN the game world (price tags on items, weight numbers on bridges, battery meter on Sparky) so the kid doesn't have to hold everything in working memory. |

### Stealth Assessment

**Every interaction is data. No interaction is a "test."**

The current system has a visible challenge screen that pops up — this is a test, and kids know it. The quest system replaces this with puzzles that ARE the gameplay:

| Old (test) | New (stealth) |
|-----------|--------------|
| Pop-up: "What is 12 ÷ 3?" with multiple choice buttons | NPC: "Split these 12 gems into 3 bags for me!" — kid drags gems into bags, or types/clicks the number per bag |
| Correct/wrong celebration screen | The bags fill up and the NPC says thanks (or "hmm, that doesn't look even...") |
| Visible streak counter | Hidden — the adaptive system tracks everything silently |
| Difficulty badge ("Band 5: Multiply") | Gone — the kid just knows "the quests in Crystal Caves are harder" |

**The kid should never feel tested.** They should feel like they're solving problems in a world that responds to their solutions.

**Failure is normalized.** In school, wrong = bad grade. In the game, wrong = Sparky says something funny, the bridge wobbles, the merchant scratches his head. It's feedback, not judgment. Try again.

### Implications for Atypical Thinkers

These frameworks together create a space that works for kids traditional school fails:

- **No speed pressure.** Flow theory says challenge matters, not time. No timers, ever.
- **No handwriting penalty.** Clicks, drags, and number selection. The interface never penalizes motor skills.
- **Top-down exploration.** The game world IS the big picture. Kids who need to see the whole before the parts can wander the map, see the quests, understand what math is FOR, and then choose to engage.
- **Asynchronous development is fine.** A kid who reasons at a high level but processes slowly will naturally be in higher bands but with a low `pace` dial. The system handles this without contradiction.
- **Failure as iteration.** Every wrong answer leads to a game-world response, not a red X. The kid learns that wrong answers are information, not identity.

## Design Pillars

1. **The math must feel like a puzzle, not a test.** "The door code is 7 + ? = 15" is a puzzle. "What is 7 + 8?" is a test. Same math, completely different experience.

2. **Stakes create motivation.** If you get it wrong, something happens in the game world — Sparky's battery drains, you take the long way around, the shop charges you more. Not punishment — just natural consequences that make getting it right feel meaningful.

3. **Story context gives math meaning.** "What is 24 ÷ 4?" is abstract. "Split 24 Dum Dums equally between 4 friends" is fairness — a schema kids deeply understand. The RPG provides endless natural contexts.

4. **Multiple solution paths.** A locked door might accept the exact answer OR let you try a different route that's harder. This prevents hard blocks while rewarding mathematical thinking.

5. **Numbers live in the world.** Price tags on shop items. Weight numbers painted on bridges. Battery meter on Sparky's chest. The math is visible in the environment, not just in UI pop-ups. This offloads working memory and makes the world feel alive with math.

## Architecture

### Quest Structure

Quests are the primary narrative unit. Each quest has:

```
Quest {
  id: string,
  title: string,                    // "Sparky's Battery Crisis"
  description: string,              // shown in quest log
  prereqs: string[],                // quest IDs that must be completed first
  steps: QuestStep[],               // ordered sequence
  reward: Reward,                   // Dum Dums, items, map unlocks, cosmetics
  mathDomain: string[],             // which operations this quest exercises
  minBand: number,                  // minimum math band to attempt
  maxBand: number,                  // problems scale within this range
  adaptiveScaling: boolean,         // if true, problem difficulty matches learner profile
}
```

### Quest Steps

Each step is one of several types:

```
QuestStep =
  | DialogueStep       // NPC talks, sets up context
  | TravelStep         // go to a location on the map
  | MathPuzzleStep     // the core — a math problem embedded in a story context
  | ChoiceStep         // branching decision (may involve math reasoning)
  | RewardStep         // receive items, celebration
```

### Math Puzzle Types (Story-Embedded)

These replace the current "flash card" system. Each puzzle type maps to a math operation but is framed as a game-world problem:

#### Resource Puzzles
- **Shop math**: "Potions cost 3 gold each. You need 12 potions. How much gold?" (multiplication)
- **Crafting**: "You have 47 wood. A bridge costs 23. How much left after?" (subtraction)
- **Sharing**: "Split 24 Dum Dums between 4 friends." (division)
- **Inventory**: "You have 8 red gems and 7 blue gems. How many total?" (addition)

#### Obstacle Puzzles
- **Door codes**: "The lock shows: ? + 6 = 13" (number bonds / algebra intro)
- **Bridge weight limits**: "You weigh 23, Sparky weighs 19. Limit is 50. Can you cross?" (addition + comparison)
- **Path planning**: "Sparky's battery drains 3 per room. He has 20. How many rooms?" (repeated subtraction / division)
- **Tile puzzles**: Step on tiles that sum to a target number (multi-step addition)

#### Economy Puzzles
- **Trading**: "The merchant offers 5 apples for 2 gems. You have 8 gems. How many apples?" (multiplication + division)
- **Budgeting**: "You have 50 gold. Sword costs 28, shield costs 15. Can you buy both? How much left?" (multi-step)
- **Comparison shopping**: "Shop A: 3 for 12 gold. Shop B: 5 for 15 gold. Which is cheaper per item?" (division + comparison, higher bands)

#### Spatial Puzzles
- **Grid navigation**: "Move exactly 7 steps to reach the treasure. Which path works?" (counting + spatial reasoning)
- **Area/perimeter**: "The garden is 4 tiles wide and 3 tiles tall. How many flowers fit?" (multiplication as area, higher bands)
- **Symmetry**: "Complete the pattern on the other side of the mirror" (spatial + pattern recognition)

### Quest Examples

#### Starter Quest: "Welcome to Robot Village"
**Band 1-2, teaches basic addition**

1. **Dialogue**: Sparky says "Boss! I just moved here and I need to set up my house! Can you help me carry stuff?"
2. **Travel**: Go to the shop
3. **Puzzle (shop math)**: "I need 3 bolts and 2 gears. How many parts is that?" (3+2)
4. **Travel**: Bring parts to Sparky's house
5. **Puzzle (inventory)**: "We have 4 bolts already in the toolbox. Now we're adding 3 more. How many total?" (4+3)
6. **Dialogue**: Sparky builds something silly, celebration
7. **Reward**: Dum Dum + house gets a funny decoration

#### Mid Quest: "The Great Dum Dum Heist"
**Band 4-6, multi-operation**

1. **Dialogue**: Someone stole all the Dum Dums from the shop! 48 total! We need to figure out who and get them back.
2. **Puzzle (division/clue)**: "The thief left footprints in groups of 4. There are 12 footprints. How many trips did they make?" (12÷4)
3. **Travel**: Follow the trail
4. **Puzzle (subtraction/tracking)**: "The thief started with 48 but dropped some. You found 13 on the ground. How many does the thief still have?" (48-13)
5. **Choice**: Confront the thief directly (harder puzzle) or set a trap (easier puzzle, but need to go get supplies)
6. **Puzzle (varies by choice)**: Trap = "You need rope that's 15 feet long but only have 8-foot and 6-foot pieces..." OR Confront = "The thief wants to trade: they'll return the Dum Dums if you solve their riddle..."
7. **Reward**: Dum Dums returned, thief becomes a friend NPC

#### Advanced Quest: "Sparky's Upgrade"
**Band 8-10, multiplication/division focus**

1. **Dialogue**: Sparky can get a SUPER UPGRADE but needs exactly the right parts
2. **Puzzle (multiplication)**: "Each upgrade module needs 6 power cells. We need 8 modules." (6×8)
3. **Puzzle (division)**: "The power cells come in boxes of 12. How many boxes for 48 cells?" (48÷12)
4. **Puzzle (multi-step)**: "Each box costs 15 gold. We need 4 boxes. We have 50 gold. How much more do we need?" (15×4=60, 60-50=10)
5. **Travel + minigame**: Earn the extra gold by helping NPCs (smaller math puzzles)
6. **Reward**: Sparky gets a visible upgrade (new antenna? jet boots? laser eyes?)

### Failure & Retry Mechanics

Wrong answers should have consequences that feel natural, not punitive:

| Failure type | Consequence | Recovery |
|-------------|-------------|----------|
| Wrong shop calculation | "Hmm, that's not enough gold. Let me count again..." — retry | Immediate retry with optional hint |
| Wrong door code | Door buzzes, Sparky says "BZZT! That tickled!" | Try again, teaching mode if repeated failure |
| Wrong path planning | Sparky runs out of battery halfway, funny animation | Respawn at start of room, battery refilled, problem re-asked with hint |
| Wrong budget math | Merchant says "That's not right, friend! Count again!" | Retry, merchant progressively gives hints |
| Multi-step error | Partial credit — get credit for steps done right | Only re-do the step you got wrong |

**Key principle: never hard-block.** If a kid is truly stuck after 3 attempts + teaching mode, offer an alternative path that bypasses the puzzle (but they miss a bonus reward). The story continues either way.

### World Integration

#### Map Zones as Math Domains
The game world is organized so different areas emphasize different math:

| Zone | Theme | Primary math | Bands |
|------|-------|-------------|-------|
| Robot Village | Home base, shops | Addition, basic subtraction | 1-3 |
| The Workshop | Crafting, building | Multiplication, repeated addition | 4-6 |
| The Trading Post | Economy, merchants | Division, multi-step | 6-8 |
| The Crystal Caves | Puzzles, codes | Number bonds, missing values | 3-7 |
| The Sky Tower | Advanced challenges | All operations, multi-step | 8-10 |

#### NPCs as Quest Givers
Each NPC has a personality that maps to a math framing:

| NPC | Personality | Typical puzzle framing |
|-----|------------|----------------------|
| Mommy | Caring, domestic | Sharing, cooking (division, fractions later) |
| Professor Gizmo | Dramatic, experimental | Codes, formulas, patterns |
| Bolt (shopkeeper) | Merchant, practical | Money, trading, comparison |
| B0RK.exe (glitch dog) | Chaotic, silly | Random/surprise puzzles, bonus challenges |
| Old Oak (grove spirit) | Wise, slow | Story problems, multi-step reasoning |

### Dynamic Quest Generation

Pre-authored quests provide the narrative backbone, but the system also needs **procedurally generated micro-quests** to keep content fresh:

```
MicroQuest {
  template: "fetch_and_count" | "shop_trip" | "delivery" | "build_project" | ...,
  mathOps: derived from learner profile,
  numbers: scaled to current band,
  context: randomly selected from NPC + zone combos,
  reward: scaled to difficulty,
}
```

Templates are story skeletons with slots for numbers and operations:

```
Template: "delivery"
  "${npc} needs ${quantity} ${item}s delivered to ${location}."
  "Each trip you can carry ${capacity}."
  "How many trips do you need?"   // → division

  quantity and capacity filled by adaptive system based on band
```

This gives us infinite content while the hand-crafted quests provide narrative milestones.

## Interaction with Adaptive Learning System

The quest system and the adaptive learning system are deeply intertwined:

### How the Adaptive System Feeds the Quest System

| Adaptive dial | Effect on quests |
|--------------|-----------------|
| Band level | Determines which zones are unlocked and what numbers appear in puzzles |
| representationStyle (CRA stage) | When hints are shown, use appropriate representation (dots vs blocks vs abstract) |
| pace | Controls how long NPCs talk before getting to the puzzle |
| frustrationState | If high, generate easier micro-quests to rebuild confidence before story quests |
| operationStats | Weight generated puzzles toward strengths (60%) with growth sprinkles (40%) |
| streakToPromote | After N quests completed successfully in a zone, unlock next zone |

### How the Quest System Feeds Back to Adaptive

| Game event | Adaptive signal |
|-----------|----------------|
| Completed puzzle on first try | Record correct + response time |
| Used hint | Record correct but flag "needed help" |
| Failed and retried | Record initial failure + eventual success |
| Chose easier path | Possible frustration signal |
| Chose harder path | Challenge-seeking signal |
| Skipped optional side quest | Engagement signal (might be bored or focused) |
| Spent time exploring without doing quests | Engagement style — explorer, not optimizer |

## Quest Progression & Pacing

### Act Structure

The game has a loose 3-act structure that provides long-term motivation:

**Act 1: "New in Town" (Bands 1-4)**
- Arrive in Robot Village, meet NPCs, do errands
- Math is woven into daily life (shopping, building, sharing)
- Unlock the Workshop zone

**Act 2: "The Mystery" (Bands 5-7)**
- Something is wrong in Robot Village (Dum Dums disappearing? Machines breaking?)
- Investigation requires harder math (clues, codes, multi-step problems)
- Unlock Crystal Caves and Trading Post

**Act 3: "The Big Build" (Bands 8-10)**
- Build something huge to save the village (rocket ship? mega robot?)
- Requires gathering resources, trading, complex calculations
- Sky Tower unlocked for the final challenge
- Celebration ending with all NPCs

This is aspirational — we don't need all 3 acts before shipping. Act 1 alone is a viable game.

## Technical Architecture

### Quest Data Format

Quests are defined as JSON-like data structures in a `quests.js` file:

```js
const QUESTS = {
  welcome: {
    id: 'welcome',
    title: "Welcome to Robot Village",
    steps: [
      { type: 'dialogue', speaker: 'Sparky', text: '...' },
      { type: 'travel', targetX: 22, targetY: 5, zone: 'overworld' },
      { type: 'puzzle', template: 'shop_math', op: 'add', schema: 'buying' },
      { type: 'travel', targetX: 5, targetY: 7, zone: 'home' },
      { type: 'puzzle', template: 'inventory', op: 'add', schema: 'collecting' },
      { type: 'reward', dumDums: 2, item: 'house_decoration_1' },
    ],
    prereqs: [],
    minBand: 1, maxBand: 3,
  },
  // ...
};
```

### Quest State Machine

```
AVAILABLE → ACTIVE → step1 → step2 → ... → COMPLETED
                ↓                    ↓
              FAILED            (retry step)
              (never permanent)
```

### Quest Journal UI

In-game, the player can open a quest journal (Tab key or menu button) showing:
- Active quest + current step description
- Available quests (with NPC location hints)
- Completed quests (with a star for each)

This gives the kid a sense of progress and a clear "what do I do next?"

## Open Questions

- **How many hand-crafted quests do we need for a viable Act 1?** Thinking ~8-10 story quests + infinite micro-quests.
- **Should quests be strictly linear or can you have multiple active?** For young kids, one active quest is probably clearer. Older kids might enjoy juggling.
- **How do we handle the case where a kid levels up mid-quest?** The quest was designed for band 3 but they're now band 5. Do we scale up the remaining puzzles or let them coast through? (Probably coast — they earned it.)
- **How do micro-quests trigger?** Random NPC approach? Quest board in town? Sparky suggests one when there's nothing else to do?
- **Inventory system**: quests imply items (potions, gems, wood, gold). How complex should inventory be? Simple counter per item type? Or actual inventory management (which itself is math)?
- **Save granularity**: do we save per-quest-step or just per-quest? Mid-quest saves would be more forgiving but more complex.

## Relationship to Other Specs

- **Adaptive Learning Spec**: provides the difficulty scaling, representation style, and frustration detection that the quest system consumes. Quest outcomes feed back into the adaptive system.
- **Future: Phonics Spec** (if built): would need its own quest types — reading signs, decoding messages, spelling-based locks. Separate design.
