# Adaptive Learning System — Design Spec

## Problem

The game needs to serve a wide range of kids — from a 4-year-old learning to count, to a 7-year-old doing multiplication, to kids who struggle and need extra patience. Right now difficulty is either manually configured or slowly adapts via streaks. This is too blunt.

## Principles

- **No labels.** We never tell a kid they're "slow" or "advanced." The game just feels right for them.
- **Measure behavior, don't ask parents.** Parents will all say their kid is gifted. The game should figure it out.
- **Disguise assessment as play.** The intake is Sparky's "sensor calibration," not a test.
- **"Learning styles" (visual/auditory/kinesthetic) are largely debunked.** We focus on what actually varies: processing speed, scaffolding needs, frustration tolerance, and challenge-seeking behavior.

## Architecture

### 1. Intake Quiz ("Sparky's Calibration")

When a new game starts, Sparky says:

> "BEEP BOOP! My sensors need calibrating! Can you help me test them? Answer a few questions so I know how to be the BEST robot buddy!"

**Math placement (3 questions):**
- Start at band 3 (add/sub within 15)
- Correct → next question goes up 2 bands
- Wrong → next question goes down 1 band
- Final band = last question they got right (minimum band 1)

**Phonics placement (2 questions):**
- Start at band 2 (identify letter by sound)
- Same up/down logic
- Final band = last correct (minimum band 1)

**Behavioral signals (measured silently during intake):**

| Signal | What we measure | What it tells us |
|--------|----------------|------------------|
| Response time | Milliseconds from question shown to answer clicked | Fast (<3s) = confident. Slow (>8s) = thinking carefully or uncertain. |
| Skip behavior | Did they press Space before typewriter finished? | Fast reader / impatient = shorter text preferred |
| Retry willingness | After first wrong answer, did they try again or hesitate? | High = resilient. Low = needs gentler failure handling. |

### 2. Learner Profile (Hidden Dials)

These dials are derived from the intake and continuously adjusted during play. They are never shown to the kid. Stored in save data.

```
LEARNER_PROFILE = {
  pace:              0.5,   // 0 = very patient, 1 = fast-paced
  scaffolding:       0.5,   // 0 = always show visual aids, 1 = minimal hints
  challengeFreq:     0.5,   // 0 = more chat/exploration, 1 = challenge-heavy
  streakToPromote:   3,     // how many correct in a row to level up (2-5)
  wrongsBeforeTeach: 2,     // how many wrong before showing teaching mode (1-3)
  hintVisibility:    0.5,   // 0 = always show dots/visual aids, 1 = only after mistakes
  textSpeed:         0.035, // seconds per character in typewriter (lower = faster)
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

This sets the initial dial positions. The intake quiz then fine-tunes the actual starting bands.

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
- 3+ wrong answers in a row → drop down a band AND reduce wrongsBeforeTeach to 1 temporarily
- Kid gets the same type of problem wrong repeatedly → switch to the other type (math ↔ phonics) for variety

### 5. How Dials Affect Gameplay

| Dial | Low value effect | High value effect |
|------|-----------------|-------------------|
| pace | 1.5s pause between challenge and next interaction. Longer celebration animation. Slower typewriter. | Instant transitions. Short celebration. Fast typewriter. |
| scaffolding | Dots/visual aids shown with every math question. Letters always broken down for phonics. Teaching mode is detailed and slow. | No visual aids unless wrong. Teaching mode is brief. |
| challengeFreq | Only 30% of NPC interactions trigger a challenge. More silly robot dialogue. | 70% of interactions are challenges. Less filler chat. |
| streakToPromote | Need 5 correct to level up. Builds confidence through repetition. | Need 2 correct to level up. Keeps pushing. |
| wrongsBeforeTeach | Teaching mode after 1 wrong. Immediate support. | Teaching mode after 3 wrong. Lets them struggle productively. |
| hintVisibility | Dot representations shown alongside every math problem. | Dots only shown during teaching mode after wrong answers. |
| textSpeed | 0.05s/char (slow, readable for early readers) | 0.02s/char (fast, for kids who read ahead) |

### 6. Data Model (Save Data Addition)

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
},
intake: {
  completed: false,
  mathPlacement: null,    // band placed into
  phonicsPlacement: null,
  avgResponseTime: null,  // ms
  skippedText: false,
},
rollingWindow: []  // last 20 results: [{ type, correct, responseTime, band }]
```

### 7. Implementation Plan

**Phase 1: Intake quiz**
- Sparky calibration dialogue
- 5 adaptive questions
- Silent behavioral measurement
- Sets starting bands + initial dial positions

**Phase 2: Learner profile dials**
- Add LEARNER_PROFILE to game state
- Wire dials into existing systems: typewriter speed, challenge frequency, teaching triggers, visual aid display
- Save/load profile

**Phase 3: Ongoing adaptation**
- Rolling window tracking
- Accuracy-based dial adjustment after every challenge
- Engagement detection (space-mashing, idle time)
- Frustration detection and response

**Phase 4: Title screen simplification**
- 3-way teaching style picker (Patient / Balanced / Challenge me!)
- Move manual band dropdowns to Settings accordion
- Show learner profile summary on save slot display

## Open Questions

- Should the intake quiz be skippable? (Probably yes for returning players who switch save slots)
- Should parents have a way to see/override the learner profile? (Maybe in a hidden "parent mode" behind a long-press or code)
- Should we track which specific problem types are hardest (e.g., "subtraction with borrowing" vs "addition") and weight generation toward weak spots?
- How do we handle a kid who is advanced in math but still learning phonics? (The dials are already per-subject for bands, but pace/scaffolding are global — should they be per-subject too?)
