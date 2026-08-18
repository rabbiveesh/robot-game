//! Embodied number line — Shelly's pearl stones, laid into the world as
//! terrain the kid *leaps* across. The avatar IS the token; the leaping IS the
//! skip-counting. This module is just the *data* (which tiles are stones on
//! which map, and what a pearl there is worth); the rules live in
//! `robot_buddy_domain::logic::leap`, and the game drives both off the
//! player's stone index.
//!
//! The stones sit every other tile with a rip current in the gaps, so walking
//! the path is impossible — the only way along it is a leap of the size the
//! kid commits to before launching. That's the whole point: a pearl can't be
//! stumbled onto.

/// A chain of numbered stones. `tiles[i]` is the stone for mark `i`, in order,
/// spaced `STONE_SPACING` tiles apart with current between them. `clam` is
/// where Shelly perches and calls out the trip; `payout` is what one of her
/// pearls is worth here — the deeper path pays better, which is the reward for
/// making the descent.
pub struct NumberTrack {
    pub id: &'static str,
    pub tiles: &'static [(usize, usize)],
    pub clam: (usize, usize),
    pub payout: u32,
}

/// Tiles between consecutive stones. One gap is enough to make the path
/// unwalkable, and keeps the whole chain inside a reasonable map width.
pub const STONE_SPACING: usize = 2;

impl NumberTrack {
    /// Mark (index) of `tile` on this path, or `None` if it isn't a stone.
    pub fn index_of(&self, tile: (usize, usize)) -> Option<usize> {
        self.tiles.iter().position(|&t| t == tile)
    }

    /// The highest mark on the path — the ceiling any generated trip fits in.
    pub fn max_mark(&self) -> u8 {
        (self.tiles.len().saturating_sub(1)) as u8
    }

    /// The tiles between the stones, which the map paints as rip current.
    /// Derived from the stones themselves so the terrain can never drift out
    /// of step with the path.
    pub fn current_tiles(&self) -> Vec<(usize, usize)> {
        let mut gaps = Vec::new();
        for pair in self.tiles.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (lo, hi) = if a.0 <= b.0 { (a.0, b.0) } else { (b.0, a.0) };
            for x in (lo + 1)..hi {
                gaps.push((x, a.1));
            }
        }
        gaps
    }
}

/// Shelly's pearl path for `map_id`, if it has one.
pub fn track_for_map(map_id: &str) -> Option<NumberTrack> {
    match map_id {
        // Reef row 13 is a long clear stretch of the middle basin — thirteen
        // stones (marks 0..12) every other tile, currents between. Shelly
        // perches two tiles west of the launch stone and calls the trip.
        "reef" => Some(NumberTrack {
            id: "reef_path_1",
            tiles: &[
                (5, 13), (7, 13), (9, 13), (11, 13), (13, 13), (15, 13), (17, 13),
                (19, 13), (21, 13), (23, 13), (25, 13), (27, 13), (29, 13),
            ],
            clam: (3, 13),
            payout: 1,
        }),
        // The trench's path is shorter — the canyon walls don't leave room —
        // but a deep pearl is worth double. That's the payoff for the descent.
        "trench" => Some(NumberTrack {
            id: "trench_path_1",
            tiles: &[
                (4, 8), (6, 8), (8, 8), (10, 8), (12, 8), (14, 8), (16, 8), (18, 8),
            ],
            clam: (2, 8),
            payout: 2,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stones_are_evenly_spaced_with_the_clam_off_the_path() {
        for map_id in ["reef", "trench"] {
            let t = track_for_map(map_id).unwrap();
            for pair in t.tiles.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                assert_eq!(a.1, b.1, "{map_id}: the path runs along one row");
                assert_eq!(b.0 - a.0, STONE_SPACING,
                    "{map_id}: stones {a:?} and {b:?} must be a leap apart, not a step");
            }
            assert!(
                t.index_of(t.clam).is_none(),
                "{map_id}: Shelly's perch must not sit on a stone (she'd block the launch)",
            );
            assert!(t.max_mark() >= 4, "{map_id}: a path needs room for a real trip");
        }
    }

    #[test]
    fn the_gaps_between_stones_are_current() {
        let t = track_for_map("reef").unwrap();
        let gaps = t.current_tiles();
        assert_eq!(gaps.len(), t.tiles.len() - 1, "one gap between each pair of stones");
        assert!(gaps.contains(&(6, 13)), "the tile between stones 0 and 1: {gaps:?}");
        for g in &gaps {
            assert!(t.index_of(*g).is_none(), "a gap must never be a stone: {g:?}");
        }
    }

    #[test]
    fn index_and_max_round_trip() {
        let t = track_for_map("reef").unwrap();
        assert_eq!(t.index_of((9, 13)), Some(2));
        assert_eq!(t.index_of((10, 13)), None, "the current between stones isn't a mark");
        assert_eq!(t.max_mark() as usize, t.tiles.len() - 1);
    }

    #[test]
    fn the_deep_path_pays_better() {
        assert!(
            track_for_map("trench").unwrap().payout > track_for_map("reef").unwrap().payout,
            "diving to the trench should be worth the trip",
        );
    }

    #[test]
    fn maps_without_a_track_return_none() {
        assert!(track_for_map("overworld").is_none());
    }
}
