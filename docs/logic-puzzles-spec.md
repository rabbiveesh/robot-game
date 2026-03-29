# Logic Puzzles — Design Spec

## Why This Matters

The current game is 100% arithmetic — compute an answer, select it. Logic puzzles develop different mathematical skills (deduction, constraint satisfaction, pattern recognition) that REINFORCE arithmetic rather than replacing it. Research shows these reasoning skills are better predictors of later math success than early computation ability.

Logic puzzles also pass the broccoli test by construction. Nobody looks at a Sudoku and thinks "this is a math test." The puzzle IS the game.

## Puzzle Types (in priority order)

### 1. KenKen (bridges arithmetic + logic)

The first puzzle to build. KenKen uses arithmetic the kid already knows as a TOOL for solving a logic puzzle. Two independent difficulty axes: grid size (logic) and cage operations (arithmetic).

#### What it is

A grid where:
- Each row contains the numbers 1 to N exactly once (like Sudoku)
- Each column contains the numbers 1 to N exactly once
- Cells are grouped into "cages" with a target number and an operation
- The numbers in each cage must produce the target using the operation

#### Scaling

| Grid | Age | Logic difficulty | Example |
|------|-----|-----------------|---------|
| 2×2 | 4yo | Trivial (2 constraints per row/col) | Numbers 1-2, cages with + |
| 3×3 | 4-5yo | Easy (3 constraints) | Numbers 1-3, cages with + and - |
| 4×4 | 5-7yo | Medium | Numbers 1-4, cages with +, -, × |
| 6×6 | 7+yo | Hard | Numbers 1-6, all operations |

Cage operations scale with the kid's arithmetic band:
- Bands 1-2: addition-only cages
- Bands 3-4: addition + subtraction cages
- Bands 5-8: add multiplication cages
- Bands 9-10: add division cages

The kid's CRA stage applies to cage visuals — concrete shows dots in the cage, representational shows the operation symbol + target, abstract shows just the target number.

#### Example (3×3, addition cages)

```
┌─────┬─────┬─────┐
│ 5+  │     │     │
│     │  3+ │     │
├─────┼─────┼─────┤
│     │     │ 4+  │
│ 3+  │     │     │
├─────┼─────┼─────┤
│     │     │     │
│     │  5+ │     │
└─────┴─────┴─────┘

Solution:
  2  3  1
  1  2  3
  3  1  2

Cage "5+" (top-left 2 cells): 2+3 = 5 ✓
Cage "3+" (top-right 2 cells): 1+2 = 3 ✓ (wait, need to design cages carefully)
```

#### For a 4-year-old (2×2)

```
┌─────┬─────┐
│ 3+  │     │
│     │     │
├─────┼─────┤
│     │     │
│     │ 3+  │
└─────┴─────┘

"The top two cells add to 3. The bottom two cells add to 3.
 Each row has a 1 and a 2. Each column has a 1 and a 2."

Answer:
  2  1
  1  2
```

Sparky: "Hmm, these two boxes need to add up to 3! What two numbers make 3?"

The kid is doing addition AND deduction AND constraint satisfaction. Three skills from one puzzle.

### 2. Pattern Sequences

#### What it is

Continue a pattern. Scales from visual (4yo) to numeric (7yo).

| Level | Example | Skill |
|-------|---------|-------|
| Picture AB | 🔴🔵🔴🔵🔴??? | Pattern recognition |
| Picture ABB | 🔴🔵🔵🔴🔵🔵🔴??? | More complex pattern |
| Picture ABC | 🔴🔵🟢🔴🔵🟢??? | Three-element pattern |
| Number +1 | 1, 2, 3, 4, ??? | Counting pattern |
| Number +2 | 2, 4, 6, 8, ??? | Skip counting (multiplication in disguise) |
| Number ×2 | 1, 2, 4, 8, ??? | Doubling pattern |
| Number mixed | 1, 4, 9, 16, ??? | Squares (advanced) |

For a 4yo: picture patterns using game sprites (Sparky, Dum Dum, star). "What comes next?"

For a 7yo: number patterns that connect to multiplication tables. "2, 4, 6, 8, ???" is the 2× table as a pattern.

### 3. Balance Puzzles (visual algebra)

#### What it is

A balance scale. Left side and right side must be equal. Some values are known, some are mystery blocks.

```
    ⚖️
   / \
  /   \
[3][?]  [7]

"The scale is balanced. 3 plus what equals 7?"
```

This IS our number bonds but as a physical metaphor. The domain already handles number bonds as an operation. The new part is the renderer — a balance scale instead of text.

Scales naturally:
- Band 1-2: single unknown. `3 + ? = 5`
- Band 3-4: unknown on either side. `? + 4 = 11` or `8 - ? = 3`
- Band 5+: multiple unknowns. `? + ? = 8` where both boxes must be equal (so ? = 4)
- Advanced: `2 × ? = 12`

The balance tips visually when the kid guesses wrong — immediate physical feedback without words.

### 4. Mini Sudoku (pure logic)

#### What it is

4×4 grid with pictures instead of numbers for young kids.

```
┌────┬────┬────┬────┐
│ 🐱 │    │ 🐶 │    │
├────┼────┼────┼────┤
│    │ 🐶 │    │ 🐱 │
├────┼────┼────┼────┤
│ 🐶 │    │    │    │
├────┼────┼────┼────┤
│    │ 🐱 │    │ 🐶 │
└────┘────┴────┴────┘

"Each row has one of each animal.
 Each column has one of each animal.
 Each 2×2 box has one of each animal."
```

For 4yo: 4×4 with pictures (4 animals). No arithmetic at all — pure constraint satisfaction.
For 7yo: 6×6 with numbers. Still no arithmetic but harder logic.

No direct arithmetic connection, but develops the working memory and systematic reasoning that carrying/borrowing requires.

### 5. Spatial/Tangram (future)

Arrange shapes to fill a target outline. Develops geometric thinking. Canvas-based drag-and-drop. Future feature — complex renderer.

## Domain Model (Rust)

### New module: `robot-buddy-domain/src/logic/`

```
robot-buddy-domain/src/logic/
  mod.rs
  kenken.rs           # KenKen generation, validation, hints
  patterns.rs         # Pattern sequence generation, validation
  balance.rs          # Balance puzzle generation (uses existing number generation)
  sudoku.rs           # Mini Sudoku generation, validation
```

### KenKen domain

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KenKenPuzzle {
    pub grid_size: u8,
    pub grid: Vec<Vec<Option<u8>>>,          // pre-filled cells (None = empty)
    pub solution: Vec<Vec<u8>>,               // full solution
    pub cages: Vec<Cage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cage {
    pub cells: Vec<(usize, usize)>,           // (row, col) pairs
    pub target: i32,
    pub operation: Operation,                  // reuses types::Operation
    pub display_label: String,                 // "5+" or "12×"
}

impl KenKenPuzzle {
    /// Check if placing `value` at (row, col) violates any constraint
    pub fn validate_placement(&self, row: usize, col: usize, value: u8) -> ValidationResult {
        // Check row uniqueness
        // Check column uniqueness
        // Check cage arithmetic (if cage is complete)
        // Returns: Valid, RowConflict(existing_col), ColConflict(existing_row), CageWrong
    }

    /// Generate a hint for the kid
    pub fn get_hint(&self, grid_state: &Vec<Vec<Option<u8>>>) -> Hint {
        // Find the most constrained empty cell (fewest valid options)
        // Returns: Hint { row, col, hint_type }
        // hint_type: OnlyOneOption(value), RowElimination(values_eliminated), CageConstraint(text)
    }

    /// Check if the puzzle is complete and correct
    pub fn is_solved(&self, grid_state: &Vec<Vec<Option<u8>>>) -> bool
}

/// Generate a KenKen puzzle
pub fn generate_kenken(
    grid_size: u8,
    allowed_operations: &[Operation],
    difficulty: KenKenDifficulty,
    rng: &mut impl Rng,
) -> KenKenPuzzle {
    // 1. Generate a valid Latin square (random permutation rows/cols)
    // 2. Partition cells into cages (random contiguous groups)
    // 3. Assign operations and compute targets from the solution
    // 4. Remove pre-filled cells based on difficulty
    // 5. Verify unique solution (backtracking solver)
}

#[derive(Debug, Clone)]
pub struct KenKenDifficulty {
    pub min_cage_size: usize,    // 1 (single cell = given) to grid_size
    pub max_cage_size: usize,
    pub num_givens: usize,       // pre-filled cells (more = easier)
}
```

### Validation result (for feedback)

```rust
pub enum ValidationResult {
    Valid,
    RowConflict { col: usize },        // "There's already a 2 in this row"
    ColConflict { row: usize },        // "There's already a 3 in this column"
    CageArithmeticWrong,               // "These numbers don't add to 5"
}
```

The renderer can use this for visual feedback — highlight the conflicting cell, show the cage constraint that failed. Natural consequences, not "WRONG!"

### Pattern domain

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternPuzzle {
    pub pattern_type: PatternType,
    pub visible_elements: Vec<PatternElement>,   // what the kid sees
    pub answer_position: usize,                   // which position to fill
    pub correct_answer: PatternElement,
    pub choices: Vec<PatternElement>,              // multiple choice options
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PatternElement {
    Sprite { name: String },          // "sparky", "dum_dum", "star"
    Color { color: String },          // "red", "blue", "green"
    Number { value: i32 },
    Shape { shape: String },          // "circle", "square", "triangle"
}

pub fn generate_pattern(
    complexity: PatternComplexity,
    numeric: bool,                     // false = pictures, true = numbers
    rng: &mut impl Rng,
) -> PatternPuzzle
```

### Balance domain

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalancePuzzle {
    pub left_side: Vec<BalanceItem>,
    pub right_side: Vec<BalanceItem>,
    pub correct_answer: i32,           // what the unknown equals
    pub choices: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BalanceItem {
    Known { value: i32 },
    Unknown,                            // the ? block
    Operation { op: Operation },        // + or × between items
}
```

Balance generation reuses `generate_numbers` from the challenge generator — same band scaling, same sub-skill tracking. The puzzle is just a different visual for the same math.

## Renderers

### KenKenRenderer

```
src/presentation/renderers/
  kenken-renderer.js
```

Canvas grid. Thick lines for cage boundaries, thin for cell boundaries. Numbers in cells. Cage target+operation label in top-left corner of each cage. Tap a cell → show number picker (1 to grid_size). Wrong placement → conflicting cell highlights red briefly (natural consequence). Complete → celebration.

Show-me: highlights the most constrained cell + explains why ("This row already has 1 and 3, so this cell must be 2").

Tell-me: fills one cell.

### PatternRenderer

Renders a horizontal sequence of sprites/colors/numbers with a gap. Choices below. Animations: correct answer slides into place. Wrong choice bounces back.

### BalanceRenderer

A balance scale with items on each side. The scale tips when values don't match. The kid fills the unknown. Visual tipping gives immediate physical feedback — more concrete than text.

## Integration with Adaptive System

### New event type

```rust
#[serde(rename = "PUZZLE_COMPLETED")]
PuzzleCompleted {
    puzzle_type: PuzzleType,          // KenKen, Pattern, Balance, Sudoku
    difficulty: PuzzleDifficulty,
    correct: bool,
    hints_used: u8,
    time_ms: f64,
    // KenKen-specific:
    grid_size: Option<u8>,
    operations_used: Option<Vec<Operation>>,
    constraint_violations: Option<u8>,  // how many wrong placements before solving
}
```

### Learner profile addition

```rust
pub struct LearnerProfile {
    // ... existing arithmetic fields ...

    // Logic puzzle tracking
    pub kenken_level: u8,              // grid size: 2, 3, 4, 6
    pub pattern_level: u8,             // complexity tier
    pub logic_confidence: f64,         // 0-1, adapts like arithmetic scaffolding
}
```

### How KenKen uses arithmetic data

When generating a KenKen, the cage operations are filtered by the kid's arithmetic ability:

```rust
fn kenken_operations_for_profile(profile: &LearnerProfile) -> Vec<Operation> {
    let mut ops = vec![Operation::Add];
    if profile.math_band >= 3 { ops.push(Operation::Sub); }
    if profile.math_band >= 5 { ops.push(Operation::Multiply); }
    if profile.math_band >= 9 { ops.push(Operation::Divide); }
    ops
}
```

A kid at band 2 gets addition-only KenKen. A kid at band 8 gets KenKen with multiplication cages — practicing multiplication tables in the context of a puzzle, not a quiz. The arithmetic is the tool, the puzzle is the goal.

### How arithmetic uses puzzle data

If a kid solves KenKen cages with multiplication fluently but struggles with standalone multiplication challenges, the system knows: they CAN multiply, they just need context. The framing dial adjusts — more story-embedded math, fewer naked equations.

## Challenge Lifecycle Integration

Logic puzzles use the existing challenge lifecycle but with different phases:

```
Presented → InProgress → Hint → InProgress → Complete
```

`InProgress` is new — the kid is working on the puzzle, placing numbers, getting feedback. It's not a single-answer challenge, it's a multi-step interaction. The lifecycle reducer gets new actions:

```rust
CellPlaced { row: usize, col: usize, value: u8 }
CellCleared { row: usize, col: usize }
RequestHint
```

The reducer validates each placement and tracks constraint violations. The presentation shows visual feedback per the `ValidationResult`.

## Implementation Plan

**Phase 1: KenKen**
- `robot-buddy-domain/src/logic/kenken.rs` — generation, validation, hints, solver
- `KenKenRenderer` in JS — grid, tap to place, cage labels, conflict highlighting
- Wire into challenge lifecycle with InProgress phase
- Adaptive system filters cage operations by arithmetic band
- Boundary tests for KenKenPuzzle struct

**Phase 2: Pattern sequences**
- `robot-buddy-domain/src/logic/patterns.rs` — generation, validation
- `PatternRenderer` in JS — sequence display, choices, animations
- Picture patterns (4yo) and numeric patterns (7yo)

**Phase 3: Balance puzzles**
- `robot-buddy-domain/src/logic/balance.rs` — generation (reuses number gen)
- `BalanceRenderer` in JS — scale visual, tipping animation
- Uses existing band system for number difficulty

**Phase 4: Mini Sudoku**
- `robot-buddy-domain/src/logic/sudoku.rs` — 4×4 and 6×6 generation
- Picture mode (animals) for young kids, number mode for older
- Shares the KenKenRenderer grid infrastructure

## Open Questions

- Should KenKen cages allow single-cell "given" cages? (A cage with one cell and no operation = that number is given.) This is how standard KenKen controls difficulty — more givens = easier. But for a 4yo, explicit pre-filled cells might be clearer than a cage that says "2" with one cell.
- Should the kid be able to pencil-mark? (Write small candidate numbers in a cell before committing.) This is a key Sudoku strategy. For a 4yo it's too complex. For a 7yo it's valuable. Could be unlocked at higher kenken_level.
- How do logic puzzles interact with Dum Dums? Complete a puzzle = earn Dum Dums? Or should puzzles be their own reward? (The puzzle IS the game — adding Dum Dums might make it broccoli-adjacent.)
- Should Sparky help during KenKen? ("Hmm, look at row 2 — what's missing?") Or should the kid discover on their own? The show-me/tell-me pattern applies but the hints need to be constraint-aware, not arithmetic-aware.
