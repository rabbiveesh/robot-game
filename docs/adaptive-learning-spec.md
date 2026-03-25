# Adaptive Learning System — Design Spec

## Problem

The game needs to serve a wide range of kids — from a 4-year-old learning to count, to a 7-year-old doing multiplication, to kids who struggle and need extra patience. Right now difficulty is either manually configured or slowly adapts via streaks. This is too blunt.

## Principles

- **No labels.** We never tell a kid they're "slow" or "advanced." The game just feels right for them.
- **Measure behavior, don't ask parents.** Parents will all say their kid is gifted. The game should figure it out.
- **Disguise assessment as play.** The intake is Sparky's "sensor calibration," not a test.
- **Math only.** Phonics is dropped from the challenge system — the TTS-based dialogue is the wrong medium for real phonics instruction. Phonics done right needs a dedicated UX (letter tracing, audio playback of individual phonemes, blending animations). That's a separate project.
- **Concrete vs abstract is real.** Unlike the debunked "visual/auditory/kinesthetic" model, kids genuinely differ in whether they reason better with concrete manipulatives (count the dots) or structured/abstract representations (place value: tens and ones columns). The game should detect which works better and adapt.

## Representation Styles

This is the key insight: not all visual aids are equal. Some kids reason concretely, others structurally.

### Concrete ("count the things")
- Groups of dots/stars/objects
- "Here are 3 blue stars and 2 yellow stars. Count them all!"
- Works well for: small numbers, addition, early subtraction
- Breaks down for: larger numbers (who wants to count 47 dots?)

### Structured ("place value / number line")
- Tens-and-ones blocks (e.g., 23 = two long bars + three small cubes)
- Number line jumps (start at 8, jump forward 5, where do you land?)
- Works well for: carrying/borrowing, larger numbers, multiplication as repeated groups
- Can feel abstract to kids who haven't built the mental model yet

### How we detect preference
During intake and ongoing play, when a kid gets a question wrong and enters teaching mode, we show BOTH representations side by side (concrete left, structured right). Then we re-ask the question. Whichever side they seem to engage with (measured by where they click, or which style leads to correct retry answers) becomes their preferred style. The preference dial adjusts over time.

```
LEARNER_PROFILE = {
  ...
  representationStyle: 0.5,  // 0 = concrete (dots/objects), 1 = structured (place value/number line)
}
```

## Architecture

### 1. Intake Quiz ("Sparky's Calibration")

**The intake is NOT skippable.** Every new save file goes through it. If a parent wants to set up a second child with a similar profile, there will be a "Copy Profile" option in the save slot UI that copies the learner profile from another slot — but the new kid still plays the intake, and the copied profile is just the starting point that gets overwritten by their actual performance.

When a new game starts, Sparky says:

> "BEEP BOOP! My sensors need calibrating! Can you help me test them? Answer a few questions so I know how to be the BEST robot buddy!"

**Math placement (4 questions):**
- Start at band 3 (add/sub within 15)
- Correct → next question goes up 2 bands
- Wrong → next question goes down 1 band
- Final band = last question they got right (minimum band 1)
- One of the 4 questions is intentionally re-asked at a lower level after a wrong answer — this lets us show the teaching screen and observe which representation style they respond to

**Behavioral signals (measured silently during intake):**

| Signal | What we measure | What it tells us |
|--------|----------------|------------------|
| Response time | Milliseconds from question shown to answer clicked | Fast (<3s) = confident. Slow (>8s) = thinking carefully or uncertain. |
| Skip behavior | Did they press Space before typewriter finished? | Fast reader / impatient = shorter text preferred |
| Retry willingness | After first wrong answer, did they try again or hesitate? | High = resilient. Low = needs gentler failure handling. |
| Teaching engagement | During teaching mode, which representation did they engage with? | Concrete vs structured preference. |

### 2. Learner Profile (Visible Dials)

These dials are derived from the intake and continuously adjusted during play. **They are visible to parents** — a "Parent Dashboard" screen (accessible via a small gear icon or a long-press/konami-code on the title screen) shows the current profile with plain-English explanations. Parents can see what the game has learned about their kid and optionally override dials.

This serves dual purpose: parents understand what's happening (builds trust), and it's useful for debugging during development.

```
LEARNER_PROFILE = {
  pace:                0.5,   // 0 = very patient, 1 = fast-paced
  scaffolding:         0.5,   // 0 = always show visual aids, 1 = minimal hints
  challengeFreq:       0.5,   // 0 = more chat/exploration, 1 = challenge-heavy
  streakToPromote:     3,     // how many correct in a row to level up (2-5)
  wrongsBeforeTeach:   2,     // how many wrong before showing teaching mode (1-3)
  hintVisibility:      0.5,   // 0 = always show visual aids, 1 = only after mistakes
  textSpeed:           0.035, // seconds per character in typewriter (lower = faster)
  representationStyle: 0.5,   // 0 = concrete (dots), 1 = structured (place value/number line)
}
```

**How intake sets the dials:**

| Intake result | pace | scaffolding | challengeFreq | streakToPromote | wrongsBeforeTeach |
|--------------|------|-------------|---------------|-----------------|-------------------|
| Fast + mostly correct | 0.8 | 0.7 | 0.7 | 2 | 2 |
| Average speed + mixed | 0.5 | 0.5 | 0.5 | 3 | 2 |
| Slow + mostly wrong | 0.3 | 0.2 | 0.3 | 5 | 1 |

### 3. Title Screen UX

Replace the complexity of manual band dropdowns with a simple picker (keep dropdowns available under "Settings" for parents who want fine control):

**"How should Sparky teach?"**
- **Patient** — More encouragement, always shows visual aids, gentle pace, promotes after 5 correct. Good for younger kids or kids who need more support.
- **Balanced** (default) — Adapts to how your kid plays. Starts in the middle and adjusts.
- **Challenge me!** — Harder problems, faster pace, less hand-holding, promotes after 2 correct. For kids who get bored easily.

This sets the initial dial positions. The intake quiz then fine-tunes the actual starting band.

**Save slot "Copy Profile" button:** When creating a new game, if another slot has a completed profile, offer "Start like [sibling name]'s profile?" This copies the learner profile dials (not the bands or progress) as a starting point.

### 4. Ongoing Adaptation

The dials adjust continuously based on a rolling window of the last 20 challenges:

**Accuracy tracking (rolling window):**
- Above 85% correct → nudge pace up, scaffolding up, increase challengeFreq
- 60-85% correct → hold steady (sweet spot)
- Below 60% correct → nudge pace down, scaffolding down, decrease challengeFreq

**Engagement tracking:**
- If kid is mashing space through dialogue → increase textSpeed, reduce dialogue length
- If kid keeps playing for 10+ minutes → they're engaged, maintain current balance
- If kid stops interacting for 30+ seconds → they might be stuck or bored, Sparky offers a hint or cracks a joke

**Frustration detection:**

This needs careful balance. Some productive struggle is good — the goal isn't to prevent all wrong answers. The signals for *unproductive* frustration are:

- 3+ wrong answers in a row on the SAME band → drop down a band AND reduce wrongsBeforeTeach to 1 temporarily
- Rapid clicking (mashing random answers without thinking, <1s response time) → pause, Sparky says "Hey, take your time! I believe in you!"
- Long idle after wrong answer (>15s, no input) → kid might be upset. Sparky offers encouragement, maybe switches to a chat interaction instead of another challenge
- Repeated wrong answers on a specific operation type (e.g., always gets subtraction wrong but addition right) → weight generation AWAY from the weak spot temporarily, sprinkle it back in gradually. The opposite of drilling the weak spot — reduce frustration first, then revisit when confidence is higher.

**Representation style tracking:**
- After every teaching mode, track whether the retry was correct
- If retries are more successful after concrete representations → nudge representationStyle toward 0
- If retries are more successful after structured representations → nudge toward 1
- This dial moves slowly (only after teaching events, not every question)

### 5. How Dials Affect Gameplay

| Dial | Low value effect | High value effect |
|------|-----------------|-------------------|
| pace | 1.5s pause between challenge and next interaction. Longer celebration animation. Slower typewriter. | Instant transitions. Short celebration. Fast typewriter. |
| scaffolding | Visual aids shown with every math question. Teaching mode is detailed and slow. | No visual aids unless wrong. Teaching mode is brief. |
| challengeFreq | Only 30% of NPC interactions trigger a challenge. More silly robot dialogue. | 70% of interactions are challenges. Less filler chat. |
| streakToPromote | Need 5 correct to level up. Builds confidence through repetition. | Need 2 correct to level up. Keeps pushing. |
| wrongsBeforeTeach | Teaching mode after 1 wrong. Immediate support. | Teaching mode after 3 wrong. Lets them struggle productively. |
| hintVisibility | Dot/block representations shown alongside every math problem. | Visual aids only shown during teaching mode after wrong answers. |
| textSpeed | 0.05s/char (slow, readable for early readers) | 0.02s/char (fast, for kids who read ahead) |
| representationStyle | Teaching uses dots/objects (concrete). Count 'em up! | Teaching uses place value blocks / number line (structured). Tens and ones! |

### 6. Problem Type Tracking

Beyond overall accuracy, track per-operation performance:

```js
operationStats: {
  'add_small':  { correct: 0, attempts: 0 },  // bands 1-2
  'add_large':  { correct: 0, attempts: 0 },  // bands 3-4, 6-7
  'sub_small':  { correct: 0, attempts: 0 },
  'sub_large':  { correct: 0, attempts: 0 },
  'multiply':   { correct: 0, attempts: 0 },  // bands 5, 8-9
  'divide':     { correct: 0, attempts: 0 },  // band 10
  'number_bond': { correct: 0, attempts: 0 }, // "what + 3 = 8?"
}
```

This feeds into challenge generation: instead of pure random within a band, weight toward operations the kid is succeeding at (for confidence) with periodic forays into weaker operations (for growth). Ratio: roughly 60% strength / 40% growth, adjusted by frustration state — if frustrated, go 80/20 until frustration subsides.

### 7. Data Model (Save Data Addition)

```js
// Added to save data
learnerProfile: {
  pace: 0.5,
  scaffolding: 0.5,
  challengeFreq: 0.5,
  streakToPromote: 3,
  wrongsBeforeTeach: 2,
  hintVisibility: 0.5,
  textSpeed: 0.035,
  representationStyle: 0.5,
},
intake: {
  completed: false,
  mathPlacement: null,
  avgResponseTime: null,
  skippedText: false,
},
rollingWindow: [],       // last 20 results: [{ correct, responseTime, band, operation }]
operationStats: { ... }, // per-operation tracking (see above)
```

### 8. Implementation Plan

**Phase 1: Drop phonics from challenges**
- Remove phonics challenge generation
- Remove phonics bands, skill tracking, and UI badges
- Keep TTS for dialogue (it's great for that)
- Clean up title screen (remove reading level picker)

**Phase 2: Representation styles**
- Add structured representation renderer (tens/ones blocks, number line)
- Teaching mode shows both concrete and structured side by side
- Track which leads to successful retries

**Phase 3: Intake quiz**
- Sparky calibration dialogue flow
- 4 adaptive math questions
- Silent behavioral measurement
- Sets starting band + initial dial positions + representation preference

**Phase 4: Learner profile dials**
- Add LEARNER_PROFILE to game state
- Wire dials into existing systems: typewriter speed, challenge frequency, teaching triggers, visual aid display, representation choice
- Save/load profile

**Phase 5: Ongoing adaptation**
- Rolling window tracking
- Accuracy-based dial adjustment after every challenge
- Engagement detection (space-mashing, idle time)
- Frustration detection and graduated response
- Operation-type weighting in challenge generation

**Phase 6: Parent dashboard**
- Accessible from title screen (gear icon or code)
- Shows all dials with plain-English labels
- Shows operation stats as a simple bar chart
- Optional overrides for each dial
- "Copy Profile" option between save slots

## Open Questions

- What specific structured representations work best for each operation? (Tens/ones blocks for add/sub with carrying, arrays for multiplication, number line for subtraction as distance?)
- Should the game ever explicitly teach a new representation? ("Hey, let me show you a new way to think about this!") Or should it just silently switch?
- How do we handle the transition from concrete to structured as numbers get larger? (At some point dots become impractical and the game needs to nudge toward structured even if the kid prefers concrete)
- Are there other representation styles beyond concrete/structured worth exploring? (e.g., story-based: "You have 12 cookies and give 5 to Sparky...")

## Research Findings

### The Procedural vs Conceptual Tension (Veesh's research, 2026-03-25)

Math education research identifies two approaches that are both necessary:

- **Procedural fluency** (memorized facts, algorithms) frees working memory for complex problems
- **Conceptual understanding** (why it works) enables transfer and novel problem-solving

The game needs both. The RPG quest system (see `rpg-quest-spec.md`) provides conceptual framing (story problems, real-world schemas). The adaptive system provides procedural drill when needed, but only as much as needed.

### Why Gifted Kids Fail at Traditional Math

Research-backed reasons this matters for our design:

1. **"Speed = Smart" is wrong.** Gifted kids often process deeply, which looks slow. Timed pressure blocks working memory. → Our game should NEVER time-pressure. The `pace` dial controls presentation speed, not answer deadlines.

2. **Repetition kills engagement.** A gifted kid gets it after problem #2. By #5 they're bored. By #15, careless errors from disengagement. → The `streakToPromote` dial (set to 2 for fast learners) prevents this. But we should also detect the pattern: right-right-wrong(fast/careless)-right and NOT treat the wrong as a real failure.

3. **Top-down thinkers.** Many gifted kids need the big picture first, then fill in the details. → The RPG quest system naturally provides this: "Here's why you need this math" comes before "solve this problem."

4. **2e profiles (twice exceptional).** College-level reasoning but average processing speed or working memory. → Our system must separate *understanding* from *speed*. A slow correct answer is just as good as a fast one. The `pace` dial should never penalize slow thinkers.

### Why Pure Discovery Fails for Some Kids

The flip side — some kids need explicit instruction:

1. **Working memory overload.** Open-ended problems without scaffolding max out cognitive load. → The `scaffolding` and `hintVisibility` dials control this. Low-scaffolding kids get step-by-step breakdowns.

2. **Need for worked examples.** Some kids learn best by studying a solved problem first, then trying their own. → Teaching mode should sometimes SHOW a worked example before asking the kid to solve a similar one.

### Concrete-Representational-Abstract (CRA) Progression

The `representationStyle` dial should actually be a **3-stage progression**, not a binary:

| Stage | What it looks like | When to use |
|-------|-------------------|-------------|
| **Concrete** | Dots, stars, physical objects. Count them up. | Young kids, new concepts, low bands |
| **Representational** | Number lines, tens/ones blocks, bar models, drawings | Transitional — understands the concept, building mental models |
| **Abstract** | Just the numbers and symbols. No visual aids needed. | Mastered — procedural fluency achieved for this operation |

A kid can be at different CRA stages for different operations (concrete for division, abstract for addition). This should be tracked per-operation, not globally.

Key observation: Veesh's daughter struggles with concrete (dots) but succeeds with representational (tens/ones columns) — she's past concrete for her operations. The game was showing her the wrong stage.

### Spaced Practice & Interleaving

Instead of drilling the same operation type repeatedly, the research says:
- **Interleave** different operation types within a session (add, then multiply, then subtract)
- **Space** practice over time (revisit old bands periodically)

This maps to the quest system's micro-quest generation: vary the math type per quest, and periodically generate "review" quests that revisit earlier material.

### Schema-Based Instruction

Teach kids to recognize problem PATTERNS, not keyword-hunt:
- "Altogether" doesn't always mean add
- "Left over" doesn't always mean subtract
- The underlying STORY determines the operation

The RPG quest system is inherently schema-based — every problem is embedded in a story context. Different NPCs provide different schemas (sharing = division, shopping = multiplication, building = multi-step).

## Relationship to Other Specs

- **RPG Quest Spec** (`rpg-quest-spec.md`): defines how math is embedded in gameplay. The adaptive system provides difficulty scaling and representation choices; the quest system provides story context and motivation.
- **Future: Phonics Spec**: dropped from current scope. TTS dialogue is the wrong medium for phonics instruction. Would need dedicated UX (letter tracing, phoneme audio, blending animations).
