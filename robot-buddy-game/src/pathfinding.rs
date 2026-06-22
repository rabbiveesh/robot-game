//! Grid pathfinding for tap-to-move. Pure logic — no macroquad, no rendering.
//!
//! Tap-to-move (tap-to-move-spec) needs to turn "the kid tapped that tile" into
//! a walk path. The core is a 4-directional BFS over the tile grid, parameter-
//! ized by a walkability predicate so it's testable without a `Map` (and reused
//! by quest travel steps). Screen→tile conversion and turning the path into
//! per-frame move intents stay in the game layer.

use std::collections::VecDeque;

pub type Tile = (usize, usize);

/// Breadth-first shortest path from `start` to `goal` over a `width`×`height`
/// grid, moving only up/down/left/right onto tiles where `is_walkable` is true.
///
/// Returns the steps AFTER `start` up to and including `goal` (so an empty Vec
/// means start == goal). `None` if `goal` is unreachable or not walkable. BFS
/// guarantees the fewest steps.
pub fn find_path(
    start: Tile,
    goal: Tile,
    width: usize,
    height: usize,
    is_walkable: impl Fn(usize, usize) -> bool,
) -> Option<Vec<Tile>> {
    if !in_bounds(goal, width, height) || !is_walkable(goal.0, goal.1) {
        return None;
    }
    if start == goal {
        return Some(Vec::new());
    }
    bfs(start, |t| t == goal, width, height, &is_walkable)
}

/// Path to the nearest tile ADJACENT to `goal` (used to walk up to an NPC or a
/// solid object you can't stand on). Returns the steps after `start`; an empty
/// Vec means `start` is already adjacent to `goal`. `None` if no adjacent tile
/// is reachable.
pub fn find_path_adjacent(
    start: Tile,
    goal: Tile,
    width: usize,
    height: usize,
    is_walkable: impl Fn(usize, usize) -> bool,
) -> Option<Vec<Tile>> {
    if is_adjacent(start, goal) {
        return Some(Vec::new());
    }
    let goal_neighbors: Vec<Tile> = neighbors(goal, width, height)
        .into_iter()
        .filter(|&(c, r)| is_walkable(c, r))
        .collect();
    if goal_neighbors.is_empty() {
        return None;
    }
    bfs(start, |t| goal_neighbors.contains(&t), width, height, &is_walkable)
}

fn bfs(
    start: Tile,
    is_goal: impl Fn(Tile) -> bool,
    width: usize,
    height: usize,
    is_walkable: &impl Fn(usize, usize) -> bool,
) -> Option<Vec<Tile>> {
    if !in_bounds(start, width, height) {
        return None;
    }
    let mut came_from: Vec<Vec<Option<Tile>>> = vec![vec![None; width]; height];
    let mut visited = vec![vec![false; width]; height];
    let mut queue = VecDeque::new();
    visited[start.1][start.0] = true;
    queue.push_back(start);

    while let Some(cur) = queue.pop_front() {
        if is_goal(cur) && cur != start {
            return Some(reconstruct(came_from, start, cur));
        }
        for (nc, nr) in neighbors(cur, width, height) {
            if visited[nr][nc] || !is_walkable(nc, nr) {
                continue;
            }
            visited[nr][nc] = true;
            came_from[nr][nc] = Some(cur);
            queue.push_back((nc, nr));
        }
    }
    None
}

fn reconstruct(came_from: Vec<Vec<Option<Tile>>>, start: Tile, goal: Tile) -> Vec<Tile> {
    let mut path = vec![goal];
    let mut cur = goal;
    while let Some(prev) = came_from[cur.1][cur.0] {
        if prev == start {
            break;
        }
        path.push(prev);
        cur = prev;
    }
    path.reverse();
    path
}

fn in_bounds((c, r): Tile, width: usize, height: usize) -> bool {
    c < width && r < height
}

fn neighbors((c, r): Tile, width: usize, height: usize) -> Vec<Tile> {
    let mut out = Vec::with_capacity(4);
    if c + 1 < width { out.push((c + 1, r)); }
    if c > 0 { out.push((c - 1, r)); }
    if r + 1 < height { out.push((c, r + 1)); }
    if r > 0 { out.push((c, r - 1)); }
    out
}

pub fn is_adjacent(a: Tile, b: Tile) -> bool {
    let dc = a.0.abs_diff(b.0);
    let dr = a.1.abs_diff(b.1);
    dc + dr == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a walkability predicate from an ASCII map: '#' = wall, anything
    /// else walkable. Rows are equal length.
    fn grid(rows: &[&str]) -> (usize, usize, Vec<Vec<bool>>) {
        let h = rows.len();
        let w = rows[0].len();
        let walk: Vec<Vec<bool>> = rows.iter()
            .map(|row| row.chars().map(|ch| ch != '#').collect())
            .collect();
        (w, h, walk)
    }

    #[test]
    fn straight_line_path_is_shortest() {
        let (w, h, g) = grid(&["......"]);
        let path = find_path((0, 0), (5, 0), w, h, |c, r| g[r][c]).unwrap();
        assert_eq!(path.len(), 5, "five steps to cross five tiles");
        assert_eq!(*path.last().unwrap(), (5, 0));
        // Each step is a single orthogonal move.
        let mut prev = (0usize, 0usize);
        for &step in &path {
            assert!(is_adjacent(prev, step));
            prev = step;
        }
    }

    #[test]
    fn start_equals_goal_is_empty_path() {
        let (w, h, g) = grid(&["..", ".."]);
        assert_eq!(find_path((1, 1), (1, 1), w, h, |c, r| g[r][c]), Some(vec![]));
    }

    #[test]
    fn routes_around_a_wall() {
        // Wall down the middle with a gap at the bottom row.
        let (w, h, g) = grid(&[
            ".#.",
            ".#.",
            "...",
        ]);
        let path = find_path((0, 0), (2, 0), w, h, |c, r| g[r][c]).unwrap();
        assert_eq!(*path.last().unwrap(), (2, 0));
        // Must detour through the open bottom row, so longer than the 2-tile gap.
        assert!(path.len() > 2, "should route around, got {path:?}");
        // Never steps on a wall.
        for &(c, r) in &path {
            assert!(g[r][c], "path stepped on a wall at {c},{r}");
        }
    }

    #[test]
    fn unreachable_goal_is_none() {
        let (w, h, g) = grid(&[
            "..#..",
            "..#..",
            "..#..",
        ]);
        assert_eq!(find_path((0, 0), (4, 0), w, h, |c, r| g[r][c]), None);
    }

    #[test]
    fn goal_on_wall_is_none() {
        let (w, h, g) = grid(&["...", ".#.", "..."]);
        assert_eq!(find_path((0, 0), (1, 1), w, h, |c, r| g[r][c]), None);
    }

    #[test]
    fn adjacent_path_stops_next_to_a_solid_target() {
        // Goal (2,0) is a wall (an NPC/object you can't stand on); we should
        // stop on a walkable neighbor.
        let (w, h, g) = grid(&[
            "..#",
            "...",
        ]);
        let path = find_path_adjacent((0, 0), (2, 0), w, h, |c, r| g[r][c]).unwrap();
        let end = *path.last().unwrap();
        assert!(is_adjacent(end, (2, 0)), "should end adjacent to the target, ended at {end:?}");
        assert!(g[end.1][end.0], "should stop on a walkable tile");
    }

    #[test]
    fn already_adjacent_needs_no_steps() {
        let (w, h, g) = grid(&["..", ".."]);
        assert_eq!(find_path_adjacent((0, 0), (1, 0), w, h, |c, r| g[r][c]), Some(vec![]));
    }
}
