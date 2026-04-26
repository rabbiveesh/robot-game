# Ten-Frames + Random Encounters — Implementation Spec

## 1. Ten-Frame Visual

### What it is

A 2×5 grid. Each dot fills one cell. Empty cells show "how many more to make 10." The classic K-1 manipulative for building number sense.

```
8 =  ●  ●  ●  ●  ●
     ●  ●  ●  ○  ○      ← 2 empty = "2 more to make 10"
```

```
13 = ●  ●  ●  ●  ●      ← full frame (10)
     ●  ●  ●  ●  ●
     ●  ●  ●  ○  ○      ← second frame (3 of 10)
```

For addition (8 + 5):
```
     ●  ●  ●  ●  ●      ← 8 (blue)
     ●  ●  ●  ○  ○
         +
     ●  ●  ●  ●  ●      ← 5 (yellow)
     ○  ○  ○  ○  ○
```

The kid can SEE that 8 needs 2 more to fill the frame, and 5 has 2 to spare, so 8+5 = 10+3 = 13. This is the "make 10" strategy — the most powerful early addition strategy — taught visually without words.

### When it shows

- Bands 1-4 (numbers 1-20)
- When the kid presses show-me
- Available in the Dev Zone playground alongside the other concrete visuals

### Layout

Two 2×5 grids of cells, one per operand. Filled cells = a colored dot; empty cells within the operand's count = a faint translucent square (these are the "how many more to make 10" empties). Cells beyond the operand's count are blank cells with just the border.

| Constant     | Value                                           |
|--------------|-------------------------------------------------|
| Cell         | 24 px                                           |
| Gap          | 3 px between cells                              |
| Frame gap    | 20 px between operand frames and the operator  |
| Cols × Rows  | 5 × 2                                           |

For numbers > 10, stack additional frames vertically. Operator glyph (`+` / `−`) sits between the two operand frames at 24 px bold. Each operand has a numeric label above it.

Operand colors: blue (`#42A5F5`) for `a`, yellow (`#FFD54F`) for `b` in addition; red (`#EF5350`) for `b` in subtraction.

### Selection

The challenge generator's `render_hint.cra_stage = Concrete` plus `band ≤ 4` and `operation ∈ {Add, Sub}` selects the ten-frame visual. The presentation renderer dispatches off `render_hint.visual_method` (a per-method tag the learner profile chooses) with a default of `TenFrame` for that range.

## 2. Random Encounters

### What it is

As the kid walks around exploring, things happen. Not every interaction requires walking up to an NPC and pressing Space. The world is alive.

### Encounter Types

| Encounter | Trigger | What happens |
|-----------|---------|-------------|
| **Sparky finds something** | Random tile step (1 in 30 chance) | "BEEP BOOP! I found a shiny thing! It's a... math puzzle!" → challenge or Dum Dum |
| **NPC appears** | Enter an area | A wandering NPC approaches and says something. "Hey! Can you help me count my apples?" |
| **Treasure sparkle** | Walk near a certain tile | A sparkle appears on the ground. Walk to it → Dum Dum or puzzle |
| **Sparky malfunction** | Random (1 in 50) | "BZZT! My circuits are scrambled! Quick, what's 3+2?!" → quick challenge, bonus Dum Dum for fast answer |
| **Animal sighting** | Enter flower/pond area | "Look boss! A butterfly! It has... 4 spots on each wing! How many spots total?" → contextual math |
| **Weather event** | Time-based (every 5 min) | Rain starts, Sparky says something silly. No challenge, just ambiance. |
| **Found Dum Dum** | Random tile (1 in 60) | A Dum Dum is just sitting on the ground. Free reward for exploring. |

### Design Principles

1. **Most encounters are NOT challenges.** The world should feel alive, not like every step is a quiz. Ratio: ~60% flavor (dialogue, silly events, free Dum Dums) / ~40% optional challenges.

2. **Encounters are interruptible.** Kid can walk away during an encounter dialogue. The encounter just ends — no penalty. They're optional by nature.

3. **Challenge encounters use the full interaction model.** Same lifecycle, same CRA, same show-me/tell-me. The kid doesn't know or care that this challenge was triggered by walking instead of by talking to an NPC.

4. **Encounter frequency adapts.** The `challengeFreq` dial (on the learner profile) controls how often encounters include a challenge vs just being flavor. Low challengeFreq = more "Sparky saw a butterfly" and fewer "quick, solve this!" High challengeFreq = more puzzles, fewer distractions.

5. **Never interrupt movement.** The encounter waits until the kid stops moving. If they're holding an arrow key, the encounter queues and fires when they release.

### Domain

Encounter logic is lightweight — it's mostly a random check + content selection. But the domain should decide:
- Whether an encounter fires (based on steps taken, area, time)
- Whether it's a challenge encounter (based on challengeFreq dial)
- What kind of challenge (uses existing generateChallenge)

```rust
// robot-buddy-domain/src/encounters.rs

pub struct EncounterConfig {
    pub steps_since_last_encounter: u32,
    pub min_steps_between: u32,          // don't fire back-to-back
    pub challenge_freq: f64,             // from learner profile
    pub area: String,                    // current map area
}

pub enum EncounterType {
    FlavorDialogue { speaker: String, text: String, speech: String },
    FoundDumDum,
    Challenge,
    SparkySighting { animal: String, math_context: String },
}

pub fn should_trigger_encounter(config: &EncounterConfig, rng: &mut impl Rng) -> bool {
    if config.steps_since_last_encounter < config.min_steps_between { return false; }
    // Base chance: 1 in 30 steps, modified by area (more in unexplored areas)
    rng.gen::<f64>() < 1.0 / 30.0
}

pub fn pick_encounter(config: &EncounterConfig, rng: &mut impl Rng) -> EncounterType {
    let is_challenge = rng.gen::<f64>() < config.challenge_freq;
    if is_challenge {
        EncounterType::Challenge
    } else {
        // Pick from flavor encounters based on area
        pick_flavor_encounter(config, rng)
    }
}
```

### Presentation

The encounter system hooks into the player movement update in `robot-buddy-game/src/main.rs`. After a successful tile step, increment a step counter, ask the domain whether to trigger an encounter, and if so queue it to fire when the player stops moving:

```rust
steps_counter += 1;

let should_trigger = encounters::should_trigger_encounter(
    &EncounterRequest {
        steps_since_last: steps_counter,
        min_steps_between: 15,
        challenge_freq: profile.challenge_freq,
        area: area_for_tile(player.tile_x, player.tile_y),
    },
    &mut rng,
);

if should_trigger && !player.moving {
    trigger_encounter(...);
    steps_counter = 0;
}
```

When an encounter fires:
1. Sparky does a little jump animation (antenna bounces)
2. Dialogue starts: "BEEP BOOP! I found something!"
3. If challenge: enters the challenge lifecycle (same as NPC interaction)
4. If flavor: just dialogue, maybe a free Dum Dum
5. If the kid walks away during dialogue: encounter ends, no penalty

### Encounter Dialogue Library

Per-area flavor text. Can use AI generation if API key is set, falls back to these:

```js
const ENCOUNTER_DIALOGUE = {
  'Home': [
    { speaker: 'Sparky', text: "I found a dust bunny under the rug! It's so fluffy!", type: 'flavor' },
    { speaker: 'Sparky', text: "Mommy's cookies smell SO GOOD! Can robots eat cookies?", type: 'flavor' },
  ],
  'Main Path': [
    { speaker: 'Sparky', text: "BZZZT! A ladybug landed on my antenna!", type: 'flavor' },
    { speaker: 'Sparky', text: "Hey boss, I found a shiny Dum Dum on the ground!", type: 'dum_dum' },
  ],
  'Pond': [
    { speaker: 'Sparky', text: "Look! A frog! It jumped 3 times! Wait, now 2 more times! How many jumps total?", type: 'challenge_context' },
    { speaker: 'Sparky', text: "The fish are swimming in circles. I'm getting dizzy watching them!", type: 'flavor' },
  ],
  'Forest Edge': [
    { speaker: 'Sparky', text: "I hear birds! I count 4 in that tree and 3 in this one!", type: 'challenge_context' },
    { speaker: 'Sparky', text: "This mushroom looks like a tiny umbrella!", type: 'flavor' },
  ],
  'Treasure Woods': [
    { speaker: 'Sparky', text: "My treasure sensor is beeping! There might be a Dum Dum nearby!", type: 'dum_dum' },
  ],
};
```

The `challenge_context` type provides story framing for the next challenge — "4 birds + 3 birds" naturally leads into "What is 4 + 3?" using the story template pattern.

### Files

- `robot-buddy-domain/src/encounters.rs` (new) — `should_trigger_encounter`, encounter type selection, area config.
- `robot-buddy-game/src/main.rs` — step counter, post-step encounter check, fire on movement stop.
- `robot-buddy-game/src/ui/dialogue.rs` — render encounter dialogue (uses the existing dialogue box).

### Events

```js
{
  type: 'ENCOUNTER_TRIGGERED',
  encounterType: 'flavor' | 'challenge' | 'dum_dum' | 'sighting',
  area: 'Pond',
  stepsWalked: 23,
  timestamp: 1234567890,
}
```

Feeds into session export so we can see how much the kid explored between challenges.

## Implementation Order

1. **Ten-frame visual** — one file, register, wire into QuizRenderer. Test in Dev Zone.
2. **Random encounters (flavor only)** — Sparky says silly things while walking. No challenges. Just life.
3. **Random encounters (with challenges)** — some encounters become challenges. Uses challengeFreq dial.
4. **Encounter context → story framing** — "I saw 4 birds and 3 birds" leads into "4 + 3 = ?" with context.

Step 1 is independent. Steps 2-4 are sequential. Step 2 is the most impactful — the world feels alive even without challenges.
