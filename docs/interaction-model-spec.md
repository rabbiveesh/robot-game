# Core Interaction Model — Design Spec

## Overview

This is the atomic unit of every math interaction in the game. Intake, regular challenges, quest puzzles — they all use this model. Build it once, generalize everywhere.

The model has three independent axes that combine:

```
WHAT they see:    CRA axis       (concrete ↔ representational ↔ abstract)
HOW they answer:  Answer axis    (multiple choice ↔ eliminate ↔ free input ↔ build/drag)
HOW MUCH help:    Scaffold axis  (show-me ↔ tell-me ↔ nothing)
```

Each axis is a dial on the learner profile, tracked per-operation, adapted independently.

## Axis 1: CRA (What They See)

How the problem is visually presented. Drops down on "Show me!" hint.

| Level | Visual | Example for 8 + 5 |
|-------|--------|--------------------|
| **Abstract** | Text only | "What is 8 + 5?" |
| **Representational** | Number line, tens/ones blocks, bar model | Number line showing 8, arrow jumping +5 |
| **Concrete** | Countable objects, draggable items | 8 blue stars and 5 yellow stars |

**"Show me!" button** drops one level:
- Abstract → shows representational alongside the question
- Representational → shows concrete alongside
- Concrete → Sparky counts them out (animated, slow)

Already at the kid's CRA stage? Show-me isn't available at that level — they're already seeing their preferred representation.

**Start level per question:** determined by `profile.craStages[operation]`.

## Axis 2: Answer Mode (How They Answer)

How the kid submits their answer. Independent from CRA.

| Mode | UX | Signal density |
|------|-----|---------------|
| **3-choice (wide)** | Three options, one obviously wrong (e.g. 13, 47, 2) | Low — elimination is trivial |
| **3-choice (tight)** | Three close options (12, 13, 14) | Medium — need to compute |
| **Eliminate + pick** | Three choices with X buttons to cross out, then confirm | Rich — reveals partial knowledge. "I know it's not 12" = understands bounds |
| **Free input** | Number pad or keyboard, type the answer | High — no guessing. They know it or they don't |
| **Build/drag** | Drag objects to construct the answer (drag 13 items into a box, or assemble tens+ones blocks) | Highest — combines answer with CRA concrete/representational |

**Profile dial:**
```js
answerMode: 0.5,  // 0 = multiple choice, 1 = free input
// Tracked per-operation eventually, global for MVP
```

**Adaptation:**
- High accuracy + fast + no hints → nudge toward free input
- Low accuracy or frequent hints → nudge toward multiple choice
- Kid successfully eliminates wrong answers → they're ready for tighter choices

**Elimination signal extraction:**

When in eliminate mode, we record WHICH options they cross out and in what order:

```js
EliminationEvent {
  type: 'ANSWER_ELIMINATED',
  eliminatedValue: 12,
  correctAnswer: 13,
  wasCorrectElimination: true,  // they eliminated a wrong answer
  eliminationOrder: 1,           // first thing they eliminated
  responseTimeMs: 2300,          // time from question shown to this elimination
}
```

A kid who eliminates correctly shows number sense ("12 is too small because..."). A kid who eliminates the correct answer shows a misconception we should address.

## Axis 3: Scaffold (How Much Help)

Available at every step. In-character as Sparky helping.

| Button | What happens | Signal |
|--------|-------------|--------|
| **Show me!** | Drops CRA one level. If already at concrete, Sparky counts/animates slowly. | Kid knows they need help. Which CRA level helps = CRA stage data. |
| **Tell me!** | Sparky shows the answer with full concrete walkthrough. No penalty. Moves to next question. | Kid's ceiling for this operation/band. Not failure — information. |
| *(neither pressed)* | Kid answers without help | Confident at this level. |

**Show me! is NOT a hint about the answer.** It's a different way of SEEING the problem. "I can't do 8+5 in my head" → show-me → number line appears → "oh, I jump from 8 forward 5... 13!" The kid still has to figure out the answer. Show-me changes the representation, not the difficulty.

**Tell me! IS giving up on this question.** But it's framed as Sparky helping, not as failure. "Let me figure it out! Hmm... 8... 9, 10, 11, 12, 13! It's 13! I'll remember that." The kid watches Sparky solve it. This is a worked example — research says these are valuable for learning.

## Combined UX Flow

For a single math interaction:

```
1. Present problem at kid's CRA stage + answer mode
   ┌─────────────────────────────────────────────┐
   │  [CRA visual if not abstract]                │
   │                                              │
   │  Sparky: I found 8 bolts and 5 more!        │
   │  How many do I have?                         │
   │                                              │
   │  [answer mechanism: choices / input / drag]   │
   │                                              │
   │  💡 Show me!              🤷 Tell me!         │
   └─────────────────────────────────────────────┘

2. Kid interacts:
   ├─ Answers correctly → celebration, next
   ├─ Answers wrong →
   │   ├─ First wrong: "Hmm, not quite!" choices/input reset
   │   ├─ Second wrong: Sparky offers "Want me to show you?"
   │   └─ Third wrong: auto tell-me (don't let kid spiral)
   ├─ Hits "Show me!" → CRA drops one level, same question
   │   └─ Kid now answers with visual aid (recorded as hint-assisted)
   └─ Hits "Tell me!" → Sparky walks through it, next question
```

## Domain Events (Extended)

The existing `PUZZLE_ATTEMPTED` event gets richer fields:

```js
PuzzleAttempted {
  type: 'PUZZLE_ATTEMPTED',
  correct: boolean,
  operation: string,
  band: number,
  responseTimeMs: number,
  attemptNumber: number,
  timestamp: number,

  // NEW: interaction model signals
  craLevelShown: 'abstract' | 'representational' | 'concrete',
  hintUsed: boolean,        // did they press show-me?
  hintLevel: number,        // how many times (0, 1, 2)
  toldMe: boolean,          // did they press tell-me?
  answerMode: 'choice_wide' | 'choice_tight' | 'eliminate' | 'free_input' | 'build',
  eliminations: [],          // which options were eliminated before answering
}
```

The reducer consumes these new fields to adjust CRA stages, answer mode dial, and scaffolding dial. The existing fields still work — old events without the new fields are handled with defaults (backward compatible).

## Profile Additions

```js
LearnerProfile {
  // ... existing dials ...

  // NEW: answer mode (0 = multiple choice, 1 = free input)
  answerMode: 0.3,  // start with choices for everyone

  // EXISTING but now actually used:
  // craStages: per-operation CRA stage (tracked via hint usage)
  // scaffolding: overall scaffolding need (tracked via hint/tell-me frequency)
  // hintVisibility: whether to proactively show visuals (tracked via success-without-hint rate)
}
```

## Reducer Additions

```js
case 'PUZZLE_ATTEMPTED': {
  // ... existing band/streak logic ...

  // CRA stage tracking: if kid used hint, note which CRA level helped
  if (event.hintUsed && event.correct) {
    // Kid needed a lower CRA level but still got it right
    // → their CRA stage for this operation is the level that helped
    // e.g., showed abstract, hit show-me to get representational, then correct
    // → CRA stage for this operation = 'representational'
    newCraStages = updateCraStage(state.craStages, event.operation, event.craLevelShown);
  }
  if (!event.hintUsed && event.correct) {
    // Kid succeeded without help at their current CRA level
    // → potential promotion (after enough no-hint successes)
  }

  // Answer mode tracking: nudge toward free input on confident correct
  if (event.correct && !event.hintUsed && event.responseTimeMs < 5000) {
    newAnswerMode = Math.min(1, state.answerMode + 0.05);
  }
  if (event.toldMe || (event.hintUsed && !event.correct)) {
    newAnswerMode = Math.max(0, state.answerMode - 0.1);
  }

  // Scaffolding dial: frequent hint usage → more scaffolding
  // Infrequent → less
}
```

## Signal Extraction Summary

One interaction now yields:

| What | How | Richness vs old system |
|------|-----|----------------------|
| Can they solve it? | Correct/wrong | Same |
| How fast? | Response time | Same |
| What representation helps? | Which CRA level they needed (via show-me) | NEW — direct measurement |
| Do they need choices? | Success rate at free input vs choice | NEW |
| Can they eliminate wrong answers? | Which ones, in what order | NEW — reveals partial knowledge |
| Do they know they're stuck? | How quickly they press show-me or tell-me | NEW — metacognition signal |
| Where's their ceiling? | Tell-me = ceiling for this band/operation | NEW — cleaner than "wrong 3 times" |

## Implementation Plan

### Step 1: Intake with CRA + hint + tell-me (this PR)

- Render CRA visuals at three levels for the existing challenge UI
- Add "Show me!" / "Tell me!" buttons to the challenge panel
- Track hint/tell-me usage in intake answers
- Feed enriched events to the reducer
- 4 intake questions, each starting at abstract, kid can drop CRA with show-me
- Tests: verify CRA stage detection from hint patterns, verify tell-me doesn't penalize

### Step 2: Generalize to all challenges

- Replace old challenge pop-up with new interaction model
- All NPC challenges use CRA + hint + tell-me
- Answer mode dial starts affecting choice generation (tight vs wide vs eliminate)
- Remove legacy challenge rendering code

### Step 3: Story templates

- NPC-specific text wrapping the same math
- Story text replaces "What is 8 + 5?" but CRA visuals stay the same
- Cheapest de-broccoli layer

### Step 4: Answer mode progression

- Implement eliminate mode (X buttons on choices)
- Implement free input mode (number pad)
- Answer mode dial actively adapts
- Elimination signal extraction

### Step 5: Interactive CRA (mini-games)

- Concrete: drag objects to build answers
- Representational: interactive number line (tap to jump), base-10 block manipulation
- Replace static CRA visuals with interactive ones
- This is where we truly kill the broccoli

### Step 6: Quest integration

- Quest steps specify interaction context (NPC, story framing, stakes)
- Same core interaction model underneath
- Math IS the gameplay because the interaction is rich enough to be interesting

## Self-Selected Answer Mode

During intake (and periodically during play), let the kid CHOOSE how they want to answer:

```
How do you want to figure it out?

  🔢 Pick the answer     ✏️ Type it     ⭐ Count them out
```

### Why this matters

1. **Autonomy (SDT).** The kid feels in control from the first interaction. This is their game, not a test.

2. **Direct signal.** Instead of inferring answer mode preference from behavior, the kid tells us. Then we compare self-assessment to actual performance:

| Picks | Gets it right | Interpretation |
|-------|--------------|----------------|
| "Pick" | Yes | Comfortable with choices. Maybe ready to try harder. |
| "Pick" | No | Needs choices AND needs an easier band. |
| "Type it" | Yes | Confident and correct. Fluent. |
| "Type it" | No | Overconfident at this band. Interesting metacognition gap. |
| "Count them out" | Yes (quickly) | Prefers concrete but may not need it. Nudge toward representational. |
| "Count them out" | Yes (slowly) | Actually needed the concrete support. Right CRA stage. |

3. **Metacognition tracking.** The gap between what a kid picks and what they succeed at is itself a signal. A kid who always picks "type it" and gets 80% right has good self-knowledge. A kid who always picks "type it" and gets 40% right needs gentler redirection — not forced into choices (that kills autonomy), but Sparky might say "Want to try counting them? It's fun!"

### When to offer the choice

- **Intake**: questions 1 and 3. Questions 2 and 4 use whatever they last picked. If they switched between 1 and 3, that's notable.
- **Regular play**: every 5-8 interactions, or when the adaptive system is about to change the answer mode dial, give the kid the choice instead of forcing the transition. "Hey boss, want to try typing the answer this time?"
- **Never force.** If the kid always picks "count them out," that's fine. The system notes it and may gently suggest alternatives, but the kid's choice is respected.

### Domain events

```js
AnswerModeChosen {
  type: 'ANSWER_MODE_CHOSEN',
  chosen: 'choice' | 'free_input' | 'concrete_build',
  questionBand: number,
  operation: string,
}
```

This feeds into the answer mode dial but weighted less than actual performance — what the kid picks is a preference signal, what they succeed at is an ability signal. Both matter.

## Open Questions

- Should the "Show me!" / "Tell me!" buttons be available on every single interaction, or should we hide them once the kid demonstrates they don't need them? (Risk: kid gets stuck on one hard problem and can't find the hint button because it was hidden.)
- For elimination mode, do we show all 3 choices and let the kid X out, or show 3 and require them to X one before they can submit? (Forced elimination = more signal, but more friction.)
- Should the self-selected mode choice use icons only (for pre-readers) or icons + text? Probably icons + TTS reading the options aloud.
- Can we use the self-selection data to detect learning style shifts over time? (e.g., kid starts always picking "count" and gradually shifts to "type" over weeks — that's CRA progression happening naturally)

## Presentation Migration

**See `docs/presentation-migration.md` for migration trigger and plan.**
