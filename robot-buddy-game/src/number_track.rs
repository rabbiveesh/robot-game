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
/// the goal stone — hop there and the path gives a little cheer.
pub struct NumberTrack {
    pub id: &'static str,
    pub tiles: &'static [(usize, usize)],
    pub target: usize,
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

/// The ambient number path for `map_id`, if it has one.
pub fn track_for_map(map_id: &str) -> Option<NumberTrack> {
    match map_id {
        // Reef row 11 is a clear stretch of the lower basin — stepping-stones
        // the kid hops along (marks 0..9). Hop to 6 for the goal.
        "reef" => Some(NumberTrack {
            id: "reef_path_1",
            tiles: &[
                (4, 11), (5, 11), (6, 11), (7, 11), (8, 11),
                (9, 11), (10, 11), (11, 11), (12, 11), (13, 11),
            ],
            target: 6,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reef_track_is_ordered_and_adjacent() {
        let t = track_for_map("reef").unwrap();
        for pair in t.tiles.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let dist = (a.0 as i32 - b.0 as i32).abs() + (a.1 as i32 - b.1 as i32).abs();
            assert_eq!(dist, 1, "stones {a:?} and {b:?} must be adjacent");
        }
        assert!(t.target < t.tiles.len(), "target index in range");
    }

    #[test]
    fn index_and_target_tile_round_trip() {
        let t = track_for_map("reef").unwrap();
        assert_eq!(t.index_of((6, 11)), Some(2));
        assert_eq!(t.index_of((0, 0)), None);
        assert_eq!(t.index_of(t.target_tile()), Some(t.target));
    }

    #[test]
    fn maps_without_a_track_return_none() {
        assert!(track_for_map("overworld").is_none());
    }
}
