//! Embodied number line — ambient stepping-stones laid into the world as
//! terrain the kid hops across. The avatar IS the token; walking the stones IS
//! counting. This module is just the *data* (which tiles form a numbered path
//! on which map, and which stone is the goal); the game drives feedback off the
//! player's index along it, and the render draws the stones.
//!
//! Spike-1: one ambient path in the reef. Gated dive-gauges (the deliberate
//! number-line challenge) come later and reuse the `number_line` domain reducer.

/// A numbered path of tiles. `tiles[i]` is the stone for mark `i` (so they must
/// be listed in order and each adjacent to the next). `target` is the index of
/// the *initial* goal stone — the game rerolls it after each find. `clam` is
/// the tile where Shelly the clam perches and calls out the goal number; her
/// pearl hides under that stone, invisible until the kid lands on it.
pub struct NumberTrack {
    pub id: &'static str,
    pub tiles: &'static [(usize, usize)],
    pub target: usize,
    pub clam: (usize, usize),
}

impl NumberTrack {
    /// Mark (index) of `tile` on this path, or `None` if it isn't a stone.
    pub fn index_of(&self, tile: (usize, usize)) -> Option<usize> {
        self.tiles.iter().position(|&t| t == tile)
    }

    /// The goal stone's tile.
    pub fn target_tile(&self) -> (usize, usize) {
        self.tiles[self.target]
    }
}

/// A vertical "dive gauge": numbered depth-stones the kid counts *down* to
/// descend. Stepping onto `tiles[target]` (the deepest, glowing stone) sits on
/// a portal to the deeper zone — so walking the gauge to the bottom IS the
/// descent. Bail = just walk off; nothing happens unless you reach the door.
pub struct DiveTrack {
    pub tiles: &'static [(usize, usize)],
    pub target: usize,
}

impl DiveTrack {
    pub fn target_tile(&self) -> (usize, usize) {
        self.tiles[self.target]
    }
}

/// The dive-gauge shaft for `map_id`, if it has one.
pub fn dive_track_for_map(map_id: &str) -> Option<DiveTrack> {
    match map_id {
        // A vertical shaft on the reef's east edge (col 36): hop DOWN the
        // depth-stones 0..5; the deepest (depth 5, (36,13)) is the trench
        // door. Inkwell the octopus loiters at the top.
        "reef" => Some(DiveTrack {
            tiles: &[(36, 8), (36, 9), (36, 10), (36, 11), (36, 12), (36, 13)],
            target: 5,
        }),
        _ => None,
    }
}

/// The ambient number path for `map_id`, if it has one.
pub fn track_for_map(map_id: &str) -> Option<NumberTrack> {
    match map_id {
        // Reef row 13 is a long clear stretch of the middle basin — stepping-
        // stones the kid hops along (marks 0..11). Shelly perches just west of
        // stone 0 and calls the goal number.
        "reef" => Some(NumberTrack {
            id: "reef_path_1",
            tiles: &[
                (5, 13), (6, 13), (7, 13), (8, 13), (9, 13), (10, 13),
                (11, 13), (12, 13), (13, 13), (14, 13), (15, 13), (16, 13),
            ],
            target: 6,
            clam: (3, 13),
        }),
        // The trench (deeper zone) has its own, richer pearl path — the payoff
        // for descending. Row 8, marks 0..11.
        "trench" => Some(NumberTrack {
            id: "trench_path_1",
            tiles: &[
                (5, 8), (6, 8), (7, 8), (8, 8), (9, 8), (10, 8),
                (11, 8), (12, 8), (13, 8), (14, 8), (15, 8), (16, 8),
            ],
            target: 7,
            clam: (3, 8),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_are_ordered_adjacent_with_clam_off_the_path() {
        for map_id in ["reef", "trench"] {
            let t = track_for_map(map_id).unwrap();
            for pair in t.tiles.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let dist = (a.0 as i32 - b.0 as i32).abs() + (a.1 as i32 - b.1 as i32).abs();
                assert_eq!(dist, 1, "{map_id}: stones {a:?} and {b:?} must be adjacent");
            }
            assert!(t.target < t.tiles.len(), "{map_id}: target index in range");
            assert!(
                t.index_of(t.clam).is_none(),
                "{map_id}: Shelly's perch must not sit on a stone (she'd block the path)",
            );
        }
    }

    #[test]
    fn index_and_target_tile_round_trip() {
        let t = track_for_map("reef").unwrap();
        assert_eq!(t.index_of((7, 13)), Some(2));
        assert_eq!(t.index_of((0, 0)), None);
        assert_eq!(t.index_of(t.target_tile()), Some(t.target));
    }

    #[test]
    fn maps_without_a_track_return_none() {
        assert!(track_for_map("overworld").is_none());
    }
}
