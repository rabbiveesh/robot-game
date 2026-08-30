//! The Goyish Map — a number-bond space shooter. Pure logic, no rendering.
//!
//! A row of numbered aliens drifts slowly down toward the ship. A target is
//! shown ("MAKE 10"). The kid slides the ship left/right and fires a bolt that
//! travels visibly up its column and tags the first alien it reaches — so you
//! can see exactly where you're lined up. Tag two aliens whose numbers *sum to
//! the target* and both pop — number bonds (part-part-whole) ARE the aiming
//! logic, so this passes the Broccoli Test. A wrong pair simply deselects
//! (never a "WRONG", never punishment).
//!
//! Mild stakes, no clock (Invariant 4): an alien that drifts off the bottom
//! dims the shield one notch and retreats; clearing a pair refills it. When the
//! shield empties the aliens *hover* (drift freezes) so a stuck kid always
//! recovers — there is no game-over and no timer anywhere.
//!
//! Every wave is built purely from pairs that sum to the target, so it is
//! always fully clearable: for any valid clear `a+c=T`, the leftover partners
//! `b=T-a` and `d=T-c` still satisfy `b+d = 2T-(a+c) = T`. No wave can strand an
//! unpairable alien (the anti-freeze rule from ADR-003).
//!
//! Difficulty is silent (Invariant 6): the math band sets the bond-total range
//! (mirroring the core `challenge_generator`'s NumberBond bands), and the
//! learner's NumberBond CRA stage sets how the numbers are drawn — pips
//! (Concrete), grouped dots (Representational), or numerals (Abstract). Repeated
//! mis-pairs scaffold that representation one step more concrete, mid-run.
//!
//! Public surface mirrors the other logic modules:
//!   - `ShooterSession::new(band, cra_stage, pace, &mut impl Rng)` → fresh session (all
//!     waves pre-generated up front, so the reducer itself needs no RNG)
//!   - `shooter_reducer(session, action)` → new session

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::types::{CraStage, GamePace};

/// Logical play-field extent. Positions are in these units, independent of
/// screen pixels — the renderer maps them to whatever the window is.
pub const FIELD_W: f32 = 100.0;
pub const FIELD_H: f32 = 100.0;
/// Where a fresh wave of aliens starts (near the top).
pub const SPAWN_Y: f32 = 8.0;
/// An alien at or past this depth has breached the shield and retreats.
pub const BREACH_Y: f32 = 92.0;
/// A bolt tags an alien whose column is within this of the bolt. Kept well
/// under half the alien spacing (~15) so hit-zones never overlap — a bolt fired
/// between two aliens misses both, which reads clearly and rewards lining up.
pub const HIT_TOLERANCE: f32 = 6.0;
/// Bolt travel speed, logical units/sec (upward). Fast enough to feel snappy,
/// slow enough that the kid watches it cross the gap.
pub const SHOT_SPEED: f32 = 150.0;
/// Where a fired bolt starts — just above the ship on the bottom rail.
pub const SHOT_SPAWN_Y: f32 = 96.0;
/// Shield capacity (also the starting value).
pub const MAX_SHIELD: u8 = 3;
/// Waves per run.
pub const TOTAL_WAVES: usize = 3;
/// Consecutive mis-pairs (no correct one between) before the representation
/// scaffolds one step more concrete. Kept high so exploration isn't punished.
pub const MISS_STREAK_TO_SCAFFOLD: u8 = 3;
/// Largest value drawn as countable dots. Above this, counting pips is *harder*
/// than reading the numeral, so the renderer always shows the numeral instead —
/// regardless of CRA stage.
pub const DOT_MAX: u32 = 10;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alien {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub value: u32,
    pub selected: bool,
}

/// A bolt in flight: fixed column `x`, `y` rising toward 0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shot {
    pub x: f32,
    pub y: f32,
}

/// One wave's worth of aliens, expressed as the target and the values on the
/// aliens (already paired so it's fully clearable). Positions are assigned when
/// the wave is spawned.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wave {
    pub target: u32,
    pub values: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShooterPhase {
    Playing,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShooterSession {
    pub ship_x: f32,
    pub target: u32,
    pub aliens: Vec<Alien>,
    /// Bolts currently in flight.
    pub shots: Vec<Shot>,
    pub shield: u8,
    pub max_shield: u8,
    /// Pairs cleared across the whole run — the "score".
    pub score: u32,
    /// Index into `waves` of the wave now on screen.
    pub wave: usize,
    pub waves: Vec<Wave>,
    /// Correct pairings (stealth-assessment signal).
    pub hits: u32,
    /// Mismatched pairings — never punished, just counted.
    pub misses: u32,
    /// Consecutive mis-pairs since the last correct one. Resets on any hit;
    /// drives the gentle in-run representation scaffold.
    pub miss_streak: u8,
    pub drift_speed: f32,
    /// How the numbers are shown — the learner's `NumberBond` CRA stage, clamped
    /// up to `min_representation`, and scaffolded *down* (toward manipulatives,
    /// but never below the floor) during a run if the kid keeps mis-pairing. The
    /// renderer draws pips / grouped dots / numerals accordingly; the child never
    /// sees the label (Invariant 6).
    pub representation: CraStage,
    /// Most-concrete representation this band allows — dots stop helping once the
    /// numbers get big, so a high band floors this above Concrete.
    pub min_representation: CraStage,
    pub phase: ShooterPhase,
    next_id: u32,
}

impl ShooterSession {
    /// Build a run scaled *silently* to the learner (Invariant 6 — no difficulty
    /// shown to the kid). `band` sets the bond-total range, alien count and drift;
    /// `cra_stage` is the learner's `NumberBond` CRA stage, which picks how the
    /// numbers are drawn. Each wave rolls its own target within the band's range,
    /// so a run mixes (e.g.) "make 12" then "make 9". All waves are generated here
    /// so the reducer stays RNG-free.
    pub fn new(band: u8, cra_stage: CraStage, pace: GamePace, rng: &mut impl Rng) -> Self {
        let (lo, hi) = bond_target_range(band);
        let count = alien_count(band);
        let drift_speed = drift_speed(band, pace);
        // Clamp the learner's stage up to what this band's numbers can show as
        // dots — no Concrete when the bonds are big (per the band floor).
        let min_representation = representation_floor(band);
        let representation = if cra_stage.order() >= min_representation.order() {
            cra_stage
        } else {
            min_representation
        };
        let waves: Vec<Wave> = (0..TOTAL_WAVES)
            .map(|_| {
                let target = rng.gen_range(lo..=hi);
                generate_wave(target, count, rng)
            })
            .collect();

        let mut session = ShooterSession {
            ship_x: FIELD_W / 2.0,
            target: waves[0].target,
            aliens: Vec::new(),
            shots: Vec::new(),
            shield: MAX_SHIELD,
            max_shield: MAX_SHIELD,
            score: 0,
            wave: 0,
            waves,
            hits: 0,
            misses: 0,
            miss_streak: 0,
            drift_speed,
            representation,
            min_representation,
            phase: ShooterPhase::Playing,
            next_id: 0,
        };
        session.spawn_current_wave();
        session
    }

    /// Lay the current wave's aliens out in a row across the field at the top.
    fn spawn_current_wave(&mut self) {
        self.shots.clear();
        let wave = &self.waves[self.wave];
        self.target = wave.target;
        let n = wave.values.len().max(1);
        let margin = 12.0;
        let span = FIELD_W - 2.0 * margin;
        let mut aliens = Vec::with_capacity(wave.values.len());
        for (i, &value) in wave.values.iter().enumerate() {
            let x = if n == 1 {
                FIELD_W / 2.0
            } else {
                margin + span * (i as f32) / (n as f32 - 1.0)
            };
            aliens.push(Alien { id: self.next_id, x, y: SPAWN_Y, value, selected: false });
            self.next_id += 1;
        }
        self.aliens = aliens;
    }

    fn selected_count(&self) -> usize {
        self.aliens.iter().filter(|a| a.selected).count()
    }
}

/// Inclusive bond-total range for a math band, mirroring how the core
/// `challenge_generator` scopes `NumberBond`: band 3 → 5..=14, band 4 → 10..=19.
/// Bands 1–2 (which the core keeps to add-to-5/10) get gentle intro bonds; the
/// higher bands extend the range so a strong kid still gets a real challenge.
fn bond_target_range(band: u8) -> (u32, u32) {
    match band {
        0..=2 => (5, 8),
        3 => (5, 14),
        4 => (10, 19),
        5..=6 => (12, 20),
        7..=8 => (15, 22),
        _ => (18, 25),
    }
}

/// Aliens per wave — even so every wave pairs off exactly. Fewer for the
/// youngest so the board never feels crowded.
fn alien_count(band: u8) -> usize {
    if band <= 2 { 4 } else { 6 }
}

/// The most-concrete representation that still reads well at a band's number
/// sizes. Once bonds routinely pass `DOT_MAX`, pure dots stop helping — so a
/// high band forbids Concrete (and eventually dots entirely). The learner's own
/// stage can only sit *at or above* this floor.
fn representation_floor(band: u8) -> CraStage {
    let (_, hi) = bond_target_range(band);
    if hi <= DOT_MAX {
        CraStage::Concrete          // bands 1–2: small bonds, dots are ideal
    } else if hi <= 16 {
        CraStage::Representational  // band 3: numeral always present, dots help small values
    } else {
        CraStage::Abstract         // band 4+: numbers too big for dots — numerals only
    }
}

/// Drift speed, logical units/sec. With FIELD_H≈100 a full descent takes
/// ~11–14s at `GamePace::Steady` — a slow drift, never a countdown. `pace` is
/// the parent's dial for kids who can do the maths but not at that speed.
fn drift_speed(band: u8, pace: GamePace) -> f32 {
    (6.5 + band as f32 * 0.4) * pace.drift_multiplier()
}

/// Build one fully-clearable wave: `count/2` pairs that each sum to `target`,
/// values in `1..target`, shuffled. `count` is rounded up to even.
pub fn generate_wave(target: u32, count: usize, rng: &mut impl Rng) -> Wave {
    let pairs = count.max(2).div_ceil(2);
    let mut values = Vec::with_capacity(pairs * 2);
    for _ in 0..pairs {
        // target ≥ 2, so 1..target is non-empty and both halves are ≥ 1.
        let a = rng.gen_range(1..target);
        values.push(a);
        values.push(target - a);
    }
    values.shuffle(rng);
    Wave { target, values }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ShooterAction {
    /// Advance the world by `dt` seconds (alien drift + breaches + bolt flight
    /// and collisions).
    Tick { dt: f32 },
    /// Slide the ship horizontally (clamped to the field).
    MoveShip { dx: f32 },
    /// Fire a bolt up the ship's column. It travels on each `Tick` and tags the
    /// first alien it reaches.
    Fire,
}

/// Resolve a completed selection: when two aliens are tagged, pop them if their
/// values sum to the target, otherwise release both (gentle, never punished).
fn resolve_selection(s: &mut ShooterSession) {
    if s.selected_count() != 2 {
        return;
    }
    let sum: u32 = s.aliens.iter().filter(|a| a.selected).map(|a| a.value).sum();
    if sum == s.target {
        s.aliens.retain(|a| !a.selected);
        s.score += 1;
        s.hits += 1;
        s.miss_streak = 0; // a correct pair clears the struggle streak
        s.shield = (s.shield + 1).min(s.max_shield);
    } else {
        for a in &mut s.aliens {
            a.selected = false;
        }
        s.misses += 1;
        s.miss_streak = s.miss_streak.saturating_add(1);
        // Gentle in-run scaffolding (fail gracefully, Invariant 7): only after a
        // real struggle streak — three mis-pairs in a row with no correct one
        // between — drop the representation one step toward manipulatives, but
        // never below the band's floor. A stray wrong shot never collapses the
        // numbers to dots, and a high band never drops to Concrete.
        if s.miss_streak >= MISS_STREAK_TO_SCAFFOLD
            && s.representation.order() > s.min_representation.order()
        {
            s.representation = s.representation.prev();
            s.miss_streak = 0;
        }
    }
}

pub fn shooter_reducer(state: ShooterSession, action: ShooterAction) -> ShooterSession {
    if state.phase == ShooterPhase::Complete {
        return state;
    }
    let mut next = state;
    match action {
        ShooterAction::MoveShip { dx } => {
            next.ship_x = (next.ship_x + dx).clamp(0.0, FIELD_W);
        }
        ShooterAction::Tick { dt } => {
            // Shield-empty freezes the drift so a stuck kid can always recover.
            if next.shield > 0 && dt > 0.0 {
                for a in &mut next.aliens {
                    a.y += next.drift_speed * dt;
                }
                // Any alien past the line breaches: it retreats and dims the
                // shield one notch.
                let before = next.aliens.len();
                next.aliens.retain(|a| a.y < BREACH_Y);
                let breached = before - next.aliens.len();
                for _ in 0..breached {
                    next.shield = next.shield.saturating_sub(1);
                }
            }

            // Bolts always fly (even at zero shield, so the kid can shoot their
            // way back). Each rises; the first alien it reaches gets tagged.
            for shot in &mut next.shots {
                shot.y -= SHOT_SPEED * dt;
            }
            let mut i = 0;
            while i < next.shots.len() {
                let shot = next.shots[i];
                // The un-selected alien the bolt has risen to (a.y >= shot.y),
                // preferring the lowest (largest y = nearest the ship = hit
                // first) and then the nearest column. Tolerance is tight enough
                // that at most one alien qualifies per column.
                let hit = next
                    .aliens
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| !a.selected && (a.x - shot.x).abs() <= HIT_TOLERANCE && a.y >= shot.y)
                    .min_by(|(_, a), (_, b)| {
                        b.y.partial_cmp(&a.y).unwrap()
                            .then((a.x - shot.x).abs().partial_cmp(&(b.x - shot.x).abs()).unwrap())
                    })
                    .map(|(idx, _)| idx);
                if let Some(idx) = hit {
                    next.aliens[idx].selected = true;
                    next.shots.remove(i);
                    resolve_selection(&mut next);
                } else if shot.y <= 0.0 {
                    next.shots.remove(i); // sailed off the top, a clean miss
                } else {
                    i += 1;
                }
            }
        }
        ShooterAction::Fire => {
            next.shots.push(Shot { x: next.ship_x, y: SHOT_SPAWN_Y });
        }
    }

    // Wave cleared (by pairing or by the last aliens breaching) → advance, or
    // finish the run if that was the final wave.
    if next.phase == ShooterPhase::Playing && next.aliens.is_empty() {
        next.wave += 1;
        if next.wave < next.waves.len() {
            next.shield = next.max_shield;
            next.spawn_current_wave();
        } else {
            next.phase = ShooterPhase::Complete;
        }
    }

    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn rng() -> SmallRng {
        SmallRng::seed_from_u64(42)
    }

    /// Move the ship under alien `id`, fire, and tick until the bolt is
    /// consumed — whether it tagged the alien, completed a pair, or sailed off.
    fn shoot(mut s: ShooterSession, id: u32) -> ShooterSession {
        let x = s.aliens.iter().find(|a| a.id == id).unwrap().x;
        s.ship_x = x;
        s = shooter_reducer(s, ShooterAction::Fire);
        for _ in 0..500 {
            if s.shots.is_empty() {
                break;
            }
            s = shooter_reducer(s, ShooterAction::Tick { dt: 1.0 / 60.0 });
        }
        s
    }

    /// Find any two on-screen aliens whose values sum to the target.
    fn a_matching_pair(s: &ShooterSession) -> (u32, u32) {
        for i in 0..s.aliens.len() {
            for j in (i + 1)..s.aliens.len() {
                if s.aliens[i].value + s.aliens[j].value == s.target {
                    return (s.aliens[i].id, s.aliens[j].id);
                }
            }
        }
        panic!("wave had no summing pair — anti-freeze invariant violated");
    }

    #[test]
    fn a_relaxed_pace_gives_a_kid_roughly_twice_as_long_to_think() {
        let steady = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        let relaxed = ShooterSession::new(3, CraStage::Abstract, GamePace::Relaxed, &mut rng());
        let brisk = ShooterSession::new(3, CraStage::Abstract, GamePace::Brisk, &mut rng());
        assert!(relaxed.drift_speed < steady.drift_speed);
        assert!(brisk.drift_speed > steady.drift_speed);
        // The dial only changes the clock, never the maths.
        assert_eq!(relaxed.aliens.len(), steady.aliens.len());
        assert_eq!(relaxed.target, steady.target,
            "same seed, same band — pace must not touch the number bonds");
        let ratio = steady.drift_speed / relaxed.drift_speed;
        assert!((1.7..2.1).contains(&ratio), "expected ~2x longer, got {ratio}x");
    }

    #[test]
    fn generated_wave_is_all_pairs_summing_to_target() {
        for band in 1..=10u8 {
            let s = ShooterSession::new(band, CraStage::Abstract, GamePace::Steady, &mut rng());
            let sum: u32 = s.aliens.iter().map(|a| a.value).sum();
            assert_eq!(sum % s.target, 0, "band {band}: values must group into target-sums");
            // Every alien is pairable — the anti-freeze property.
            a_matching_pair(&s);
        }
    }

    #[test]
    fn targets_track_the_band_range() {
        // Bond totals sit in the band's range (mirrors challenge_generator):
        // band 4 → 10..=19, the youngest → 5..=8.
        let mut r = rng();
        for _ in 0..12 {
            let s4 = ShooterSession::new(4, CraStage::Abstract, GamePace::Steady, &mut r);
            for w in &s4.waves {
                assert!((10..=19).contains(&w.target), "band 4 target {} out of range", w.target);
            }
            let s1 = ShooterSession::new(1, CraStage::Abstract, GamePace::Steady, &mut r);
            for w in &s1.waves {
                assert!((5..=8).contains(&w.target), "band 1 target {} out of range", w.target);
            }
        }
    }

    #[test]
    fn low_band_respects_the_learner_stage() {
        // Band 1 bonds are tiny (≤8), so a concrete kid gets concrete numbers.
        let s = ShooterSession::new(1, CraStage::Concrete, GamePace::Steady, &mut rng());
        assert_eq!(s.representation, CraStage::Concrete);
        assert_eq!(s.min_representation, CraStage::Concrete);
    }

    #[test]
    fn mid_band_floors_concrete_to_representational() {
        // Band 3 bonds reach 14 — too big for pure dots, so Concrete is bumped
        // up to Representational (numerals always present).
        let s = ShooterSession::new(3, CraStage::Concrete, GamePace::Steady, &mut rng());
        assert_eq!(s.representation, CraStage::Representational);
    }

    #[test]
    fn high_band_never_uses_concrete() {
        // Band 5 bonds reach 20 — dots don't help; even a "concrete" learner
        // sees numerals.
        let s = ShooterSession::new(5, CraStage::Concrete, GamePace::Steady, &mut rng());
        assert_eq!(s.representation, CraStage::Abstract);
        assert_eq!(s.min_representation, CraStage::Abstract);
    }

    #[test]
    fn scaffold_never_drops_below_the_band_floor() {
        // Band 3 floors at Representational; a long miss streak can't reach
        // Concrete.
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        s.target = 10;
        for _ in 0..12 {
            force_miss(&mut s);
        }
        assert_eq!(s.representation, CraStage::Representational, "held at the band floor");
    }

    // Force a wrong pairing (2 + 3 ≠ 10) and resolve it.
    fn force_miss(s: &mut ShooterSession) {
        s.aliens = vec![
            Alien { id: 0, x: 20.0, y: 20.0, value: 2, selected: true },
            Alien { id: 1, x: 40.0, y: 20.0, value: 3, selected: true },
        ];
        resolve_selection(s);
    }

    // Force a correct pairing (4 + 6 = 10) and resolve it.
    fn force_hit(s: &mut ShooterSession) {
        s.aliens = vec![
            Alien { id: 0, x: 20.0, y: 20.0, value: 4, selected: true },
            Alien { id: 1, x: 40.0, y: 20.0, value: 6, selected: true },
        ];
        resolve_selection(s);
    }

    #[test]
    fn a_streak_of_misses_scaffolds_representation_down() {
        // Band 1 floors at Concrete, so the full ladder is reachable.
        let mut s = ShooterSession::new(1, CraStage::Abstract, GamePace::Steady, &mut rng());
        s.target = 10;
        force_miss(&mut s);
        force_miss(&mut s);
        assert_eq!(s.representation, CraStage::Abstract, "two misses isn't a struggle yet");
        force_miss(&mut s); // three in a row → one step more concrete
        assert_eq!(s.representation, CraStage::Representational);
        force_miss(&mut s);
        force_miss(&mut s);
        force_miss(&mut s); // another streak of three → concrete
        assert_eq!(s.representation, CraStage::Concrete);
    }

    #[test]
    fn a_correct_pair_resets_the_struggle_streak() {
        let mut s = ShooterSession::new(1, CraStage::Abstract, GamePace::Steady, &mut rng());
        s.target = 10;
        force_miss(&mut s);
        force_miss(&mut s);
        force_hit(&mut s); // resets the streak
        force_miss(&mut s);
        force_miss(&mut s);
        // Only two consecutive misses since the hit — no scaffold.
        assert_eq!(s.representation, CraStage::Abstract);
        assert_eq!(s.miss_streak, 2);
    }

    #[test]
    fn correct_pair_pops_both_and_scores() {
        let s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        let start = s.aliens.len();
        let (a, b) = a_matching_pair(&s);
        let s = shoot(s, a);
        let s = shoot(s, b);
        assert_eq!(s.aliens.len(), start - 2, "both aliens removed");
        assert_eq!(s.score, 1);
        assert_eq!(s.hits, 1);
    }

    #[test]
    fn wrong_pair_just_deselects_no_removal() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        // Force a known non-summing pair.
        s.target = 10;
        s.aliens = vec![
            Alien { id: 0, x: 20.0, y: 20.0, value: 2, selected: false },
            Alien { id: 1, x: 40.0, y: 20.0, value: 3, selected: false },
        ];
        let start = s.aliens.len();
        let s = shoot(s, 0);
        let s = shoot(s, 1); // 2 + 3 ≠ 10
        assert_eq!(s.aliens.len(), start, "nothing removed on a miss");
        assert!(s.aliens.iter().all(|a| !a.selected), "both deselected");
        assert_eq!(s.misses, 1);
        assert_eq!(s.score, 0);
    }

    #[test]
    fn firing_into_empty_space_does_nothing() {
        let mut s = ShooterSession::new(1, CraStage::Abstract, GamePace::Steady, &mut rng());
        s.aliens = vec![Alien { id: 0, x: 10.0, y: 20.0, value: 2, selected: false }];
        s.ship_x = 90.0; // far from the only alien
        let mut s = shooter_reducer(s, ShooterAction::Fire);
        // The bolt sails up its empty column and off the top, tagging nothing.
        for _ in 0..200 {
            if s.shots.is_empty() { break; }
            s = shooter_reducer(s, ShooterAction::Tick { dt: 1.0 / 60.0 });
        }
        assert!(s.shots.is_empty(), "the bolt should sail off the top");
        assert!(s.aliens.iter().all(|a| !a.selected), "no alien in the column");
    }

    #[test]
    fn a_bolt_travels_before_it_tags_an_alien() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        s.aliens = vec![Alien { id: 0, x: 30.0, y: 20.0, value: 4, selected: false }];
        s.ship_x = 30.0;
        s = shooter_reducer(s, ShooterAction::Fire);
        // Right after firing the bolt is in flight, well below the alien, and
        // nothing is tagged yet.
        assert_eq!(s.shots.len(), 1);
        assert!(s.shots[0].y > s.aliens[0].y, "bolt starts below the alien");
        assert!(!s.aliens[0].selected, "not tagged on the firing frame");
        // Let it climb; it tags the alien and is consumed.
        for _ in 0..200 {
            if s.aliens[0].selected { break; }
            s = shooter_reducer(s, ShooterAction::Tick { dt: 1.0 / 60.0 });
        }
        assert!(s.aliens[0].selected, "the bolt reached and tagged the alien");
        assert!(s.shots.is_empty(), "the bolt is consumed on impact");
    }

    #[test]
    fn breach_dims_shield_and_retreats_alien() {
        let mut s = ShooterSession::new(5, CraStage::Abstract, GamePace::Steady, &mut rng());
        let shield0 = s.shield;
        // Park one alien right at the line, the rest safely up top.
        for (i, a) in s.aliens.iter_mut().enumerate() {
            a.y = if i == 0 { BREACH_Y - 0.01 } else { SPAWN_Y };
        }
        let start = s.aliens.len();
        let s = shooter_reducer(s, ShooterAction::Tick { dt: 1.0 });
        assert_eq!(s.aliens.len(), start - 1, "the breached alien retreats");
        assert_eq!(s.shield, shield0 - 1, "shield dims one notch");
    }

    #[test]
    fn empty_shield_freezes_the_drift() {
        let mut s = ShooterSession::new(5, CraStage::Abstract, GamePace::Steady, &mut rng());
        s.shield = 0;
        let y_before: Vec<f32> = s.aliens.iter().map(|a| a.y).collect();
        let s = shooter_reducer(s, ShooterAction::Tick { dt: 1.0 });
        let y_after: Vec<f32> = s.aliens.iter().map(|a| a.y).collect();
        assert_eq!(y_before, y_after, "aliens hover while the shield is empty");
    }

    #[test]
    fn correct_pair_refills_shield_up_to_max() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        s.shield = 1;
        let (a, b) = a_matching_pair(&s);
        let s = shoot(s, a);
        let s = shoot(s, b);
        assert_eq!(s.shield, 2, "a clear tops the shield up a notch");
    }

    #[test]
    fn clearing_a_wave_advances_to_the_next() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        assert_eq!(s.wave, 0);
        // Clear every alien in wave 0 (stop as soon as the wave index advances).
        while s.wave == 0 {
            let (a, b) = a_matching_pair(&s);
            s = shoot(s, a);
            s = shoot(s, b);
        }
        assert_eq!(s.wave, 1, "advanced to the next wave");
        assert!(!s.aliens.is_empty(), "next wave spawned");
        assert_eq!(s.phase, ShooterPhase::Playing);
    }

    #[test]
    fn clearing_the_last_wave_completes_the_run() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        while s.phase == ShooterPhase::Playing {
            let (a, b) = a_matching_pair(&s);
            s = shoot(s, a);
            s = shoot(s, b);
        }
        assert_eq!(s.phase, ShooterPhase::Complete);
        assert_eq!(s.wave, TOTAL_WAVES);
    }

    #[test]
    fn completed_run_ignores_further_actions() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        while s.phase == ShooterPhase::Playing {
            let (a, b) = a_matching_pair(&s);
            s = shoot(s, a);
            s = shoot(s, b);
        }
        let score = s.score;
        let s = shooter_reducer(s, ShooterAction::Fire);
        let s = shooter_reducer(s, ShooterAction::Tick { dt: 5.0 });
        assert_eq!(s.score, score, "no further change once complete");
    }

    #[test]
    fn move_ship_clamps_to_the_field() {
        let s = ShooterSession::new(1, CraStage::Abstract, GamePace::Steady, &mut rng());
        let s = shooter_reducer(s, ShooterAction::MoveShip { dx: -1000.0 });
        assert_eq!(s.ship_x, 0.0);
        let s = shooter_reducer(s, ShooterAction::MoveShip { dx: 1000.0 });
        assert_eq!(s.ship_x, FIELD_W);
    }
}
