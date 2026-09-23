//! The Goyish Map — a number-bond space shooter. Pure logic, no rendering.
//!
//! A row of numbered aliens drifts slowly down toward the ship. A target is
//! shown ("MAKE 10"). The ship sits in *lanes* — one per alien column. The kid
//! hops it from alien to alien (`Hop`) and fires a bolt that travels visibly up
//! its column and tags the alien there — so every keyboard shot is aimed at
//! exactly one number, and a wrong pair is a maths signal, never bad aim. A tap
//! on the field snaps the ship to the tapped column and fires from there. Tag
//! two aliens whose numbers *sum to the target* and both pop — number bonds (part-part-whole) ARE the aiming
//! logic, so this passes the Broccoli Test. A wrong pair simply deselects
//! (never a "WRONG", never punishment).
//!
//! No clock, no stakes (Invariant 4): the aliens drift down to the floor line
//! (`FLOOR_Y`) and WAIT there, hovering, for as long as the kid needs. Nothing
//! is ever lost by being slow — an alien never leaves the field except by
//! being paired. A wave clears only when every alien in it has been paired off,
//! and the run completes only when every wave has. There is no game-over.
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
//! Stealth assessment: the session keeps an ordered log of every pairing
//! attempt (`attempts`) and a record per cleared wave (`cleared_waves`) with
//! the time spent on it. The kid never sees any of it.
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
/// How far down the aliens drift. They stop here and hover until paired —
/// comfortably above the ship so a waiting alien never sits on top of it.
pub const FLOOR_Y: f32 = 80.0;
/// A bolt tags an alien whose column is within this of the bolt. Kept well
/// under half the alien spacing (~15) so hit-zones never overlap — a bolt fired
/// between two aliens misses both, which reads clearly and rewards lining up.
pub const HIT_TOLERANCE: f32 = 6.0;
/// Bolt travel speed, logical units/sec (upward). Fast enough to feel snappy,
/// slow enough that the kid watches it cross the gap.
pub const SHOT_SPEED: f32 = 150.0;
/// Where a fired bolt starts — just above the ship on the bottom rail.
pub const SHOT_SPAWN_Y: f32 = 96.0;
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

/// How a bolt was aimed. Silent assessment only: a tap means the kid pointed
/// straight at the number they chose; keys mean they steered the ship there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShotSource {
    /// Tap/click on the field: the ship snapped to the tapped column.
    Tap,
    /// Arrow keys (or A/D) to hop lanes, then Space/Enter.
    Keys,
    /// Fired via the legacy `ShooterAction::Fire`, which doesn't say.
    #[default]
    Unknown,
}

/// A bolt in flight: fixed column `x`, `y` rising toward 0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shot {
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub source: ShotSource,
}

/// One pairing attempt — two aliens tagged, resolved as a bond or not. Logged
/// in the order they happen so the game can feed the learner profile in true
/// order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairAttempt {
    pub correct: bool,
    /// The target on screen when the pair was made.
    pub target: u32,
    /// The two values, in the order they were tagged.
    pub values: [u32; 2],
    /// How each of the two selections was aimed, in tag order.
    pub sources: [ShotSource; 2],
    /// Index of the wave this happened in.
    pub wave: usize,
    /// Session clock (seconds of `Tick`) when the pair resolved.
    pub at_secs: f32,
    /// Seconds since the previous attempt resolved (or since the wave spawned,
    /// for the wave's first attempt) — a per-pair think time.
    pub think_secs: f32,
}

/// A wave the kid finished by pairing off every alien in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveRecord {
    pub wave: usize,
    pub target: u32,
    /// Seconds from the wave spawning to its last pair popping.
    pub secs: f32,
    pub hits: u32,
    /// Mis-pairs during this wave.
    pub misses: u32,
}

impl WaveRecord {
    /// Every pair in the wave was a real bond — no mis-pairs. Time never
    /// enters into it (Invariant 4).
    pub fn is_clean(&self) -> bool {
        self.misses == 0
    }
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

/// Which way a `Hop` goes along the lane row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HopDir {
    Left,
    Right,
}

/// Two lane positions closer than this are the same lane (the ship is "on" it).
const LANE_EPS: f32 = 0.5;

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
    /// Pairs cleared across the whole run — the "score".
    pub score: u32,
    /// Index into `waves` of the wave now on screen. Also the number of waves
    /// cleared so far — waves only ever clear by pairing.
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
    /// Session clock: total seconds of `Tick` so far. Measured silently, never
    /// shown (Invariant 4).
    pub elapsed: f32,
    /// Every pairing attempt, in the order it happened.
    pub attempts: Vec<PairAttempt>,
    /// One record per wave cleared, in order.
    pub cleared_waves: Vec<WaveRecord>,
    /// Session clock when the current wave spawned.
    wave_started_at: f32,
    /// Session clock when the last attempt resolved (or the wave spawned).
    last_attempt_at: f32,
    /// The currently-tagged aliens and how each was aimed, in tag order.
    tagged: Vec<(u32, ShotSource)>,
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
            elapsed: 0.0,
            attempts: Vec::new(),
            cleared_waves: Vec::new(),
            wave_started_at: 0.0,
            last_attempt_at: 0.0,
            tagged: Vec::new(),
            next_id: 0,
        };
        session.spawn_current_wave();
        session
    }

    /// The ship's lanes: the column of every alien still waiting to be tagged,
    /// left to right. Aliens never move sideways, so a lane is fixed until its
    /// alien is tagged or popped.
    pub fn lanes(&self) -> Vec<f32> {
        let mut xs: Vec<f32> = self.aliens.iter().filter(|a| !a.selected).map(|a| a.x).collect();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        xs
    }

    /// The lane nearest `x` (a tie goes left, so it's deterministic), or `None`
    /// when no alien is waiting.
    fn nearest_lane(&self, x: f32) -> Option<f32> {
        self.lanes().into_iter().fold(None, |best: Option<f32>, lane| match best {
            Some(b) if (b - x).abs() <= (lane - x).abs() + 1e-3 => Some(b),
            _ => Some(lane),
        })
    }

    /// Put the ship in the lane nearest where it already is, so the kid's eye
    /// doesn't have to jump — used when a wave spawns and when a pair pops out
    /// from under the ship. A ship already in a lane stays put.
    fn settle_ship(&mut self) {
        if let Some(lane) = self.nearest_lane(self.ship_x) {
            self.ship_x = lane;
        }
    }

    /// Waves finished by pairing. (The only way a wave ever finishes.)
    pub fn waves_cleared(&self) -> usize {
        self.cleared_waves.len()
    }

    /// Lay the current wave's aliens out in a row across the field at the top.
    fn spawn_current_wave(&mut self) {
        self.shots.clear();
        self.tagged.clear();
        self.wave_started_at = self.elapsed;
        self.last_attempt_at = self.elapsed;
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
        self.settle_ship();
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

/// Drift speed, logical units/sec — purely how quickly the wave settles onto
/// the floor line, since nothing happens when it gets there. `pace` is the
/// parent's dial for kids who find moving targets hard to read.
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
    /// Advance the world by `dt` seconds (alien drift down to the floor, bolt
    /// flight and collisions, and the silent session clock).
    Tick { dt: f32 },
    /// Hop the ship to the next lane (the next alien still waiting) in `dir`.
    /// At the end of the row it stays put — nothing wraps, so the ship is
    /// always where the kid expects it. The keyboard path.
    Hop { dir: HopDir },
    /// Slide the ship horizontally (clamped to the field). Tap-to-shoot uses
    /// it to snap the ship to the tapped column.
    MoveShip { dx: f32 },
    /// Fire a bolt up the ship's column, without saying how it was aimed
    /// (logged as `ShotSource::Unknown`). Prefer `FireFrom`.
    Fire,
    /// Fire a bolt up the ship's column, recording how the kid aimed it. For a
    /// tap: `MoveShip` to the tapped column, then `FireFrom { source: Tap }`.
    FireFrom { source: ShotSource },
}

/// Resolve a completed selection: when two aliens are tagged, pop them if their
/// values sum to the target, otherwise release both (gentle, never punished).
/// Either way the attempt is logged in order.
fn resolve_selection(s: &mut ShooterSession) {
    if s.selected_count() != 2 {
        return;
    }
    // Tag order comes from the tag log; anything selected without going
    // through a bolt (only possible in tests) falls in after, as Unknown.
    let mut picked: Vec<(u32, ShotSource)> = s.tagged.iter().copied()
        .filter(|(id, _)| s.aliens.iter().any(|a| a.id == *id && a.selected))
        .collect();
    for a in s.aliens.iter().filter(|a| a.selected) {
        if !picked.iter().any(|(id, _)| *id == a.id) {
            picked.push((a.id, ShotSource::Unknown));
        }
    }
    let value_of = |id: u32| s.aliens.iter().find(|a| a.id == id).map_or(0, |a| a.value);
    let values = [value_of(picked[0].0), value_of(picked[1].0)];
    let sources = [picked[0].1, picked[1].1];
    let correct = values[0] + values[1] == s.target;
    s.attempts.push(PairAttempt {
        correct,
        target: s.target,
        values,
        sources,
        wave: s.wave,
        at_secs: s.elapsed,
        think_secs: s.elapsed - s.last_attempt_at,
    });
    s.last_attempt_at = s.elapsed;
    s.tagged.clear();

    if correct {
        s.aliens.retain(|a| !a.selected);
        // The pair popped from under the ship: slide it into the nearest
        // lane that's left, so the next Space always has something to tag.
        s.settle_ship();
        s.score += 1;
        s.hits += 1;
        s.miss_streak = 0; // a correct pair clears the struggle streak
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
        ShooterAction::Hop { dir } => {
            let x = next.ship_x;
            let lanes = next.lanes();
            let to = match dir {
                HopDir::Left => lanes.into_iter().rev().find(|&l| l < x - LANE_EPS),
                HopDir::Right => lanes.into_iter().find(|&l| l > x + LANE_EPS),
            };
            if let Some(lane) = to {
                next.ship_x = lane;
            }
        }
        ShooterAction::MoveShip { dx } => {
            next.ship_x = (next.ship_x + dx).clamp(0.0, FIELD_W);
        }
        ShooterAction::Tick { dt } => {
            let dt = dt.max(0.0);
            next.elapsed += dt;
            // Drift down to the floor line and wait there. Nothing is ever
            // lost by taking your time.
            for a in &mut next.aliens {
                a.y = (a.y + next.drift_speed * dt).min(FLOOR_Y);
            }

            // Each bolt rises; the first alien it reaches gets tagged.
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
                    next.tagged.push((next.aliens[idx].id, shot.source));
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
            next.shots.push(Shot { x: next.ship_x, y: SHOT_SPAWN_Y, source: ShotSource::Unknown });
        }
        ShooterAction::FireFrom { source } => {
            next.shots.push(Shot { x: next.ship_x, y: SHOT_SPAWN_Y, source });
        }
    }

    // Every alien paired off → record the wave and advance, or finish the run
    // if that was the final wave. Pairing is the only way aliens leave.
    if next.phase == ShooterPhase::Playing && next.aliens.is_empty() {
        let wave = next.wave;
        let (hits, misses) = next.attempts.iter()
            .filter(|a| a.wave == wave)
            .fold((0, 0), |(h, m), a| if a.correct { (h + 1, m) } else { (h, m + 1) });
        next.cleared_waves.push(WaveRecord {
            wave,
            target: next.target,
            secs: next.elapsed - next.wave_started_at,
            hits,
            misses,
        });
        next.wave += 1;
        if next.wave < next.waves.len() {
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
    fn a_relaxed_pace_settles_the_wave_about_half_as_fast() {
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
        assert!((1.7..2.1).contains(&ratio), "expected ~2x slower drift, got {ratio}x");
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

    /// Tick an untouched session for `secs` seconds at 60fps.
    fn idle(mut s: ShooterSession, secs: f32) -> ShooterSession {
        let frames = (secs * 60.0) as usize;
        for _ in 0..frames {
            s = shooter_reducer(s, ShooterAction::Tick { dt: 1.0 / 60.0 });
        }
        s
    }

    #[test]
    fn an_idle_kid_loses_nothing_the_aliens_just_wait_at_the_floor() {
        for band in [0u8, 1, 3, 5, 10] {
            for pace in GamePace::ALL {
                let s = ShooterSession::new(band, CraStage::Abstract, pace, &mut rng());
                let start = s.aliens.len();
                let s = idle(s, 120.0);
                assert_eq!(s.phase, ShooterPhase::Playing, "band {band} {pace:?}: never completes on its own");
                assert_eq!(s.wave, 0, "band {band} {pace:?}: the wave is still there");
                assert_eq!(s.aliens.len(), start, "band {band} {pace:?}: no alien ever leaves unpaired");
                assert!(s.aliens.iter().all(|a| a.y <= FLOOR_Y),
                    "band {band} {pace:?}: every alien waits at or above the floor");
                assert!(s.aliens.iter().all(|a| a.y == FLOOR_Y),
                    "band {band} {pace:?}: after two minutes they've all settled on it");
                assert!(s.cleared_waves.is_empty() && s.attempts.is_empty());
            }
        }
    }

    #[test]
    fn a_waiting_alien_can_still_be_paired() {
        let s = idle(ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng()), 60.0);
        let start = s.aliens.len();
        let (a, b) = a_matching_pair(&s);
        let s = shoot(shoot(s, a), b);
        assert_eq!(s.aliens.len(), start - 2);
        assert_eq!(s.hits, 1);
    }

    #[test]
    fn pace_only_changes_how_fast_the_wave_settles() {
        // Same seed, every pace: identical waves, targets and alien count; the
        // only difference is drift speed.
        let runs: Vec<ShooterSession> = GamePace::ALL.iter()
            .map(|p| ShooterSession::new(4, CraStage::Abstract, *p, &mut rng()))
            .collect();
        for r in &runs[1..] {
            assert_eq!(r.target, runs[0].target);
            assert_eq!(r.aliens, runs[0].aliens);
            let tv = |s: &ShooterSession| s.waves.iter().map(|w| (w.target, w.values.clone())).collect::<Vec<_>>();
            assert_eq!(tv(r), tv(&runs[0]), "pace must not touch the number bonds");
            assert_eq!(r.representation, runs[0].representation);
        }
        let drifts: Vec<f32> = runs.iter().map(|r| r.drift_speed).collect();
        assert!(drifts.windows(2).all(|w| w[0] < w[1]), "slowest pace first: {drifts:?}");
    }

    #[test]
    fn every_pairing_attempt_is_logged_in_the_order_it_happened() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        // A deliberate mis-pair first: find two aliens that DON'T sum.
        let (x, y) = {
            let mut found = None;
            'o: for i in 0..s.aliens.len() {
                for j in (i + 1)..s.aliens.len() {
                    if s.aliens[i].value + s.aliens[j].value != s.target {
                        found = Some((s.aliens[i].id, s.aliens[j].id));
                        break 'o;
                    }
                }
            }
            found.expect("a 6-alien wave has a non-summing pair")
        };
        s = shoot(s, x);
        s = shoot(s, y);
        let (a, b) = a_matching_pair(&s);
        let (va, vb) = {
            let v = |id| s.aliens.iter().find(|al| al.id == id).unwrap().value;
            (v(a), v(b))
        };
        s = shoot(s, a);
        s = shoot(s, b);
        let order: Vec<bool> = s.attempts.iter().map(|t| t.correct).collect();
        assert_eq!(order, vec![false, true], "miss then hit, in true order");
        assert_eq!(s.attempts[1].values, [va, vb], "values in the order they were tagged");
        assert!(s.attempts[1].at_secs >= s.attempts[0].at_secs);
        assert!(s.attempts.iter().all(|t| t.wave == 0 && t.target == s.target));
        assert_eq!((s.hits, s.misses), (1, 1));
    }

    #[test]
    fn each_shot_remembers_how_it_was_aimed() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        let (a, b) = a_matching_pair(&s);
        let fire = |mut s: ShooterSession, id: u32, action: ShooterAction| {
            let x = s.aliens.iter().find(|al| al.id == id).unwrap().x;
            s = shooter_reducer(s.clone(), ShooterAction::MoveShip { dx: x - s.ship_x });
            s = shooter_reducer(s, action);
            while !s.shots.is_empty() {
                s = shooter_reducer(s, ShooterAction::Tick { dt: 1.0 / 60.0 });
            }
            s
        };
        s = fire(s, a, ShooterAction::FireFrom { source: ShotSource::Tap });
        s = fire(s, b, ShooterAction::FireFrom { source: ShotSource::Keys });
        assert_eq!(s.attempts[0].sources, [ShotSource::Tap, ShotSource::Keys]);

        // The legacy Fire still works; it just doesn't know.
        let (c, d) = a_matching_pair(&s);
        s = fire(s, c, ShooterAction::Fire);
        s = fire(s, d, ShooterAction::Fire);
        assert_eq!(s.attempts[1].sources, [ShotSource::Unknown, ShotSource::Unknown]);
        assert!(s.attempts[1].correct);
    }

    #[test]
    fn a_cleared_wave_records_how_long_it_took_and_how_it_went() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        s = idle(s, 5.0);
        while s.wave == 0 {
            let (a, b) = a_matching_pair(&s);
            s = shoot(s, a);
            s = shoot(s, b);
        }
        assert_eq!(s.waves_cleared(), 1);
        let w = &s.cleared_waves[0];
        assert_eq!(w.wave, 0);
        assert_eq!(w.target, s.waves[0].target);
        assert_eq!((w.hits, w.misses), (3, 0), "six aliens, three clean pairs");
        assert!(w.secs >= 5.0 && w.secs < 10.0, "wave time is measured silently: {}", w.secs);
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

    fn hop(s: ShooterSession, dir: HopDir) -> ShooterSession {
        shooter_reducer(s, ShooterAction::Hop { dir })
    }

    fn on_a_lane(s: &ShooterSession) -> bool {
        s.lanes().iter().any(|&l| (l - s.ship_x).abs() < 1e-3)
    }

    /// Keyboard-only: hop toward alien `id` until the ship is under it, fire,
    /// and let the bolt land. Panics if hopping can't reach it.
    fn hop_and_fire(mut s: ShooterSession, id: u32) -> ShooterSession {
        for _ in 0..20 {
            let ax = s.aliens.iter().find(|a| a.id == id).expect("alien on screen").x;
            if (s.ship_x - ax).abs() < 1e-3 {
                s = shooter_reducer(s, ShooterAction::FireFrom { source: ShotSource::Keys });
                while !s.shots.is_empty() {
                    s = shooter_reducer(s, ShooterAction::Tick { dt: 1.0 / 60.0 });
                }
                return s;
            }
            let dir = if ax > s.ship_x { HopDir::Right } else { HopDir::Left };
            s = hop(s, dir);
        }
        panic!("hopping never reached alien {id}");
    }

    #[test]
    fn a_fresh_wave_puts_the_ship_in_a_lane() {
        for band in 0..=10u8 {
            let s = ShooterSession::new(band, CraStage::Abstract, GamePace::Steady, &mut rng());
            assert!(on_a_lane(&s), "band {band}: ship at {} is between aliens", s.ship_x);
            // Space straight away tags something — no dead first shot.
            let s = shooter_reducer(s, ShooterAction::FireFrom { source: ShotSource::Keys });
            let s = idle(s, 1.5);
            assert_eq!(s.aliens.iter().filter(|a| a.selected).count(), 1, "band {band}");
        }
    }

    #[test]
    fn hops_walk_the_row_in_order_and_stop_at_the_ends() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        let lanes = s.lanes();
        assert_eq!(lanes.len(), 6);
        // Walk off the left end: it stops on the leftmost alien and stays.
        for _ in 0..10 {
            s = hop(s, HopDir::Left);
        }
        assert_eq!(s.ship_x, lanes[0], "left end: stays put, no wrap");
        // Right, one alien at a time, in order.
        for want in &lanes[1..] {
            s = hop(s, HopDir::Right);
            assert_eq!(s.ship_x, *want);
        }
        let s = hop(s, HopDir::Right);
        assert_eq!(s.ship_x, *lanes.last().unwrap(), "right end: stays put, no wrap");
    }

    #[test]
    fn a_hop_from_between_lanes_lands_on_the_next_one_that_way() {
        let mut s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        let lanes = s.lanes();
        s.ship_x = (lanes[1] + lanes[2]) / 2.0; // e.g. after tapping empty space
        assert_eq!(hop(s.clone(), HopDir::Right).ship_x, lanes[2]);
        assert_eq!(hop(s, HopDir::Left).ship_x, lanes[1]);
    }

    #[test]
    fn a_hop_skips_an_alien_already_tagged() {
        let s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        let lanes = s.lanes();
        let second = s.aliens.iter().find(|a| a.x == lanes[1]).unwrap().id;
        let mut s = hop_and_fire(s, second);
        assert!(s.aliens.iter().any(|a| a.id == second && a.selected));
        s.ship_x = lanes[0];
        let s = hop(s, HopDir::Right);
        assert_eq!(s.ship_x, lanes[2], "the tagged alien is not a lane");
    }

    #[test]
    fn after_a_pair_pops_the_ship_lands_in_the_nearest_lane_left() {
        let s = ShooterSession::new(3, CraStage::Abstract, GamePace::Steady, &mut rng());
        let (a, b) = a_matching_pair(&s);
        let popped_at = s.aliens.iter().find(|al| al.id == b).unwrap().x;
        let s = hop_and_fire(hop_and_fire(s, a), b);
        assert_eq!((s.hits, s.aliens.len()), (1, 4));
        assert!(on_a_lane(&s), "ship at {} after the pop is between aliens", s.ship_x);
        let nearest = s.lanes().into_iter()
            .min_by(|x, y| (x - popped_at).abs().partial_cmp(&(y - popped_at).abs()).unwrap())
            .unwrap();
        assert_eq!(s.ship_x, nearest, "the ship slides to the closest alien left");
    }

    #[test]
    fn hopping_alone_can_clear_every_wave_at_every_band() {
        // Lanes must never strand an alien: with only hops and Space, every
        // wave pairs off and the run completes.
        for seed in 0..8u64 {
            for band in 0..=10u8 {
                let mut r = SmallRng::seed_from_u64(seed);
                let mut s = ShooterSession::new(band, CraStage::Abstract, GamePace::Steady, &mut r);
                let mut guard = 0;
                while s.phase == ShooterPhase::Playing {
                    guard += 1;
                    assert!(guard < 40, "seed {seed} band {band}: run never finished");
                    let (a, b) = a_matching_pair(&s);
                    s = hop_and_fire(s, a);
                    s = hop_and_fire(s, b);
                    if s.phase == ShooterPhase::Playing {
                        assert!(on_a_lane(&s), "seed {seed} band {band}: ship off-lane");
                    }
                }
                assert_eq!(s.waves_cleared(), TOTAL_WAVES);
                assert!(s.cleared_waves.iter().all(|w| w.is_clean()));
                assert!(s.attempts.iter().all(|t| t.sources == [ShotSource::Keys; 2]));
            }
        }
    }
}
