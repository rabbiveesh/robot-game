//! Mini-Sudoku puzzles. Pure logic — no rendering, no input.
//!
//! Pure constraint satisfaction with no arithmetic at all: each row, column, and
//! box holds the symbols 1..=N exactly once. For young kids the UI swaps the
//! numbers for animal pictures (4×4, 2×2 boxes); older kids get 6×6 (2×3 boxes)
//! with numbers. It builds the working memory and systematic reasoning that
//! carrying/borrowing later require — and it passes the broccoli test by
//! construction (nobody calls Sudoku a math test).
//!
//! Public surface (mirrors `kenken`):
//!   - `generate_sudoku(grid_size, rng)` / `generate_for_level(level, rng)`
//!   - `SudokuSession::new(puzzle)` → fresh session in InProgress
//!   - `sudoku_reducer(session, action)` → new session
//!
//! Two actions: `CellPlaced`, `CellCleared`. Like KenKen, a row/column/box
//! conflict is rejected (the cell stays empty) so the kid can immediately try a
//! different symbol and never paints themselves into an unsolvable corner.

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

// ─── Types ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SudokuPuzzle {
    pub grid_size: u8,
    pub box_rows: u8,
    pub box_cols: u8,
    pub solution: Vec<Vec<u8>>,
    /// Starting board: `Some` cells are immutable givens, `None` are blanks.
    pub givens: Vec<Vec<Option<u8>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SudokuPhase {
    InProgress,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SudokuValidation {
    Valid,
    RowConflict { col: u8 },
    ColConflict { row: u8 },
    BoxConflict { row: u8, col: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SudokuSession {
    pub puzzle: SudokuPuzzle,
    pub grid: Vec<Vec<Option<u8>>>,
    pub phase: SudokuPhase,
    pub constraint_violations: u8,
    pub last_violation: Option<SudokuValidation>,
}

impl SudokuSession {
    pub fn new(puzzle: SudokuPuzzle) -> Self {
        let grid = puzzle.givens.clone();
        // A well-formed puzzle always leaves blanks, but guard against a fully
        // given board the same way KenKen does — report Complete rather than
        // freeze the kid on a finished grid that accepts no input.
        let phase = if is_solved(&puzzle, &grid) {
            SudokuPhase::Complete
        } else {
            SudokuPhase::InProgress
        };
        SudokuSession { puzzle, grid, phase, constraint_violations: 0, last_violation: None }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SudokuAction {
    CellPlaced { row: u8, col: u8, value: u8 },
    CellCleared { row: u8, col: u8 },
}

// ─── Reducer ────────────────────────────────────────────

fn is_given(puzzle: &SudokuPuzzle, row: u8, col: u8) -> bool {
    puzzle.givens[row as usize][col as usize].is_some()
}

pub fn sudoku_reducer(state: SudokuSession, action: SudokuAction) -> SudokuSession {
    if state.phase == SudokuPhase::Complete {
        return state;
    }
    match action {
        SudokuAction::CellPlaced { row, col, value } => {
            let n = state.puzzle.grid_size;
            if row >= n || col >= n || value < 1 || value > n {
                return state;
            }
            if is_given(&state.puzzle, row, col) {
                return state;
            }
            let validation = validate_placement(&state, row, col, value);
            let (grid, last_violation, violations) = match validation {
                SudokuValidation::Valid => {
                    let mut g = state.grid.clone();
                    g[row as usize][col as usize] = Some(value);
                    (g, None, state.constraint_violations)
                }
                v => (state.grid.clone(), Some(v), state.constraint_violations.saturating_add(1)),
            };
            let phase = if is_solved(&state.puzzle, &grid) {
                SudokuPhase::Complete
            } else {
                SudokuPhase::InProgress
            };
            SudokuSession { grid, phase, last_violation, constraint_violations: violations, ..state }
        }
        SudokuAction::CellCleared { row, col } => {
            let n = state.puzzle.grid_size;
            if row >= n || col >= n || is_given(&state.puzzle, row, col) {
                return state;
            }
            let mut grid = state.grid.clone();
            grid[row as usize][col as usize] = None;
            SudokuSession { grid, last_violation: None, ..state }
        }
    }
}

// ─── Validation ─────────────────────────────────────────

fn box_origin(puzzle: &SudokuPuzzle, row: u8, col: u8) -> (u8, u8) {
    let br = (row / puzzle.box_rows) * puzzle.box_rows;
    let bc = (col / puzzle.box_cols) * puzzle.box_cols;
    (br, bc)
}

fn validate_placement(state: &SudokuSession, row: u8, col: u8, value: u8) -> SudokuValidation {
    let n = state.puzzle.grid_size;
    for c in 0..n {
        if c != col && state.grid[row as usize][c as usize] == Some(value) {
            return SudokuValidation::RowConflict { col: c };
        }
    }
    for r in 0..n {
        if r != row && state.grid[r as usize][col as usize] == Some(value) {
            return SudokuValidation::ColConflict { row: r };
        }
    }
    let (br, bc) = box_origin(&state.puzzle, row, col);
    for r in br..br + state.puzzle.box_rows {
        for c in bc..bc + state.puzzle.box_cols {
            if (r, c) != (row, col) && state.grid[r as usize][c as usize] == Some(value) {
                return SudokuValidation::BoxConflict { row: r, col: c };
            }
        }
    }
    SudokuValidation::Valid
}

pub fn is_solved(puzzle: &SudokuPuzzle, grid: &[Vec<Option<u8>>]) -> bool {
    let n = puzzle.grid_size as usize;
    let mut full = vec![vec![0u8; n]; n];
    for r in 0..n {
        for c in 0..n {
            match grid[r][c] {
                Some(v) if v >= 1 && v as usize <= n => full[r][c] = v,
                _ => return false,
            }
        }
    }
    groups_unique(puzzle, &full)
}

/// True iff every row, column, and box of a full grid holds 1..=N once.
fn groups_unique(puzzle: &SudokuPuzzle, full: &[Vec<u8>]) -> bool {
    let n = puzzle.grid_size as usize;
    let seen_ok = |vals: &[u8]| -> bool {
        let mut seen = vec![false; n];
        for &v in vals {
            if v < 1 || v as usize > n || seen[(v - 1) as usize] {
                return false;
            }
            seen[(v - 1) as usize] = true;
        }
        true
    };
    for r in 0..n {
        if !seen_ok(&full[r]) {
            return false;
        }
    }
    for c in 0..n {
        let col: Vec<u8> = (0..n).map(|r| full[r][c]).collect();
        if !seen_ok(&col) {
            return false;
        }
    }
    let (br, bc) = (puzzle.box_rows as usize, puzzle.box_cols as usize);
    for box_r in (0..n).step_by(br) {
        for box_c in (0..n).step_by(bc) {
            let mut vals = Vec::with_capacity(n);
            for r in box_r..box_r + br {
                for c in box_c..box_c + bc {
                    vals.push(full[r][c]);
                }
            }
            if !seen_ok(&vals) {
                return false;
            }
        }
    }
    true
}

// ─── Generation ─────────────────────────────────────────

/// Box dimensions (rows, cols) for a supported grid size. 4→2×2, 6→2×3.
pub fn sudoku_box_dims(grid_size: u8) -> (u8, u8) {
    match grid_size {
        4 => (2, 2),
        6 => (2, 3),
        9 => (3, 3),
        // Fall back to a near-square factor pair for any other size.
        n => {
            let mut br = (n as f64).sqrt() as u8;
            while br > 1 && n % br != 0 {
                br -= 1;
            }
            (br.max(1), n / br.max(1))
        }
    }
}

pub fn generate_sudoku(grid_size: u8, rng: &mut impl Rng) -> SudokuPuzzle {
    let (box_rows, box_cols) = sudoku_box_dims(grid_size);
    let n = grid_size as usize;
    let solution = full_solution(grid_size, box_rows, box_cols, rng);

    // Hole-digging: remove symbols while the puzzle keeps a unique solution,
    // down to a difficulty-appropriate number of givens.
    let target_givens = givens_target(grid_size);
    let mut givens: Vec<Vec<Option<u8>>> =
        solution.iter().map(|row| row.iter().map(|&v| Some(v)).collect()).collect();
    let mut filled = n * n;

    let mut cells: Vec<(usize, usize)> = (0..n).flat_map(|r| (0..n).map(move |c| (r, c))).collect();
    cells.shuffle(rng);

    let proto = SudokuPuzzle { grid_size, box_rows, box_cols, solution: solution.clone(), givens: vec![] };
    for (r, c) in cells {
        if filled <= target_givens {
            break;
        }
        let saved = givens[r][c];
        givens[r][c] = None;
        // Keep the dig only if the board still has exactly one solution.
        if count_solutions(&proto, &givens, 2) == 1 {
            filled -= 1;
        } else {
            givens[r][c] = saved;
        }
    }

    SudokuPuzzle { grid_size, box_rows, box_cols, solution, givens }
}

/// Difficulty levels (1 = easiest). L1/L2 are 4×4 (more/fewer givens), L3+ are
/// 6×6. Mirrors the spec: pictures-and-small for young kids, bigger for older.
pub fn generate_for_level(level: u8, rng: &mut impl Rng) -> SudokuPuzzle {
    let grid_size = if level <= 2 { 4 } else { 6 };
    generate_sudoku(grid_size, rng)
}

fn givens_target(grid_size: u8) -> usize {
    match grid_size {
        4 => 8,   // of 16 — comfortable for a 4-year-old
        6 => 16,  // of 36
        _ => (grid_size as usize * grid_size as usize) / 2,
    }
}

fn full_solution(grid_size: u8, box_rows: u8, box_cols: u8, rng: &mut impl Rng) -> Vec<Vec<u8>> {
    let n = grid_size as usize;
    let mut grid = vec![vec![0u8; n]; n];
    let proto = SudokuPuzzle { grid_size, box_rows, box_cols, solution: vec![], givens: vec![] };
    fill_recursive(&mut grid, 0, &proto, rng);
    grid
}

fn fill_recursive(grid: &mut Vec<Vec<u8>>, idx: usize, proto: &SudokuPuzzle, rng: &mut impl Rng) -> bool {
    let n = proto.grid_size as usize;
    if idx == n * n {
        return true;
    }
    let (row, col) = (idx / n, idx % n);
    let mut candidates: Vec<u8> = (1..=proto.grid_size).collect();
    candidates.shuffle(rng);
    for v in candidates {
        if placement_ok(grid, row, col, v, proto) {
            grid[row][col] = v;
            if fill_recursive(grid, idx + 1, proto, rng) {
                return true;
            }
            grid[row][col] = 0;
        }
    }
    false
}

fn placement_ok(grid: &[Vec<u8>], row: usize, col: usize, v: u8, proto: &SudokuPuzzle) -> bool {
    let n = proto.grid_size as usize;
    for c in 0..n {
        if c != col && grid[row][c] == v {
            return false;
        }
    }
    for r in 0..n {
        if r != row && grid[r][col] == v {
            return false;
        }
    }
    let (br, bc) = (proto.box_rows as usize, proto.box_cols as usize);
    let (bor, boc) = ((row / br) * br, (col / bc) * bc);
    for r in bor..bor + br {
        for c in boc..boc + bc {
            if (r, c) != (row, col) && grid[r][c] == v {
                return false;
            }
        }
    }
    true
}

/// Count solutions of a partially-filled board, capped at `max` (so uniqueness
/// checks short-circuit at 2).
pub fn count_solutions(proto: &SudokuPuzzle, givens: &[Vec<Option<u8>>], max: usize) -> usize {
    let mut grid: Vec<Vec<u8>> = givens
        .iter()
        .map(|row| row.iter().map(|c| c.unwrap_or(0)).collect())
        .collect();
    let mut count = 0;
    solve_count(&mut grid, 0, proto, &mut count, max);
    count
}

fn solve_count(grid: &mut Vec<Vec<u8>>, idx: usize, proto: &SudokuPuzzle, count: &mut usize, max: usize) {
    let n = proto.grid_size as usize;
    if *count >= max {
        return;
    }
    if idx == n * n {
        *count += 1;
        return;
    }
    let (row, col) = (idx / n, idx % n);
    if grid[row][col] != 0 {
        solve_count(grid, idx + 1, proto, count, max);
        return;
    }
    for v in 1..=proto.grid_size {
        if placement_ok(grid, row, col, v, proto) {
            grid[row][col] = v;
            solve_count(grid, idx + 1, proto, count, max);
            grid[row][col] = 0;
            if *count >= max {
                return;
            }
        }
    }
}

// ─── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn givens_count(p: &SudokuPuzzle) -> usize {
        p.givens.iter().flatten().filter(|c| c.is_some()).count()
    }

    #[test]
    fn generated_solution_is_valid_sudoku() {
        for size in [4u8, 6] {
            for seed in 0..40u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_sudoku(size, &mut r);
                assert!(groups_unique(&p, &p.solution),
                    "size {size} seed {seed}: solution must satisfy row/col/box");
            }
        }
    }

    #[test]
    fn givens_are_consistent_with_the_solution() {
        for size in [4u8, 6] {
            for seed in 0..40u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_sudoku(size, &mut r);
                for row in 0..size as usize {
                    for col in 0..size as usize {
                        if let Some(v) = p.givens[row][col] {
                            assert_eq!(v, p.solution[row][col],
                                "size {size} seed {seed}: given disagrees with solution at {row},{col}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn puzzles_have_a_unique_solution() {
        for size in [4u8, 6] {
            for seed in 0..30u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_sudoku(size, &mut r);
                let proto = SudokuPuzzle { givens: vec![], ..p.clone() };
                assert_eq!(count_solutions(&proto, &p.givens, 3), 1,
                    "size {size} seed {seed}: must be uniquely solvable");
            }
        }
    }

    #[test]
    fn puzzles_always_leave_blanks_to_solve() {
        // Never fully given — that would freeze the kid (cf. the KenKen bug).
        for size in [4u8, 6] {
            for seed in 0..50u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_sudoku(size, &mut r);
                let total = (size as usize) * (size as usize);
                assert!(givens_count(&p) < total,
                    "size {size} seed {seed}: puzzle fully given would freeze");
                let s = SudokuSession::new(p);
                assert_eq!(s.phase, SudokuPhase::InProgress);
            }
        }
    }

    #[test]
    fn placing_the_solution_completes_the_puzzle() {
        let mut r = SmallRng::seed_from_u64(5);
        let p = generate_sudoku(4, &mut r);
        let mut s = SudokuSession::new(p.clone());
        for row in 0..4u8 {
            for col in 0..4u8 {
                if p.givens[row as usize][col as usize].is_none() {
                    let v = p.solution[row as usize][col as usize];
                    s = sudoku_reducer(s, SudokuAction::CellPlaced { row, col, value: v });
                }
            }
        }
        assert_eq!(s.phase, SudokuPhase::Complete);
        assert_eq!(s.constraint_violations, 0, "placing the solution never conflicts");
    }

    #[test]
    fn row_conflict_is_rejected_and_counted() {
        // Build a tiny known 4x4 to force a deterministic conflict.
        let mut r = SmallRng::seed_from_u64(1);
        let p = generate_sudoku(4, &mut r);
        // Find a blank cell and a value already present in its row.
        let mut placed = false;
        'outer: for row in 0..4u8 {
            let present: Vec<u8> = (0..4u8)
                .filter_map(|c| p.givens[row as usize][c as usize])
                .collect();
            if present.is_empty() {
                continue;
            }
            for col in 0..4u8 {
                if p.givens[row as usize][col as usize].is_none() {
                    let bad = present[0];
                    let s = SudokuSession::new(p.clone());
                    let s = sudoku_reducer(s, SudokuAction::CellPlaced { row, col, value: bad });
                    assert_eq!(s.grid[row as usize][col as usize], None,
                        "a row conflict must leave the cell empty");
                    assert!(matches!(s.last_violation, Some(SudokuValidation::RowConflict { .. })));
                    assert_eq!(s.constraint_violations, 1);
                    placed = true;
                    break 'outer;
                }
            }
        }
        assert!(placed, "expected to find a blank cell sharing a row with a given");
    }

    #[test]
    fn givens_cannot_be_overwritten_or_cleared() {
        let mut r = SmallRng::seed_from_u64(2);
        let p = generate_sudoku(4, &mut r);
        let (gr, gc) = (0..4u8)
            .flat_map(|r| (0..4u8).map(move |c| (r, c)))
            .find(|&(r, c)| p.givens[r as usize][c as usize].is_some())
            .unwrap();
        let original = p.givens[gr as usize][gc as usize];
        let s = SudokuSession::new(p);
        let s = sudoku_reducer(s, SudokuAction::CellPlaced { row: gr, col: gc, value: 1 });
        assert_eq!(s.grid[gr as usize][gc as usize], original, "givens are immutable");
        let s = sudoku_reducer(s, SudokuAction::CellCleared { row: gr, col: gc });
        assert_eq!(s.grid[gr as usize][gc as usize], original, "givens can't be cleared");
    }

    #[test]
    fn cleared_cell_becomes_blank_again() {
        let mut r = SmallRng::seed_from_u64(3);
        let p = generate_sudoku(4, &mut r);
        let (br, bc) = (0..4u8)
            .flat_map(|r| (0..4u8).map(move |c| (r, c)))
            .find(|&(r, c)| p.givens[r as usize][c as usize].is_none())
            .unwrap();
        let v = p.solution[br as usize][bc as usize];
        let s = SudokuSession::new(p);
        let s = sudoku_reducer(s, SudokuAction::CellPlaced { row: br, col: bc, value: v });
        assert_eq!(s.grid[br as usize][bc as usize], Some(v));
        let s = sudoku_reducer(s, SudokuAction::CellCleared { row: br, col: bc });
        assert_eq!(s.grid[br as usize][bc as usize], None);
    }

    #[test]
    fn box_dims_are_correct() {
        assert_eq!(sudoku_box_dims(4), (2, 2));
        assert_eq!(sudoku_box_dims(6), (2, 3));
        assert_eq!(sudoku_box_dims(9), (3, 3));
    }

    #[test]
    fn level_picks_grid_size() {
        let mut r = SmallRng::seed_from_u64(0);
        assert_eq!(generate_for_level(1, &mut r).grid_size, 4);
        assert_eq!(generate_for_level(2, &mut r).grid_size, 4);
        assert_eq!(generate_for_level(3, &mut r).grid_size, 6);
    }
}
