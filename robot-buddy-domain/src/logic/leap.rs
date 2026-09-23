//! Shelly's pearl leaps — skip-counting and partitioning, done with the body.
//! Pure logic, no rendering.
//!
//! A chain of numbered stones with a current running between them: you can't
//! walk it, you leap it, and every leap in a trip is the SAME size. Shelly
//! hides her pearl under stone `size × count` and gives one clue — the size
//! ("leap by 3s") for kids who are skip-counting, the count ("four leaps") for
//! kids ready to work backwards to the size. Choosing the size is the whole
//! puzzle; picking wrong sails you past the pearl, and the swim back to stone
//! zero is the cost of not thinking first.
//!
//! Exactly one of the offered sizes divides the pearl's stone evenly, so a
//! wrong pick always overshoots and a right pick always lands. There is no
//! failure state and nothing to lose — overshooting just means swimming back.
//!
//! Public surface mirrors the other logic modules:
//!   - `generate_leap(band, max, &mut impl Rng)` → a puzzle that fits the path
//!   - `LeapSession::new(puzzle)` → fresh session
//!   - `leap_reducer(session, action)` → new session

use rand::Rng;
use serde::{Deserialize, Serialize};

/// The one thing Shelly tells you about the trip. The other half is yours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Clue {
    /// "Leap by 3s!" — the kid skip-counts and has to stop in the right place.
    Size { n: u8 },
    /// "Four leaps!" — the kid works out how big each one has to be.
    Count { n: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeapPuzzle {
    /// Highest-numbered stone on the path.
    pub max: u8,
    /// The stone the pearl hides under. Always `size * count`.
    pub pearl: u8,
    /// The leap size that lands on it.
    pub size: u8,
    /// How many leaps of `size` that takes.
    pub count: u8,
    /// Sizes on offer. Exactly one — `size` — divides `pearl` evenly.
    pub choices: Vec<u8>,
    pub clue: Clue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeapPhase {
    /// On stone zero, sizing up the trip.
    Choosing,
    /// Locked into a leap size and on the way.
    Leaping,
    /// Sailed past the pearl. Swim back and pick again.
    Overshot,
    /// Landed on the pearl.
    Found,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeapSession {
    pub puzzle: LeapPuzzle,
    pub position: u8,
    /// The locked-in leap size. `None` until the kid commits.
    pub chosen: Option<u8>,
    pub leaps: u8,
    /// How many times they've swum back to try a different size. The silent
    /// read on whether the size was reasoned out or found by trial.
    pub resets: u8,
    pub phase: LeapPhase,
}

impl LeapSession {
    pub fn new(puzzle: LeapPuzzle) -> Self {
        LeapSession {
            position: 0,
            chosen: None,
            leaps: 0,
            resets: 0,
            phase: LeapPhase::Choosing,
            puzzle,
        }
    }

    /// Found the pearl on the first size they picked.
    pub fn was_clean(&self) -> bool {
        self.phase == LeapPhase::Found && self.resets == 0
    }

    /// Stone the next leap would land on. `None` before a size is locked in,
    /// and `None` when the next leap would sail clean off the end of the path
    /// — there's no stone to point at, and pointing at the last one would
    /// suggest a landing that can't happen.
    pub fn next_stone(&self) -> Option<u8> {
        let size = self.chosen?;
        let to = self.position as u16 + size as u16;
        (to <= self.puzzle.max as u16).then_some(to as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LeapAction {
    /// Commit to a leap size. Only possible back on stone zero.
    Choose { size: u8 },
    /// Launch — one leap of the chosen size.
    Leap,
    /// Swim back to stone zero and pick a different size. Always available,
    /// always free.
    SwimBack,
}

pub fn leap_reducer(state: LeapSession, action: LeapAction) -> LeapSession {
    let mut next = state;
    if next.phase == LeapPhase::Found {
        return next;
    }

    match action {
        LeapAction::SwimBack => {
            // Only a trip that actually left the launch stone counts as a
            // retry — changing your mind before you jump is free.
            if next.position > 0 {
                next.resets = next.resets.saturating_add(1);
            }
            next.position = 0;
            next.chosen = None;
            next.leaps = 0;
            next.phase = LeapPhase::Choosing;
        }
        LeapAction::Choose { size } => {
            // Committing is the point: you can only re-size from stone zero.
            if next.phase != LeapPhase::Choosing || !next.puzzle.choices.contains(&size) {
                return next;
            }
            next.chosen = Some(size);
            next.phase = LeapPhase::Leaping;
        }
        LeapAction::Leap => {
            let Some(size) = next.chosen else { return next };
            if next.phase != LeapPhase::Leaping {
                return next;
            }
            // Where the leap really goes, before any clamping. A leap past
            // the end of the path parks you on the last stone for display,
            // but it went PAST the pearl — it can never count as a landing,
            // even when the pearl sits on that last stone.
            let to = next.position as u16 + size as u16;
            next.position = to.min(next.puzzle.max as u16) as u8;
            next.leaps = next.leaps.saturating_add(1);
            if to == next.puzzle.pearl as u16 {
                next.phase = LeapPhase::Found;
            } else if to > next.puzzle.pearl as u16 {
                next.phase = LeapPhase::Overshot;
            }
        }
    }
    next
}

/// Leap sizes and trip lengths by band. Small sizes and short trips are skip-
/// counting; longer trips with a stated count are partitioning (division).
fn leap_shape(band: u8) -> (Vec<u8>, u8, u8) {
    // (sizes to draw from, min count, max count)
    match band {
        0 | 1 => (vec![2, 3], 2, 3),
        2 => (vec![2, 3, 4, 5], 2, 4),
        3 => (vec![2, 3, 4, 5, 6], 3, 5),
        _ => (vec![3, 4, 5, 6, 7, 8], 3, 6),
    }
}

/// Build a trip that fits a path of `max` stones. The pearl always sits on a
/// multiple of the true size, and every decoy size is one that would sail past
/// it — so a wrong pick is always visibly wrong, and a right pick always lands.
pub fn generate_leap(band: u8, max: u8, rng: &mut impl Rng) -> LeapPuzzle {
    let (pool, min_count, max_count) = leap_shape(band);

    // The pearl stays at least one stone short of the end, so a wrong size
    // always visibly lands on a stone PAST it (or sails off the end) instead
    // of parking on the pearl's own stone.
    let room = max.saturating_sub(1).max(1);

    // Only sizes that can make at least `min_count` leaps inside that room.
    let usable: Vec<u8> = pool.iter().copied()
        .filter(|s| (*s as u16) * (min_count as u16) <= room as u16)
        .collect();
    let size = if usable.is_empty() {
        // A path too short for this band's leaps still gets a real trip.
        2.min(room)
    } else {
        usable[rng.gen_range(0..usable.len())]
    };

    let ceiling = (room / size.max(1)).max(1);
    let hi = max_count.min(ceiling);
    let lo = min_count.min(hi);
    let count = if hi > lo { rng.gen_range(lo..=hi) } else { lo };
    let pearl = size.saturating_mul(count).min(room);

    // Decoys: sizes that DON'T divide the pearl, so choosing one overshoots.
    let mut decoys: Vec<u8> = (2..=9u8)
        .filter(|d| *d != size && pearl % *d != 0 && (*d as u16) <= max as u16)
        .collect();
    let mut choices = vec![size];
    let wanted = if band <= 1 { 2 } else { 3 };
    while choices.len() < wanted && !decoys.is_empty() {
        let i = rng.gen_range(0..decoys.len());
        choices.push(decoys.remove(i));
    }
    choices.sort_unstable();

    // Younger kids are told the size and skip-count it out; older ones are told
    // how many leaps and have to work the size out for themselves.
    let clue = if band <= 2 { Clue::Size { n: size } } else { Clue::Count { n: count } };

    LeapPuzzle { max, pearl, size, count, choices, clue }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    /// Pearl under stone 12: leap by 3 four times. 5 and 7 sail past it.
    fn trip() -> LeapPuzzle {
        LeapPuzzle {
            max: 20,
            pearl: 12,
            size: 3,
            count: 4,
            choices: vec![3, 5, 7],
            clue: Clue::Count { n: 4 },
        }
    }

    fn play(mut s: LeapSession, actions: &[LeapAction]) -> LeapSession {
        for a in actions {
            s = leap_reducer(s, *a);
        }
        s
    }

    #[test]
    fn the_right_size_lands_on_the_pearl() {
        let s = play(LeapSession::new(trip()), &[
            LeapAction::Choose { size: 3 },
            LeapAction::Leap, LeapAction::Leap, LeapAction::Leap, LeapAction::Leap,
        ]);
        assert_eq!(s.position, 12);
        assert_eq!(s.leaps, 4);
        assert_eq!(s.phase, LeapPhase::Found);
        assert!(s.was_clean(), "first size picked, straight to the pearl");
    }

    #[test]
    fn a_wrong_size_sails_past_the_pearl() {
        let s = play(LeapSession::new(trip()), &[
            LeapAction::Choose { size: 5 },
            LeapAction::Leap, LeapAction::Leap, LeapAction::Leap,
        ]);
        assert_eq!(s.position, 15, "5, 10, 15 — straight over the top of it");
        assert_eq!(s.phase, LeapPhase::Overshot);
        assert!(!s.was_clean());
    }

    #[test]
    fn leaping_stops_once_you_have_sailed_past() {
        let s = play(LeapSession::new(trip()), &[
            LeapAction::Choose { size: 7 },
            LeapAction::Leap, LeapAction::Leap, LeapAction::Leap, LeapAction::Leap,
        ]);
        assert_eq!(s.position, 14, "overshooting parks you; more leaps do nothing");
        assert_eq!(s.leaps, 2);
    }

    #[test]
    fn swimming_back_is_free_and_lets_you_pick_again() {
        let s = play(LeapSession::new(trip()), &[
            LeapAction::Choose { size: 5 },
            LeapAction::Leap, LeapAction::Leap, LeapAction::Leap,
            LeapAction::SwimBack,
            LeapAction::Choose { size: 3 },
            LeapAction::Leap, LeapAction::Leap, LeapAction::Leap, LeapAction::Leap,
        ]);
        assert_eq!(s.phase, LeapPhase::Found, "a second try still finds the pearl");
        assert_eq!(s.resets, 1);
        assert!(!s.was_clean(), "...but it wasn't reasoned out first time");
    }

    #[test]
    fn the_size_is_locked_in_once_you_launch() {
        let s = play(LeapSession::new(trip()), &[
            LeapAction::Choose { size: 5 },
            LeapAction::Leap,
            LeapAction::Choose { size: 3 }, // too late — you're mid-trip
            LeapAction::Leap,
        ]);
        assert_eq!(s.chosen, Some(5));
        assert_eq!(s.position, 10, "still leaping by fives");
    }

    #[test]
    fn changing_your_mind_on_the_launch_stone_is_not_a_retry() {
        let s = play(LeapSession::new(trip()), &[
            LeapAction::Choose { size: 5 },
            LeapAction::SwimBack,
            LeapAction::Choose { size: 3 },
            LeapAction::Leap, LeapAction::Leap, LeapAction::Leap, LeapAction::Leap,
        ]);
        assert_eq!(s.resets, 0, "thinking better of it before launching costs nothing");
        assert!(s.was_clean());
    }

    #[test]
    fn a_size_that_is_not_on_offer_does_nothing() {
        let s = play(LeapSession::new(trip()), &[LeapAction::Choose { size: 4 }]);
        assert_eq!(s.chosen, None);
        assert_eq!(s.phase, LeapPhase::Choosing);
    }

    #[test]
    fn you_cannot_leap_before_committing_to_a_size() {
        let s = play(LeapSession::new(trip()), &[LeapAction::Leap, LeapAction::Leap]);
        assert_eq!(s.position, 0);
        assert_eq!(s.leaps, 0);
    }

    #[test]
    fn exactly_one_offered_size_ever_lands_on_the_pearl() {
        for band in 1..=5u8 {
            for max in [8u8, 12, 20] {
                for seed in 0..30u64 {
                    let mut rng = SmallRng::seed_from_u64(seed);
                    let p = generate_leap(band, max, &mut rng);
                    assert!(p.pearl >= 1 && p.pearl <= p.max,
                        "band {band} max {max} seed {seed}: pearl off the path: {p:?}");
                    assert_eq!(p.size as u16 * p.count as u16, p.pearl as u16,
                        "band {band} max {max} seed {seed}: pearl must be size x count: {p:?}");
                    assert!(p.choices.contains(&p.size), "the answer has to be on offer: {p:?}");
                    let landers: Vec<u8> = p.choices.iter().copied()
                        .filter(|c| p.pearl % *c == 0)
                        .collect();
                    assert_eq!(landers, vec![p.size],
                        "band {band} max {max} seed {seed}: only one size may land: {p:?}");
                }
            }
        }
    }

    #[test]
    fn every_generated_trip_is_winnable_by_its_own_clue() {
        for band in 1..=5u8 {
            for seed in 0..30u64 {
                let mut rng = SmallRng::seed_from_u64(seed);
                let p = generate_leap(band, 20, &mut rng);
                let count = p.count;
                let mut s = LeapSession::new(p.clone());
                s = leap_reducer(s, LeapAction::Choose { size: p.size });
                for _ in 0..count {
                    s = leap_reducer(s, LeapAction::Leap);
                }
                assert_eq!(s.phase, LeapPhase::Found,
                    "band {band} seed {seed}: {count} leaps of {} should land: {p:?}", p.size);
            }
        }
    }

    /// Leap one size out until the trip stops moving; returns the end state.
    fn leap_it_out(p: &LeapPuzzle, size: u8) -> LeapSession {
        let mut s = leap_reducer(LeapSession::new(p.clone()), LeapAction::Choose { size });
        for _ in 0..=p.max {
            s = leap_reducer(s, LeapAction::Leap);
            if s.phase != LeapPhase::Leaping {
                break;
            }
        }
        s
    }

    #[test]
    fn every_decoy_overshoots_and_only_the_true_size_finds_the_pearl() {
        for band in 0..=10u8 {
            for max in [7u8, 12] {
                for seed in 0..500u64 {
                    let mut rng = SmallRng::seed_from_u64(seed);
                    let p = generate_leap(band, max, &mut rng);
                    for &c in &p.choices {
                        let s = leap_it_out(&p, c);
                        let want = if c == p.size { LeapPhase::Found } else { LeapPhase::Overshot };
                        assert_eq!(s.phase, want,
                            "band {band} max {max} seed {seed} size {c}: {p:?} -> {s:?}");
                        assert!(s.position <= p.max, "position stays on the path: {s:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn sailing_off_the_end_is_never_a_landing_even_on_a_last_stone_pearl() {
        // Hand-built: pearl on the very last stone. 5, 10, then 15 is off the
        // end — it parks on stone 12, but it went PAST the pearl.
        let p = LeapPuzzle {
            max: 12, pearl: 12, size: 4, count: 3, choices: vec![4, 5], clue: Clue::Size { n: 4 },
        };
        let s = leap_it_out(&p, 5);
        assert_eq!(s.phase, LeapPhase::Overshot);
        assert!(!s.was_clean());
        assert_eq!(leap_it_out(&p, 4).phase, LeapPhase::Found);
    }

    #[test]
    fn next_stone_is_none_when_the_next_leap_leaves_the_path() {
        let p = LeapPuzzle {
            max: 12, pearl: 8, size: 4, count: 2, choices: vec![4, 5], clue: Clue::Size { n: 4 },
        };
        let mut s = leap_reducer(LeapSession::new(p), LeapAction::Choose { size: 5 });
        assert_eq!(s.next_stone(), Some(5));
        s = leap_reducer(s, LeapAction::Leap);
        s = leap_reducer(s, LeapAction::Leap);
        assert_eq!(s.position, 10);
        assert_eq!(s.next_stone(), None, "15 is off the end — nothing to point at");
    }

    #[test]
    fn the_pearl_is_never_on_the_last_stone() {
        for band in 0..=10u8 {
            for max in [6u8, 7, 12, 20] {
                for seed in 0..100u64 {
                    let mut rng = SmallRng::seed_from_u64(seed);
                    let p = generate_leap(band, max, &mut rng);
                    assert!(p.pearl < p.max, "band {band} max {max} seed {seed}: {p:?}");
                }
            }
        }
    }

    #[test]
    fn younger_kids_are_told_the_size_and_older_ones_the_count() {
        let mut rng = SmallRng::seed_from_u64(3);
        assert!(matches!(generate_leap(1, 20, &mut rng).clue, Clue::Size { .. }));
        assert!(matches!(generate_leap(5, 20, &mut rng).clue, Clue::Count { .. }));
    }

    #[test]
    fn a_short_path_still_gets_a_real_trip() {
        for seed in 0..20u64 {
            let mut rng = SmallRng::seed_from_u64(seed);
            let p = generate_leap(5, 6, &mut rng);
            assert!(p.pearl >= 1 && p.pearl <= 6, "{p:?}");
            assert!(p.count >= 1, "{p:?}");
        }
    }
}
