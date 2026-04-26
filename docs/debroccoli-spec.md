# De-Broccoli-fication — Design Spec (DRAFT)

**Status: THINKING. Not ready for implementation.**

## The Problem

The current challenge system is chocolate-covered broccoli. A pop-up quiz interrupts gameplay. Story templates (NPC dialogue wrapping the same quiz) are a cosmetic improvement — the kid is still answering "What is 3 + 2?" with multiple choice buttons. That's the Abstract tier of CRA regardless of how we frame it in words.

## CRA Recap (Concrete → Representational → Abstract)

| Stage | What the kid does | Example for 3 + 2 |
|-------|------------------|-------------------|
| **Concrete** | Manipulates physical/visual objects directly | Drag 3 stars and 2 stars into a box, count what's in the box |
| **Representational** | Works with structured visual models | See a number line, watch it jump from 3 forward by 2, land on 5 |
| **Abstract** | Works with symbols and language | Read "3 + 2 = ?" and type/select 5 |

Our current system is 100% Abstract (text question → select answer). Story templates are still Abstract (story-framed question → select answer). TTS doesn't change this — hearing a question read aloud is still Abstract.

## What "Not Broccoli" Actually Looks Like

The math interaction must be **manipulative** — the kid does something with their hands (click, drag, tap) that IS the math, not just selects a pre-computed answer.

### Concrete interactions (bands 1-4, young kids)

- **Drag to group**: "Help Sparky sort his bolts!" — drag 3 blue bolts and 2 red bolts into a bin, count updates live as you drag
- **Tap to count**: Objects appear on screen, kid taps each one and a counter increments. Builds one-to-one correspondence.
- **Split by dragging**: "Share 6 cookies between 2 plates!" — drag cookies one at a time onto plates. The kid discovers division through action.
- **Build a tower**: Stack blocks to match a target number. 5 + 3 = stack 5, then stack 3 more, tower shows 8. Physical metaphor for addition.

### Representational interactions (bands 3-7, transitional)

- **Number line jumps**: A character stands on a number line. Kid taps "jump forward 5" and watches the character hop from 8 to 13. Subtraction = jump backward.
- **Base-10 blocks**: Tens bars and ones cubes. To solve 23 + 14, kid sees 2 tens-bars + 3 ones and 1 ten-bar + 4 ones. Drag them together. Regroup when ones exceed 10 (carrying!).
- **Array builder**: For multiplication, build a grid. 3 × 4 = make a 3-by-4 grid of dots. Count the total. Connects multiplication to area.
- **Bar model / tape diagram**: For word problems. "Sparky has some bolts. He found 5 more. Now he has 12. How many did he start with?" Draw a bar showing the whole (12) and a part (5), kid identifies the missing part.

### Abstract interactions (bands 6-10, fluent kids)

- **Quick answer** (what we have now, but without the quiz chrome): number appears in dialogue, kid types or clicks answer. No visual aids. They're fluent.
- **Equation balancer**: Show a balance scale with expressions on each side. Kid adjusts one side to make them equal. Algebraic thinking.
- **Estimation challenges**: "About how much is 47 + 38? Closer to 80 or 90?" Builds number sense, not just computation.

## The Design Challenge

This is not a template system. Each CRA stage requires fundamentally different UI:

- **Concrete** needs drag-and-drop, object counting, spatial interaction
- **Representational** needs animated number lines, base-10 block manipulation, grid builders
- **Abstract** needs text input or quick-select (what we have)

Each of these is a **mini-game** with its own input handling, rendering, and state. The challenge generator doesn't just produce `{ question, choices }` — it produces `{ type: 'drag_to_group', objects: [...], target: 5 }` or `{ type: 'number_line', start: 8, jump: 5 }`.

## Open Questions

- How many mini-game types do we need for a viable product? Probably 3-4 concrete + 2-3 representational + the existing abstract.
- How do we transition a kid from concrete to representational? Show both side-by-side? Gradually fade out the concrete?
- How does this interact with the quest system? Each quest step specifies a mini-game type, or the CRA stage of the learner profile determines which mini-game is used?
- How much does this change the domain layer? The challenge generator needs to produce richer output, but the reducer/profile/frustration detection stay the same — they just receive events from the mini-games instead of from the pop-up quiz.
- What's the mobile/touch story? Drag-and-drop is natural on tablets but awkward with a mouse for small kids. This might actually work better on iPad.
- Do we build a mini-game framework (each game is a plugin with standard interfaces) or hand-build each one?

## Parking Lot

### Story templates (cosmetic improvement)

Even without CRA mini-games, replacing the quiz pop-up with NPC dialogue is still an improvement. It removes the "test" feeling. This can ship independently:

- Template registry: `(npc, operation) → story-framed question text[]`
- Render question in dialogue box instead of challenge pop-up
- Answer choices as dialogue options
- NPC reactions replace "AMAZING!" screen

This is still Abstract-tier only. But it's a quick win while we design the real thing.

### Relationship to other specs

- **Adaptive Learning Spec**: CRA stage tracking already exists in the learner profile. The mini-games are the CONSUMERS of that data — "this kid is at Representational for addition" tells us to use the number-line mini-game, not the drag-to-group game.
- **RPG Quest Spec**: quests would specify which mini-game types are available at each step, with the CRA stage determining the default.
- **Architecture Spec**: mini-games would be presentation-layer components. The domain produces challenge data; the mini-game renders it and reports events back.

