//! KenKen puzzle domain. Pure logic — no rendering, no input.
//!
//! Public surface (the only entrypoints other code should touch):
//!   - `generate_kenken(grid_size, allowed_ops, rng)` → KenKenPuzzle
//!   - `KenKenSession::new(puzzle)` → fresh session in InProgress phase
//!   - `kenken_reducer(session, action)` → new session
//!
//! Three actions exist: `CellPlaced`, `CellCleared`, `RequestHint`. The UI
//! converts taps and key presses into these; the reducer is the single point
//! where session state changes.

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

// ─── Types ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CageOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cage {
    pub cells: Vec<(u8, u8)>,
    pub target: i32,
    pub operation: CageOp,
    pub display_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KenKenPuzzle {
    pub grid_size: u8,
    pub solution: Vec<Vec<u8>>,
    pub cages: Vec<Cage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KenKenPhase {
    InProgress,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ValidationResult {
    Valid,
    RowConflict { col: u8 },
    ColConflict { row: u8 },
    CageWrong,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hint {
    pub row: u8,
    pub col: u8,
    pub value: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KenKenSession {
    pub puzzle: KenKenPuzzle,
    pub grid: Vec<Vec<Option<u8>>>,
    pub phase: KenKenPhase,
    pub hints_used: u8,
    pub constraint_violations: u8,
    pub last_violation: Option<ValidationResult>,
}

impl KenKenSession {
    pub fn new(puzzle: KenKenPuzzle) -> Self {
        let n = puzzle.grid_size as usize;
        let grid = vec![vec![None; n]; n];
        // Pre-fill any single-cell cages — they are visual "givens".
        let mut grid = grid;
        for cage in &puzzle.cages {
            if cage.cells.len() == 1 {
                let (r, c) = cage.cells[0];
                grid[r as usize][c as usize] = Some(cage.target as u8);
            }
        }
        KenKenSession {
            puzzle,
            grid,
            phase: KenKenPhase::InProgress,
            hints_used: 0,
            constraint_violations: 0,
            last_violation: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum KenKenAction {
    CellPlaced { row: u8, col: u8, value: u8 },
    CellCleared { row: u8, col: u8 },
    RequestHint,
}

// ─── Reducer ────────────────────────────────────────────

pub fn kenken_reducer(state: KenKenSession, action: KenKenAction) -> KenKenSession {
    if state.phase == KenKenPhase::Complete {
        return state;
    }

    match action {
        KenKenAction::CellPlaced { row, col, value } => {
            let n = state.puzzle.grid_size;
            if row >= n || col >= n || value < 1 || value > n {
                return state;
            }
            // Single-cell cages (givens) are immutable.
            if is_given_cell(&state.puzzle, row, col) {
                return state;
            }

            let validation = validate_placement(&state, row, col, value);
            let mut grid = state.grid.clone();
            grid[row as usize][col as usize] = Some(value);

            let (last_violation, violations) = match validation {
                ValidationResult::Valid => (None, state.constraint_violations),
                v => (Some(v), state.constraint_violations.saturating_add(1)),
            };

            let phase = if is_solved(&state.puzzle, &grid) {
                KenKenPhase::Complete
            } else {
                KenKenPhase::InProgress
            };

            KenKenSession {
                grid,
                constraint_violations: violations,
                last_violation,
                phase,
                ..state
            }
        }

        KenKenAction::CellCleared { row, col } => {
            let n = state.puzzle.grid_size;
            if row >= n || col >= n {
                return state;
            }
            if is_given_cell(&state.puzzle, row, col) {
                return state;
            }
            let mut grid = state.grid.clone();
            grid[row as usize][col as usize] = None;
            KenKenSession {
                grid,
                last_violation: None,
                ..state
            }
        }

        KenKenAction::RequestHint => match compute_hint(&state.puzzle, &state.grid) {
            None => state,
            Some(h) => {
                let mut grid = state.grid.clone();
                grid[h.row as usize][h.col as usize] = Some(h.value);
                let phase = if is_solved(&state.puzzle, &grid) {
                    KenKenPhase::Complete
                } else {
                    KenKenPhase::InProgress
                };
                KenKenSession {
                    grid,
                    hints_used: state.hints_used.saturating_add(1),
                    last_violation: None,
                    phase,
                    ..state
                }
            }
        },
    }
}

fn is_given_cell(puzzle: &KenKenPuzzle, row: u8, col: u8) -> bool {
    puzzle
        .cages
        .iter()
        .any(|cg| cg.cells.len() == 1 && cg.cells[0] == (row, col))
}

// ─── Validation ─────────────────────────────────────────

fn validate_placement(state: &KenKenSession, row: u8, col: u8, value: u8) -> ValidationResult {
    let n = state.puzzle.grid_size;
    for c in 0..n {
        if c == col {
            continue;
        }
        if state.grid[row as usize][c as usize] == Some(value) {
            return ValidationResult::RowConflict { col: c };
        }
    }
    for r in 0..n {
        if r == row {
            continue;
        }
        if state.grid[r as usize][col as usize] == Some(value) {
            return ValidationResult::ColConflict { row: r };
        }
    }

    if let Some(cage) = state
        .puzzle
        .cages
        .iter()
        .find(|cg| cg.cells.contains(&(row, col)))
    {
        let cage_complete = cage.cells.iter().all(|&(r, c)| {
            (r, c) == (row, col) || state.grid[r as usize][c as usize].is_some()
        });
        if cage_complete {
            let values: Vec<u8> = cage
                .cells
                .iter()
                .map(|&(r, c)| {
                    if (r, c) == (row, col) {
                        value
                    } else {
                        state.grid[r as usize][c as usize].unwrap()
                    }
                })
                .collect();
            if !cage_values_match(&values, cage.target, cage.operation) {
                return ValidationResult::CageWrong;
            }
        }
    }

    ValidationResult::Valid
}

fn cage_values_match(values: &[u8], target: i32, op: CageOp) -> bool {
    match op {
        CageOp::Add => values.iter().map(|&v| v as i32).sum::<i32>() == target,
        CageOp::Mul => values.iter().map(|&v| v as i32).product::<i32>() == target,
        CageOp::Sub => {
            if values.len() != 2 {
                return false;
            }
            let (a, b) = (values[0] as i32, values[1] as i32);
            (a - b).abs() == target
        }
        CageOp::Div => {
            if values.len() != 2 {
                return false;
            }
            let mx = values[0].max(values[1]) as i32;
            let mn = values[0].min(values[1]) as i32;
            mn != 0 && mx % mn == 0 && mx / mn == target
        }
    }
}

fn cage_target_matches(cage: &Cage, full: &[Vec<u8>]) -> bool {
    let values: Vec<u8> = cage
        .cells
        .iter()
        .map(|&(r, c)| full[r as usize][c as usize])
        .collect();
    cage_values_match(&values, cage.target, cage.operation)
}

pub fn is_solved(puzzle: &KenKenPuzzle, grid: &[Vec<Option<u8>>]) -> bool {
    let n = puzzle.grid_size as usize;
    for row in grid {
        for cell in row {
            if cell.is_none() {
                return false;
            }
        }
    }
    let full: Vec<Vec<u8>> = grid
        .iter()
        .map(|row| row.iter().map(|c| c.unwrap()).collect())
        .collect();
    for r in 0..n {
        let mut seen = vec![false; n];
        for c in 0..n {
            let v = full[r][c];
            if v < 1 || v as usize > n {
                return false;
            }
            if seen[(v - 1) as usize] {
                return false;
            }
            seen[(v - 1) as usize] = true;
        }
    }
    for c in 0..n {
        let mut seen = vec![false; n];
        for r in 0..n {
            let v = full[r][c];
            if seen[(v - 1) as usize] {
                return false;
            }
            seen[(v - 1) as usize] = true;
        }
    }
    for cage in &puzzle.cages {
        if !cage_target_matches(cage, &full) {
            return false;
        }
    }
    true
}

// ─── Hints ──────────────────────────────────────────────

fn compute_hint(puzzle: &KenKenPuzzle, grid: &[Vec<Option<u8>>]) -> Option<Hint> {
    let n = puzzle.grid_size as usize;
    for r in 0..n {
        for c in 0..n {
            if grid[r][c].is_none() {
                return Some(Hint {
                    row: r as u8,
                    col: c as u8,
                    value: puzzle.solution[r][c],
                });
            }
        }
    }
    None
}

// ─── Generation ─────────────────────────────────────────

pub fn generate_kenken(
    grid_size: u8,
    allowed_ops: &[CageOp],
    rng: &mut impl Rng,
) -> KenKenPuzzle {
    assert!(grid_size >= 2, "kenken grid_size must be >= 2");
    let allowed: Vec<CageOp> = if allowed_ops.is_empty() {
        vec![CageOp::Add]
    } else {
        allowed_ops.to_vec()
    };

    for _ in 0..80 {
        let solution = random_latin_square(grid_size, rng);
        let cages = generate_cages(&solution, &allowed, rng);
        let puzzle = KenKenPuzzle {
            grid_size,
            solution: solution.clone(),
            cages,
        };
        if count_solutions(&puzzle, 2) == 1 {
            return puzzle;
        }
    }
    fallback_puzzle(grid_size)
}

fn random_latin_square(n: u8, rng: &mut impl Rng) -> Vec<Vec<u8>> {
    let n = n as usize;
    let mut rows: Vec<Vec<u8>> = (0..n)
        .map(|i| (0..n).map(|j| ((i + j) % n + 1) as u8).collect())
        .collect();
    rows.shuffle(rng);

    let mut col_perm: Vec<usize> = (0..n).collect();
    col_perm.shuffle(rng);
    let permuted: Vec<Vec<u8>> = rows
        .iter()
        .map(|row| col_perm.iter().map(|&c| row[c]).collect())
        .collect();

    let mut sym_perm: Vec<u8> = (1..=n as u8).collect();
    sym_perm.shuffle(rng);
    permuted
        .iter()
        .map(|row| row.iter().map(|&v| sym_perm[(v - 1) as usize]).collect())
        .collect()
}

fn generate_cages(
    solution: &[Vec<u8>],
    allowed_ops: &[CageOp],
    rng: &mut impl Rng,
) -> Vec<Cage> {
    let n = solution.len();
    let mut assigned: Vec<Vec<bool>> = vec![vec![false; n]; n];
    let mut cage_cells: Vec<Vec<(u8, u8)>> = Vec::new();

    let max_size: usize = if n <= 2 { 2 } else if n <= 4 { 3 } else { 4 };

    let mut order: Vec<(usize, usize)> = (0..n).flat_map(|r| (0..n).map(move |c| (r, c))).collect();
    order.shuffle(rng);

    for (r, c) in order {
        if assigned[r][c] {
            continue;
        }
        let target_size = rng.gen_range(1..=max_size);
        let cells = grow_cage((r, c), target_size, &assigned, n, rng);
        for &(cr, cc) in &cells {
            assigned[cr][cc] = true;
        }
        cage_cells.push(cells.iter().map(|&(r, c)| (r as u8, c as u8)).collect());
    }

    cage_cells
        .into_iter()
        .map(|cells| {
            let values: Vec<u8> = cells
                .iter()
                .map(|&(r, c)| solution[r as usize][c as usize])
                .collect();
            let op = pick_cage_op(&values, allowed_ops, rng);
            let target = compute_target(&values, op);
            let display_label = if cells.len() == 1 {
                format!("{}", target)
            } else {
                format!("{}{}", target, op_symbol(op))
            };
            Cage {
                cells,
                target,
                operation: op,
                display_label,
            }
        })
        .collect()
}

fn grow_cage(
    start: (usize, usize),
    target_size: usize,
    assigned: &[Vec<bool>],
    n: usize,
    rng: &mut impl Rng,
) -> Vec<(usize, usize)> {
    let mut cells = vec![start];
    while cells.len() < target_size {
        let mut neighbors: Vec<(usize, usize)> = Vec::new();
        for &(cr, cc) in &cells {
            for &(dr, dc) in &[(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let nr = cr as i32 + dr;
                let nc = cc as i32 + dc;
                if nr < 0 || nc < 0 || nr >= n as i32 || nc >= n as i32 {
                    continue;
                }
                let np = (nr as usize, nc as usize);
                if assigned[np.0][np.1] {
                    continue;
                }
                if cells.contains(&np) {
                    continue;
                }
                neighbors.push(np);
            }
        }
        if neighbors.is_empty() {
            break;
        }
        let pick = neighbors[rng.gen_range(0..neighbors.len())];
        cells.push(pick);
    }
    cells
}

fn pick_cage_op(values: &[u8], allowed: &[CageOp], rng: &mut impl Rng) -> CageOp {
    let mut candidates: Vec<CageOp> = Vec::new();
    for &op in allowed {
        match op {
            CageOp::Add => candidates.push(op),
            CageOp::Mul => candidates.push(op),
            CageOp::Sub if values.len() == 2 => candidates.push(op),
            CageOp::Div if values.len() == 2 => {
                let mx = values[0].max(values[1]);
                let mn = values[0].min(values[1]);
                if mn != 0 && mx % mn == 0 {
                    candidates.push(op);
                }
            }
            _ => {}
        }
    }
    if candidates.is_empty() {
        return CageOp::Add;
    }
    candidates[rng.gen_range(0..candidates.len())]
}

fn compute_target(values: &[u8], op: CageOp) -> i32 {
    match op {
        CageOp::Add => values.iter().map(|&v| v as i32).sum(),
        CageOp::Mul => values.iter().map(|&v| v as i32).product(),
        CageOp::Sub => {
            let a = values[0] as i32;
            let b = values[1] as i32;
            (a - b).abs()
        }
        CageOp::Div => {
            let mx = values[0].max(values[1]) as i32;
            let mn = values[0].min(values[1]) as i32;
            mx / mn
        }
    }
}

fn op_symbol(op: CageOp) -> &'static str {
    match op {
        CageOp::Add => "+",
        CageOp::Sub => "-",
        CageOp::Mul => "x",
        CageOp::Div => "/",
    }
}

// ─── Solver (uniqueness check) ──────────────────────────

pub fn count_solutions(puzzle: &KenKenPuzzle, max: usize) -> usize {
    let n = puzzle.grid_size as usize;
    let mut grid: Vec<Vec<u8>> = vec![vec![0; n]; n];
    let mut count = 0;
    solve_recursive(&mut grid, 0, n, puzzle, &mut count, max);
    count
}

fn solve_recursive(
    grid: &mut Vec<Vec<u8>>,
    idx: usize,
    n: usize,
    puzzle: &KenKenPuzzle,
    count: &mut usize,
    max: usize,
) {
    if *count >= max {
        return;
    }
    if idx == n * n {
        let full: Vec<Vec<u8>> = grid.clone();
        if puzzle
            .cages
            .iter()
            .all(|cg| cage_target_matches(cg, &full))
        {
            *count += 1;
        }
        return;
    }
    let row = idx / n;
    let col = idx % n;

    for v in 1..=n as u8 {
        if (0..col).any(|c| grid[row][c] == v) {
            continue;
        }
        if (0..row).any(|r| grid[r][col] == v) {
            continue;
        }
        grid[row][col] = v;
        let cage = puzzle
            .cages
            .iter()
            .find(|cg| cg.cells.contains(&(row as u8, col as u8)))
            .expect("every cell belongs to a cage");
        let cage_complete = cage
            .cells
            .iter()
            .all(|&(r, c)| grid[r as usize][c as usize] != 0);
        let cage_ok = !cage_complete || cage_target_matches(cage, grid);
        if cage_ok {
            solve_recursive(grid, idx + 1, n, puzzle, count, max);
        }
        grid[row][col] = 0;
        if *count >= max {
            return;
        }
    }
}

// ─── Fallback ───────────────────────────────────────────
//
// Used only when generation can't produce a unique-solution puzzle in 80 attempts.
// Hand-crafted small puzzles guarantee we never return a malformed KenKen.

fn fallback_puzzle(grid_size: u8) -> KenKenPuzzle {
    match grid_size {
        2 => KenKenPuzzle {
            grid_size: 2,
            solution: vec![vec![1, 2], vec![2, 1]],
            cages: vec![
                Cage {
                    cells: vec![(0, 0)],
                    target: 1,
                    operation: CageOp::Add,
                    display_label: "1".into(),
                },
                Cage {
                    cells: vec![(0, 1), (1, 0), (1, 1)],
                    target: 5,
                    operation: CageOp::Add,
                    display_label: "5+".into(),
                },
            ],
        },
        3 => KenKenPuzzle {
            grid_size: 3,
            solution: vec![vec![1, 2, 3], vec![2, 3, 1], vec![3, 1, 2]],
            cages: vec![
                Cage {
                    cells: vec![(0, 0)],
                    target: 1,
                    operation: CageOp::Add,
                    display_label: "1".into(),
                },
                Cage {
                    cells: vec![(0, 1), (0, 2)],
                    target: 5,
                    operation: CageOp::Add,
                    display_label: "5+".into(),
                },
                Cage {
                    cells: vec![(1, 0), (2, 0)],
                    target: 5,
                    operation: CageOp::Add,
                    display_label: "5+".into(),
                },
                Cage {
                    cells: vec![(1, 1), (1, 2), (2, 1), (2, 2)],
                    target: 7,
                    operation: CageOp::Add,
                    display_label: "7+".into(),
                },
            ],
        },
        n => {
            let n_us = n as usize;
            let solution: Vec<Vec<u8>> = (0..n_us)
                .map(|i| (0..n_us).map(|j| ((i + j) % n_us + 1) as u8).collect())
                .collect();
            let mut cells = Vec::new();
            for r in 0..n_us {
                for c in 0..n_us {
                    cells.push((r as u8, c as u8));
                }
            }
            let target: i32 = solution.iter().flat_map(|r| r.iter()).map(|&v| v as i32).sum();
            KenKenPuzzle {
                grid_size: n,
                solution,
                cages: vec![Cage {
                    cells,
                    target,
                    operation: CageOp::Add,
                    display_label: format!("{}+", target),
                }],
            }
        }
    }
}

// ─── Profile-driven helpers ─────────────────────────────

/// Map the kid's arithmetic band to the cage operations they should encounter.
/// Mirrors the spec's table (bands 1-2 add only, +sub at 3, +mul at 5, +div at 9).
pub fn cage_ops_for_band(band: u8) -> Vec<CageOp> {
    let mut ops = vec![CageOp::Add];
    if band >= 3 {
        ops.push(CageOp::Sub);
    }
    if band >= 5 {
        ops.push(CageOp::Mul);
    }
    if band >= 9 {
        ops.push(CageOp::Div);
    }
    ops
}

// ─── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn rng() -> SmallRng {
        SmallRng::seed_from_u64(42)
    }

    #[test]
    fn generates_2x2_with_unique_solution() {
        let mut r = rng();
        let p = generate_kenken(2, &[CageOp::Add], &mut r);
        assert_eq!(p.grid_size, 2);
        assert_eq!(count_solutions(&p, 5), 1);
    }

    #[test]
    fn generates_3x3_with_unique_solution() {
        let mut r = rng();
        let p = generate_kenken(3, &[CageOp::Add, CageOp::Sub], &mut r);
        assert_eq!(p.grid_size, 3);
        assert_eq!(count_solutions(&p, 5), 1);
    }

    #[test]
    fn generates_4x4_with_unique_solution() {
        let mut r = rng();
        let p = generate_kenken(4, &[CageOp::Add, CageOp::Sub, CageOp::Mul], &mut r);
        assert_eq!(p.grid_size, 4);
        assert_eq!(count_solutions(&p, 5), 1);
    }

    #[test]
    fn solution_satisfies_latin_square() {
        let mut r = rng();
        let p = generate_kenken(3, &[CageOp::Add], &mut r);
        let n = p.grid_size as usize;
        for row in &p.solution {
            let mut seen = vec![false; n];
            for &v in row {
                assert!(v >= 1 && v as usize <= n);
                assert!(!seen[(v - 1) as usize]);
                seen[(v - 1) as usize] = true;
            }
        }
        for c in 0..n {
            let mut seen = vec![false; n];
            for r in 0..n {
                let v = p.solution[r][c];
                assert!(!seen[(v - 1) as usize]);
                seen[(v - 1) as usize] = true;
            }
        }
    }

    #[test]
    fn cages_partition_grid() {
        let mut r = rng();
        let p = generate_kenken(3, &[CageOp::Add], &mut r);
        let n = p.grid_size as usize;
        let mut covered = vec![vec![0u8; n]; n];
        for cage in &p.cages {
            for &(r, c) in &cage.cells {
                covered[r as usize][c as usize] += 1;
            }
        }
        for row in &covered {
            for &v in row {
                assert_eq!(v, 1, "every cell belongs to exactly one cage");
            }
        }
    }

    #[test]
    fn cage_targets_match_solution() {
        let mut r = rng();
        let p = generate_kenken(3, &[CageOp::Add, CageOp::Sub], &mut r);
        for cage in &p.cages {
            assert!(cage_target_matches(cage, &p.solution));
        }
    }

    #[test]
    fn fresh_session_starts_in_progress() {
        let mut r = rng();
        let p = generate_kenken(2, &[CageOp::Add], &mut r);
        let s = KenKenSession::new(p);
        assert_eq!(s.phase, KenKenPhase::InProgress);
    }

    #[test]
    fn givens_pre_filled() {
        // Use the known fallback 2x2: cell (0,0) is a single-cell cage with value 1.
        let p = fallback_puzzle(2);
        let s = KenKenSession::new(p);
        assert_eq!(s.grid[0][0], Some(1));
        assert_eq!(s.grid[0][1], None);
    }

    #[test]
    fn placing_correct_value_validates() {
        let p = fallback_puzzle(2);
        let s = KenKenSession::new(p);
        let s = kenken_reducer(s, KenKenAction::CellPlaced { row: 0, col: 1, value: 2 });
        assert_eq!(s.grid[0][1], Some(2));
        assert_eq!(s.last_violation, None);
        assert_eq!(s.constraint_violations, 0);
    }

    #[test]
    fn placing_row_conflict_records_violation() {
        let p = fallback_puzzle(2);
        let s = KenKenSession::new(p);
        // (0,0)=1 is given; placing 1 at (0,1) collides with the row.
        let s = kenken_reducer(s, KenKenAction::CellPlaced { row: 0, col: 1, value: 1 });
        assert_eq!(s.last_violation, Some(ValidationResult::RowConflict { col: 0 }));
        assert_eq!(s.constraint_violations, 1);
    }

    #[test]
    fn solving_full_puzzle_completes_phase() {
        let p = fallback_puzzle(2);
        let s = KenKenSession::new(p);
        let s = kenken_reducer(s, KenKenAction::CellPlaced { row: 0, col: 1, value: 2 });
        let s = kenken_reducer(s, KenKenAction::CellPlaced { row: 1, col: 0, value: 2 });
        let s = kenken_reducer(s, KenKenAction::CellPlaced { row: 1, col: 1, value: 1 });
        assert_eq!(s.phase, KenKenPhase::Complete);
    }

    #[test]
    fn given_cells_cannot_be_overwritten() {
        let p = fallback_puzzle(2);
        let s = KenKenSession::new(p);
        let s = kenken_reducer(s, KenKenAction::CellPlaced { row: 0, col: 0, value: 2 });
        assert_eq!(s.grid[0][0], Some(1)); // unchanged
    }

    #[test]
    fn request_hint_fills_one_cell() {
        let p = fallback_puzzle(3);
        let s = KenKenSession::new(p);
        let before_filled = count_filled(&s.grid);
        let s = kenken_reducer(s, KenKenAction::RequestHint);
        let after_filled = count_filled(&s.grid);
        assert_eq!(after_filled, before_filled + 1);
        assert_eq!(s.hints_used, 1);
    }

    #[test]
    fn cell_cleared_resets_a_user_cell() {
        let p = fallback_puzzle(3);
        let s = KenKenSession::new(p);
        let s = kenken_reducer(s, KenKenAction::CellPlaced { row: 1, col: 2, value: 1 });
        assert_eq!(s.grid[1][2], Some(1));
        let s = kenken_reducer(s, KenKenAction::CellCleared { row: 1, col: 2 });
        assert_eq!(s.grid[1][2], None);
    }

    #[test]
    fn complete_phase_ignores_further_actions() {
        let p = fallback_puzzle(2);
        let s = KenKenSession::new(p);
        let s = kenken_reducer(s, KenKenAction::CellPlaced { row: 0, col: 1, value: 2 });
        let s = kenken_reducer(s, KenKenAction::CellPlaced { row: 1, col: 0, value: 2 });
        let s = kenken_reducer(s, KenKenAction::CellPlaced { row: 1, col: 1, value: 1 });
        assert_eq!(s.phase, KenKenPhase::Complete);
        let s = kenken_reducer(s.clone(), KenKenAction::CellCleared { row: 1, col: 1 });
        assert_eq!(s.grid[1][1], Some(1)); // ignored
    }

    #[test]
    fn cage_ops_scale_with_band() {
        assert_eq!(cage_ops_for_band(1), vec![CageOp::Add]);
        assert_eq!(cage_ops_for_band(3), vec![CageOp::Add, CageOp::Sub]);
        let b5 = cage_ops_for_band(5);
        assert!(b5.contains(&CageOp::Mul));
        assert!(!b5.contains(&CageOp::Div));
        let b10 = cage_ops_for_band(10);
        assert!(b10.contains(&CageOp::Div));
    }

    fn count_filled(grid: &[Vec<Option<u8>>]) -> usize {
        grid.iter()
            .flat_map(|row| row.iter())
            .filter(|c| c.is_some())
            .count()
    }
}
