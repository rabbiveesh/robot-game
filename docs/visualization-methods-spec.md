# Visualization Methods — Design Spec

## Correction: CRA Stages ≠ Visualization Methods

The original design treated CRA as a ladder: concrete (dots) → representational (number line) → abstract (text). This conflated two independent axes:

1. **CRA stage**: how abstract is the presentation? (concrete objects → visual diagrams → symbols)
2. **Visualization method**: which concrete or representational model? (dots, ten-frames, base-10 blocks, number lines, bar models, arrays)

Base-10 blocks are concrete (they're objects), not representational. A number line is representational (it's a diagram), not abstract. Different kids prefer different methods WITHIN the same CRA stage. The system should offer multiple and track which works.

## Visualization Methods

### Concrete Methods (physical/visual objects the kid can count or manipulate)

#### Individual dots/stars (unitary)
- Each object = 1
- Kid counts them
- Good for: numbers 1-10, basic addition/subtraction
- Breaks at: ~15 (too many to count, becomes tedious)
- Current implementation: `renderDotVisual` in dialogue.js

#### Ten-frame (2×5 grid of dots)
- Standard early math tool
- Shows "how many to make 10" visually (empty spaces)
- Good for: numbers 1-20, making-10 strategy, number bonds
- Example: 8 = ten-frame with 2 empty spaces
- Helps with: carrying (fill the frame, overflow to next frame)

#### Base-10 blocks (tens rods + ones cubes)
- Long bar = 10, small square = 1
- Good for: numbers 10-100+, place value, carrying/borrowing
- For 47: draw 4 long bars + 7 small squares
- For 47 + 28: show 4 bars + 7 squares and 2 bars + 8 squares. Kid sees 15 ones → trade 10 ones for 1 bar → 7 bars + 5 squares = 75
- **This is what Veesh's daughter succeeds with**

#### Array/grid (rows × columns of dots)
- For multiplication: 3 × 4 = a 3-row, 4-column grid of 12 dots
- Shows why multiplication is commutative (rotate the grid)
- Shows area model (tiles on a floor)
- Good for: multiplication, division (partition the grid)

#### Grouped objects (Dum Dums!)
- Use the game's own currency as manipulatives
- "You have 8 Dum Dums and earn 5 more"
- Groups of 5 are natural (one hand)
- Connects math to something the kid already cares about

### Representational Methods (diagrams that model the math)

#### Number line
- A horizontal line with tick marks
- Addition = jump right, subtraction = jump left
- Good for: any operation, number sense, estimation
- Shows "distance between" for subtraction
- Can be empty (kid places the numbers) or pre-marked

#### Bar model / tape diagram
- Rectangles showing parts and whole
- Good for: word problems, comparison, number bonds, missing addend
- "The whole bar is 15. One part is 8. How long is the other part?"
- Very effective for schema-based instruction

#### Written place value columns
- Vertical columns: Hundreds | Tens | Ones
- Write digits in columns, show carrying/borrowing as annotations
- More abstract than blocks but more structured than just symbols
- Good for: carrying/borrowing at higher numbers, transition to algorithm

### Abstract
- Just the numbers and symbols: 47 + 28 = ?
- No visual aid
- For kids who are fluent at this operation

## Architecture: Method as a Profile Dial

Instead of one CRA stage per operation, track a **preferred visualization method** per operation:

```js
visualMethods: {
  add: 'base10_blocks',      // prefers tens rods + ones cubes for addition
  sub: 'number_line',        // prefers number line for subtraction
  multiply: 'array',         // prefers grid for multiplication
  divide: 'base10_blocks',   // prefers blocks for division
  number_bond: 'bar_model',  // prefers bar model for bonds
}
```

The CRA stage is DERIVED from the method:
```js
const METHOD_CRA = {
  dots: 'concrete',
  ten_frame: 'concrete',
  base10_blocks: 'concrete',
  array: 'concrete',
  grouped_objects: 'concrete',
  number_line: 'representational',
  bar_model: 'representational',
  place_value_columns: 'representational',
  abstract: 'abstract',
};
```

### How the System Discovers Preferences

Same approach as the feature discovery system — track which visualization method was shown alongside correct/wrong:

```js
// In PUZZLE_ATTEMPTED events:
{
  visualMethod: 'base10_blocks',  // what was shown (null if abstract / no hint)
  hintUsed: true,
  correct: true,
  // ...
}
```

After enough data, compare accuracy per method per operation:

```
Addition with base10_blocks: 85% accuracy (after hint)
Addition with dots: 40% accuracy (after hint)
Addition with number_line: 70% accuracy (after hint)
→ This kid prefers base10_blocks for addition
```

The system doesn't need to decide upfront — it tries methods and observes.

### Show-Me With Method Selection

When the kid presses "Show me!", instead of just dropping CRA one level, offer a choice of methods appropriate to the problem:

```
┌──────────────────────────────────────────┐
│  How do you want to see it?              │
│                                          │
│  ⚫⚫⚫  Count them     (dots)            │
│  █▪▪▪  Use bars        (base-10 blocks) │
│  ←──→  Number line      (number line)   │
│                                          │
└──────────────────────────────────────────┘
```

Or: the system picks the kid's preferred method automatically (based on past success) and the kid can tap "Show differently" to cycle through alternatives.

For MVP: just show the kid's preferred method. "Show differently" button for alternatives. Track which one led to the correct answer.

### Which Methods Available at Which Bands

| Band | Numbers | Available methods |
|------|---------|------------------|
| 1-2 | 1-10 | dots, ten_frame, grouped_objects |
| 3-4 | 1-20 | dots, ten_frame, base10_blocks, number_line, bar_model |
| 5 | multiply x1,x2 | array, grouped_objects |
| 6-7 | 1-100 | base10_blocks, number_line, place_value_columns |
| 8-9 | multiply 1-12 | array, base10_blocks |
| 10 | division | array, base10_blocks, number_line |

Dots are NOT available past band 4. Nobody counts 47 dots.

### Rendering Each Method

Each method is a renderer function:

```
src/presentation/renderers/visuals/
  dots-visual.js             # Individual dots in groups (band 1-4)
  ten-frame-visual.js        # 2×5 grid (band 1-4)
  base10-blocks-visual.js    # Tens rods + ones cubes (band 3+)
  array-visual.js            # Rows × columns grid (multiplication)
  number-line-visual.js      # Horizontal line with jumps (all bands)
  bar-model-visual.js        # Parts/whole rectangles (bonds, word problems)
  grouped-objects-visual.js  # Dum Dum icons grouped by 5 (all bands, in-game flavor)
```

Each visual renderer implements:
```js
{
  render(ctx, a, b, op, answer, cx, cy, time): void
  // cx, cy = center position for the visual
  // a, b = operands, op = operation symbol, answer = correct answer
}
```

The QuizRenderer calls the appropriate visual renderer based on `challengeState.renderHint.visualMethod` when `hintUsed` is true.

### Domain Changes

```js
// LearnerProfile additions:
visualMethods: {
  add: 'dots',              // starting default per operation
  sub: 'dots',
  multiply: 'array',
  divide: 'base10_blocks',
  number_bond: 'bar_model',
},

// Challenge state renderHint additions:
renderHint: {
  craStage: 'concrete',           // derived from visualMethod
  visualMethod: 'base10_blocks',  // specific method for this challenge
  answerMode: 'choice',
  interactionType: 'quiz',
}

// PUZZLE_ATTEMPTED event additions:
{
  visualMethod: 'base10_blocks',  // what was shown (null if no hint)
}
```

### CRA Stage Derivation

The existing CRA stage logic in the reducer still works — it just reads from the method:

```js
function craStageForMethod(method) {
  return METHOD_CRA[method] || 'concrete';
}
```

Promotion: when the kid succeeds 3 times without hints at a concrete method, try a representational method (not "promote to representational" — try number_line or bar_model specifically). If they succeed, update the preferred method. If they fail, stay at the concrete method that works.

Demotion: if a kid fails with number_line, try base10_blocks. If they fail with base10_blocks, try dots (if band allows). The system searches for what works, doesn't just go "down."

### Self-Selection (from interaction model spec)

The "How do you want to figure it out?" picker from the interaction model spec maps directly:

```
🔢  Pick the answer     (abstract — no visual)
⚫  Count them          (dots — concrete unitary)
█▪  Use bars            (base-10 blocks — concrete grouped)
←→  Number line         (representational)
⭐  Count Dum Dums      (grouped objects — in-game concrete)
```

The kid picks. The system observes. Over time, the preferred method emerges per operation.

## Implementation Priority

**Phase 1: Base-10 blocks renderer (this PR)**
- `base10-blocks-visual.js` — tens rods + ones cubes
- Replace dots when band > 4 and show-me is pressed
- Track `visualMethod` in events

**Phase 2: Additional concrete renderers**
- `ten-frame-visual.js` — for bands 1-4
- `array-visual.js` — for multiplication
- `grouped-objects-visual.js` — Dum Dum icons (fun factor)

**Phase 3: Representational renderers**
- `number-line-visual.js` — jumps for add/sub
- `bar-model-visual.js` — parts/whole for bonds

**Phase 4: Method selection + discovery**
- Show-me offers method choice (or cycles)
- Track per-method accuracy
- Adaptive method selection based on past success
- Self-selection UI

## Open Questions

- Should the visual method animate? (e.g., dots appearing one by one, tens rod "building" from 10 ones, number line arrow jumping) Animation takes time but reinforces the concept.
- Should the kid be able to interact with the visual? (tap dots to count, drag ones to make a ten) That's the drag/build answer mode from the interaction model — crosses CRA with answer mode.
- At what point does a kid "graduate" from needing visuals at all? After N abstract-mode correct in a row, the system could stop offering show-me. But should it? Maybe always keep it available — even adults use scratch paper.
