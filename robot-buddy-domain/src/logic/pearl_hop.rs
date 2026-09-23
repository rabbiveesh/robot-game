//! Pearl Hop — Shelly the clam flings herself along a number path at her
//! pearl. Pure logic, no rendering.
//!
//! The kid never walks anywhere. The one thing they can do is toss Shelly:
//! pull her back (or nudge the aim with the arrow keys) and let go. How far
//! she goes IS the arithmetic, and which arithmetic depends on the stage:
//!
//! | stage       | one toss is…                               | the maths          |
//! |-------------|--------------------------------------------|--------------------|
//! | `Count`     | one hop of `aim` stones                    | counting, 1:1      |
//! | `SkipCount` | hops of `aim`, repeated until she reaches or passes the pearl | skip counting |
//! | `Hops`      | exactly `hops` equal hops of `aim`         | division (inverse) |
//! | `Estimate`  | one hop to `aim` on an unmarked 0..span line | magnitude estimation |
//!
//! Nothing here can fail. Short and she shrugs on a plain stone; long and she
//! splashes into open water and paddles back. Tosses are unlimited and
//! untimed; `tosses` only exists so the game can pay a first-try bonus and
//! the adaptive system can read the attempt, never to show the kid a score.
//!
//! Where she lands is decided the instant she's tossed, from the aim alone.
//! `Tick` only moves the animation clock along, so a slow tablet and a fast
//! laptop play out the exact same toss (and there's no timing skill at all).
//!
//! Public surface mirrors the other logic modules:
//!   - `generate_round(band, &mut impl Rng)` → a round solvable by construction
//!   - `HopSession::new(round)` → fresh session
//!   - `hop_reducer(session, action)` → new session

use rand::Rng;
use serde::{Deserialize, Serialize};

/// Seconds for one hop in the air. The single long hop of the counting stage
/// and the estimation stage takes a bit longer so it reads as a big fling.
pub const HOP_SECS: f32 = 0.55;
pub const BIG_HOP_SECS: f32 = 0.95;
/// Landed short on a plain stone: bonk, wobble, shrug, hop home.
pub const SHRUG_SECS: f32 = 1.8;
/// Sailed past: splash, bob up, paddle back to the start rock.
pub const SPLASH_SECS: f32 = 2.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HopStage {
    /// One toss straight to the pearl, a few stones away.
    Count,
    /// Shelly always hops the same size: one pull sets the hop, she repeats
    /// it, counting aloud, until she reaches the pearl or passes it.
    SkipCount,
    /// "Reach the pearl in K hops": the kid picks the hop size.
    Hops,
    /// No stones: land near a number on an open-water line.
    Estimate,
}

impl HopStage {
    /// Which stage a learner at `band` plays. The youngest count; then skip
    /// counting, then its inverse, then estimation.
    pub fn for_band(band: u8) -> HopStage {
        match band {
            0 | 1 => HopStage::Count,
            2 => HopStage::SkipCount,
            3 => HopStage::Hops,
            _ => HopStage::Estimate,
        }
    }
}

/// One pearl's worth of puzzle.
///
/// All positions (aim, landings, the pearl) are measured in *sub-units*:
/// `scale` of them per number on the path. Stones are whole numbers, so the
/// stone stages use `scale == 1`; the estimation line is continuous, so it
/// uses tenths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HopRound {
    pub stage: HopStage,
    /// The pearl's number: its stone, or its value on the estimation line.
    pub pearl: u16,
    /// The last number drawn. Stone stages: open water runs from the pearl
    /// rock to here. Estimate: the far end of the line (10, 20 or 100).
    pub span: u16,
    /// Hops per toss: 1 (Count, Estimate), K (Hops), or 0 for "repeat until
    /// she reaches or passes the pearl" (SkipCount).
    pub hops: u8,
    /// Sub-units per number (1 for stones, 10 for the estimation line).
    pub scale: u16,
    /// The smallest and largest aim a pull can set, in sub-units.
    pub min_aim: u16,
    pub max_aim: u16,
    /// How close to the pearl counts as landing on it, in sub-units. Zero on
    /// the stones (you land on it or you don't).
    pub tolerance: u16,
}

impl HopRound {
    /// The pearl's position in sub-units.
    pub fn pearl_pos(&self) -> u16 {
        self.pearl * self.scale
    }

    /// The far end of the path in sub-units.
    pub fn span_pos(&self) -> u16 {
        self.span * self.scale
    }

    /// Every spot Shelly touches down on for a toss of `aim`, in order. The
    /// last one is where she ends up.
    pub fn landings(&self, aim: u16) -> Vec<u16> {
        let aim = aim.max(1);
        match self.hops {
            0 => {
                // Repeat the hop until she reaches or passes the pearl.
                let mut out = Vec::new();
                let mut at = 0u16;
                while at < self.pearl_pos() {
                    at = at.saturating_add(aim);
                    out.push(at);
                }
                out
            }
            k => (1..=k as u16).map(|i| aim.saturating_mul(i)).collect(),
        }
    }

    /// What a toss of `aim` does. Pure function of the aim: no clock, no
    /// randomness.
    pub fn resolve(&self, aim: u16) -> Toss {
        let landings = self.landings(aim);
        let end = *landings.last().unwrap_or(&0);
        let pearl = self.pearl_pos();
        let off = end.abs_diff(pearl);
        let landing = if off <= self.tolerance {
            Landing::Pearl
        } else if end < pearl {
            Landing::Short
        } else {
            Landing::Past
        };
        // "So close!" — only meaningful where closeness is the skill.
        let near = landing != Landing::Pearl
            && self.stage == HopStage::Estimate
            && off <= self.tolerance.saturating_mul(2);
        Toss { aim, landings, landing, near }
    }

    /// Every aim in range that lands on the pearl. Never empty for a
    /// generated round.
    pub fn winning_aims(&self) -> Vec<u16> {
        (self.min_aim..=self.max_aim)
            .filter(|&a| self.resolve(a).landing == Landing::Pearl)
            .collect()
    }

    /// One arrow-key press worth of aim: a stone on the stones, a
    /// twentieth of the line out on open water.
    pub fn aim_step(&self) -> u16 {
        match self.stage {
            HopStage::Estimate => (self.span_pos() / 20).max(1),
            _ => self.scale,
        }
    }

    /// The first aim a fresh round starts at: the smallest pull there is.
    pub fn rest_aim(&self) -> u16 {
        self.min_aim
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Landing {
    /// On the pearl. It pops.
    Pearl,
    /// Came down before the pearl: on a plain stone (or open water, on the
    /// estimation line). She shrugs and hops home.
    Short,
    /// Sailed past the pearl: splash, paddle back.
    Past,
}

/// One toss, decided the moment she leaves the rock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Toss {
    pub aim: u16,
    pub landings: Vec<u16>,
    pub landing: Landing,
    /// A near miss on the estimation line: she wobbles on the edge of it.
    pub near: bool,
}

impl Toss {
    /// Where she finally comes down, in sub-units.
    pub fn end(&self) -> u16 {
        *self.landings.last().unwrap_or(&0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HopPhase {
    /// On the start rock, waiting to be pulled.
    Aiming,
    /// In the air (possibly several hops).
    Flying,
    /// Came down somewhere that isn't the pearl; reacting (shrug / splash /
    /// wobble), then back to the rock. Never an ending.
    Landed,
    /// On the pearl. The round is over until it's reset.
    Won,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HopSession {
    pub round: HopRound,
    /// The current pull, in sub-units. Kept between tosses, so after a miss
    /// the kid adjusts from where they were instead of starting over.
    pub aim: u16,
    pub phase: HopPhase,
    /// Tosses made this round. First-try success is the clean bonus.
    pub tosses: u8,
    /// The toss in flight, or the one that just landed.
    pub toss: Option<Toss>,
    /// Seconds into the current phase. Animation only.
    pub clock: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HopAction {
    /// Set the pull. Clamped into the round's range; ignored unless aiming.
    Aim { at: u16 },
    /// Let go. Ignored unless aiming.
    Toss,
    /// Let `dt` seconds of animation pass.
    Tick { dt: f32 },
    /// Back to a fresh start on the same round (used after Shelly's demo, so
    /// her show-off toss never counts against the kid).
    Reset,
}

impl HopSession {
    pub fn new(round: HopRound) -> Self {
        HopSession { aim: round.rest_aim(), phase: HopPhase::Aiming, tosses: 0, toss: None, clock: 0.0, round }
    }

    /// Won on the first toss.
    pub fn was_clean(&self) -> bool {
        self.phase == HopPhase::Won && self.tosses == 1
    }

    /// Seconds each hop of `toss` takes in the air.
    pub fn hop_secs(&self) -> f32 {
        match self.round.stage {
            HopStage::Count | HopStage::Estimate => BIG_HOP_SECS,
            _ => HOP_SECS,
        }
    }

    /// Total air time of the toss in flight.
    pub fn flight_secs(&self) -> f32 {
        self.toss.as_ref().map_or(0.0, |t| t.landings.len() as f32 * self.hop_secs())
    }

    /// While flying: which hop she's on (0-based) and how far through it
    /// (0..1). Drives the arc, the spin and the count-aloud.
    pub fn flight(&self) -> Option<(usize, f32)> {
        if self.phase != HopPhase::Flying {
            return None;
        }
        let per = self.hop_secs();
        let n = self.toss.as_ref()?.landings.len().max(1);
        let hop = ((self.clock / per) as usize).min(n - 1);
        let t = ((self.clock - hop as f32 * per) / per).clamp(0.0, 1.0);
        Some((hop, t))
    }

    /// How long the reaction to a miss lasts before she's back on the rock.
    pub fn reaction_secs(&self) -> f32 {
        if self.lands_in_water() { SPLASH_SECS } else { SHRUG_SECS }
    }

    /// Did the toss that just landed come down in the water (splash, paddle
    /// back) rather than on a plain stone (shrug)? Past the pearl there are
    /// no stones, and out on the estimation line there are none at all.
    pub fn lands_in_water(&self) -> bool {
        match self.toss.as_ref().map(|t| t.landing) {
            Some(Landing::Past) => true,
            Some(Landing::Short) => self.round.stage == HopStage::Estimate,
            _ => false,
        }
    }
}

pub fn hop_reducer(state: HopSession, action: HopAction) -> HopSession {
    let mut s = state;
    match action {
        HopAction::Aim { at } => {
            if s.phase == HopPhase::Aiming {
                s.aim = at.clamp(s.round.min_aim, s.round.max_aim);
            }
        }
        HopAction::Toss => {
            if s.phase == HopPhase::Aiming {
                s.toss = Some(s.round.resolve(s.aim));
                s.tosses = s.tosses.saturating_add(1);
                s.phase = HopPhase::Flying;
                s.clock = 0.0;
            }
        }
        HopAction::Tick { dt } => {
            // A negative or NaN dt is nonsense; treat it as no time passing.
            let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
            s.clock += dt;
            match s.phase {
                HopPhase::Flying if s.clock >= s.flight_secs() => {
                    let won = s.toss.as_ref().is_some_and(|t| t.landing == Landing::Pearl);
                    s.phase = if won { HopPhase::Won } else { HopPhase::Landed };
                    s.clock = 0.0;
                }
                HopPhase::Landed if s.clock >= s.reaction_secs() => {
                    s.phase = HopPhase::Aiming;
                    s.clock = 0.0;
                }
                _ => {}
            }
        }
        HopAction::Reset => {
            s = HopSession::new(s.round);
        }
    }
    s
}

/// Build a round for `band`. Solvable by construction: the pearl is placed
/// from a winning aim, never searched for.
pub fn generate_round(band: u8, rng: &mut impl Rng) -> HopRound {
    match HopStage::for_band(band) {
        HopStage::Count => {
            let pearl = rng.gen_range(3..=6);
            HopRound {
                stage: HopStage::Count, pearl, span: pearl + 3, hops: 1, scale: 1,
                min_aim: 1, max_aim: pearl + 3, tolerance: 0,
            }
        }
        HopStage::SkipCount => {
            // A hop of 2, 3 or 5 repeated 2–4 times. The aim starts at 2
            // (a hop of 1 is just walking) and stops short of the pearl (one
            // giant hop isn't skip counting). Any hop that divides the pearl
            // lands on it; an odd pearl rules out the always-2s habit.
            let size = [2u16, 3, 5][rng.gen_range(0..3)];
            let times = rng.gen_range(2..=4);
            let pearl = size * times;
            HopRound {
                stage: HopStage::SkipCount, pearl, span: pearl + 3, hops: 0, scale: 1,
                min_aim: 2, max_aim: (pearl - 1).min(6), tolerance: 0,
            }
        }
        HopStage::Hops => {
            let k = rng.gen_range(2..=4u8);
            let size = rng.gen_range(2..=5u16);
            let pearl = size * k as u16;
            HopRound {
                stage: HopStage::Hops, pearl, span: pearl + 3, hops: k, scale: 1,
                min_aim: 1, max_aim: 8, tolerance: 0,
            }
        }
        HopStage::Estimate => {
            let span: u16 = match band {
                0..=4 => 10,
                5 => 20,
                _ => 100,
            };
            // Keep off the ends and the dead centre — those are free.
            let edge = (span / 10).max(2);
            let pearl = loop {
                let p = rng.gen_range(edge..=span - edge);
                if p.abs_diff(span / 2) > span / 20 {
                    break p;
                }
            };
            let scale = 10;
            HopRound {
                stage: HopStage::Estimate, pearl, span, hops: 1, scale,
                min_aim: 1, max_aim: span * scale,
                // Within 8% of the line is on it.
                tolerance: span * scale * 8 / 100,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn count(pearl: u16) -> HopRound {
        HopRound { stage: HopStage::Count, pearl, span: pearl + 3, hops: 1, scale: 1, min_aim: 1, max_aim: pearl + 3, tolerance: 0 }
    }

    fn land(mut s: HopSession) -> HopSession {
        for _ in 0..200 {
            if matches!(s.phase, HopPhase::Landed | HopPhase::Won) {
                return s;
            }
            s = hop_reducer(s, HopAction::Tick { dt: 0.05 });
        }
        s
    }

    fn settle(mut s: HopSession) -> HopSession {
        for _ in 0..400 {
            if matches!(s.phase, HopPhase::Aiming | HopPhase::Won) && s.toss.is_some() {
                return s;
            }
            s = hop_reducer(s, HopAction::Tick { dt: 0.05 });
        }
        s
    }

    fn toss_at(s: HopSession, aim: u16) -> HopSession {
        let s = hop_reducer(s, HopAction::Aim { at: aim });
        hop_reducer(s, HopAction::Toss)
    }

    #[test]
    fn counting_the_stones_to_the_pearl_wins_it() {
        let s = land(toss_at(HopSession::new(count(4)), 4));
        assert_eq!(s.phase, HopPhase::Won);
        assert!(s.was_clean());
    }

    #[test]
    fn too_far_splashes_and_she_paddles_back_to_try_again() {
        let s = land(toss_at(HopSession::new(count(4)), 6));
        assert_eq!(s.phase, HopPhase::Landed, "a miss is a beat, not an ending");
        assert_eq!(s.toss.as_ref().unwrap().landing, Landing::Past);
        let s = settle(s);
        assert_eq!(s.phase, HopPhase::Aiming, "back on the rock");
        assert_eq!(s.aim, 6, "the pull is remembered so the kid adjusts from it");
        let s = land(toss_at(s, 4));
        assert_eq!(s.phase, HopPhase::Won);
        assert!(!s.was_clean(), "second try: the pearl, but no first-try bonus");
    }

    #[test]
    fn too_short_lands_on_a_plain_stone() {
        let s = land(toss_at(HopSession::new(count(5)), 3));
        let t = s.toss.as_ref().unwrap();
        assert_eq!(t.landing, Landing::Short);
        assert_eq!(t.end(), 3);
    }

    #[test]
    fn skip_counting_repeats_the_hop_until_it_reaches_or_passes() {
        let r = HopRound { stage: HopStage::SkipCount, pearl: 9, span: 12, hops: 0, scale: 1, min_aim: 2, max_aim: 6, tolerance: 0 };
        assert_eq!(r.landings(3), vec![3, 6, 9]);
        assert_eq!(r.resolve(3).landing, Landing::Pearl);
        assert_eq!(r.landings(2), vec![2, 4, 6, 8, 10], "2s hop right over an odd pearl");
        assert_eq!(r.resolve(2).landing, Landing::Past);
        assert_eq!(r.winning_aims(), vec![3]);
    }

    #[test]
    fn k_hops_is_the_inverse() {
        let r = HopRound { stage: HopStage::Hops, pearl: 12, span: 15, hops: 3, scale: 1, min_aim: 1, max_aim: 8, tolerance: 0 };
        assert_eq!(r.landings(4), vec![4, 8, 12]);
        assert_eq!(r.resolve(3).landing, Landing::Short);
        assert_eq!(r.resolve(5).landing, Landing::Past);
        assert_eq!(r.winning_aims(), vec![4]);
    }

    #[test]
    fn estimation_takes_close_enough_and_wobbles_near_misses() {
        let r = HopRound { stage: HopStage::Estimate, pearl: 37, span: 100, hops: 1, scale: 10, min_aim: 1, max_aim: 1000, tolerance: 80 };
        assert_eq!(r.resolve(400).landing, Landing::Pearl, "40 is close enough to 37");
        let near = r.resolve(480);
        assert_eq!(near.landing, Landing::Past);
        assert!(near.near, "48 is a near miss");
        let far = r.resolve(800);
        assert!(!far.near, "80 is just a miss");
    }

    #[test]
    fn aiming_is_clamped_and_ignored_mid_air() {
        let s = HopSession::new(count(4));
        let s = hop_reducer(s, HopAction::Aim { at: 99 });
        assert_eq!(s.aim, 7);
        let s = hop_reducer(s, HopAction::Aim { at: 0 });
        assert_eq!(s.aim, 1);
        let s = hop_reducer(hop_reducer(s, HopAction::Aim { at: 4 }), HopAction::Toss);
        let s = hop_reducer(s, HopAction::Aim { at: 2 });
        assert_eq!(s.aim, 4, "no steering in flight");
        let again = hop_reducer(s.clone(), HopAction::Toss);
        assert_eq!(again.tosses, 1, "can't toss her while she's already flying");
    }

    #[test]
    fn reset_forgets_the_demo_toss() {
        let s = land(toss_at(HopSession::new(count(4)), 6));
        let s = hop_reducer(s, HopAction::Reset);
        assert_eq!(s.tosses, 0);
        assert_eq!(s.phase, HopPhase::Aiming);
        assert!(s.toss.is_none());
    }

    // ─── Properties, over many seeds and every band ───────

    fn rounds() -> impl Iterator<Item = (u8, u64, HopRound)> {
        (0..=8u8).flat_map(|band| {
            (0..60u64).map(move |seed| (band, seed, generate_round(band, &mut SmallRng::seed_from_u64(seed))))
        })
    }

    #[test]
    fn every_generated_round_is_solvable_with_the_offered_aims() {
        for (band, seed, r) in rounds() {
            assert!(r.min_aim >= 1 && r.min_aim <= r.max_aim, "band {band} seed {seed}: {r:?}");
            assert!(!r.winning_aims().is_empty(), "band {band} seed {seed}: no aim wins {r:?}");
            assert!(r.pearl_pos() < r.span_pos(), "band {band} seed {seed}: water past the pearl {r:?}");
            // And playing the winning aim through the reducer really wins.
            let aim = r.winning_aims()[0];
            let s = land(toss_at(HopSession::new(r.clone()), aim));
            assert_eq!(s.phase, HopPhase::Won, "band {band} seed {seed}");
        }
    }

    #[test]
    fn some_offered_aim_misses_so_there_is_something_to_think_about() {
        for (band, seed, r) in rounds() {
            let misses = (r.min_aim..=r.max_aim).filter(|&a| r.resolve(a).landing != Landing::Pearl).count();
            assert!(misses > 0, "band {band} seed {seed}: every aim wins {r:?}");
        }
    }

    #[test]
    fn a_miss_is_never_a_fail_state() {
        for (band, seed, r) in rounds() {
            for aim in (r.min_aim..=r.max_aim).step_by(r.aim_step() as usize) {
                let s = settle(toss_at(HopSession::new(r.clone()), aim));
                match s.toss.as_ref().unwrap().landing {
                    Landing::Pearl => assert_eq!(s.phase, HopPhase::Won),
                    _ => {
                        assert_eq!(s.phase, HopPhase::Aiming, "band {band} seed {seed} aim {aim}: back to the rock");
                        // ...and the winning aim still works from here.
                        let w = r.winning_aims()[0];
                        assert_eq!(land(toss_at(s, w)).phase, HopPhase::Won, "band {band} seed {seed}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_outcome_never_depends_on_time() {
        for (_, _, r) in rounds().step_by(7) {
            for aim in [r.min_aim, (r.min_aim + r.max_aim) / 2, r.max_aim] {
                let mut outcomes = Vec::new();
                for dts in [[0.001f32, 0.001], [1.0 / 60.0, 1.0 / 30.0], [0.3, 5.0], [10.0, 0.0]] {
                    let mut s = toss_at(HopSession::new(r.clone()), aim);
                    let decided = s.toss.clone();
                    for i in 0..20_000 {
                        if matches!(s.phase, HopPhase::Landed | HopPhase::Won) {
                            break;
                        }
                        s = hop_reducer(s, HopAction::Tick { dt: dts[i % 2] });
                    }
                    assert_eq!(s.toss, decided, "ticking never changes where she lands");
                    outcomes.push((s.phase, s.toss.clone()));
                }
                assert!(outcomes.windows(2).all(|w| w[0] == w[1]), "same toss, any frame rate: {outcomes:?}");
            }
        }
    }

    #[test]
    fn nonsense_ticks_do_nothing() {
        let s = toss_at(HopSession::new(count(4)), 4);
        for dt in [f32::NAN, -3.0, f32::NEG_INFINITY] {
            let t = hop_reducer(s.clone(), HopAction::Tick { dt });
            assert_eq!(t.clock, 0.0);
        }
    }

    #[test]
    fn bands_pick_the_stages_in_order() {
        let stage = |b| HopStage::for_band(b);
        assert_eq!(stage(1), HopStage::Count);
        assert_eq!(stage(2), HopStage::SkipCount);
        assert_eq!(stage(3), HopStage::Hops);
        assert_eq!(stage(4), HopStage::Estimate);
        assert_eq!(stage(10), HopStage::Estimate);
        for (band, seed, r) in rounds() {
            assert_eq!(r.stage, HopStage::for_band(band), "band {band} seed {seed}");
            if r.stage == HopStage::Count {
                assert!((3..=6).contains(&r.pearl), "the youngest get 3–6 stones: {r:?}");
            }
            if r.stage == HopStage::SkipCount {
                assert!(r.min_aim >= 2, "a hop of 1 is just walking");
                assert!(r.max_aim < r.pearl, "one giant hop isn't skip counting");
            }
        }
    }

    #[test]
    fn the_same_seed_makes_the_same_round() {
        for band in 1..=6 {
            let a = generate_round(band, &mut SmallRng::seed_from_u64(3));
            let b = generate_round(band, &mut SmallRng::seed_from_u64(3));
            assert_eq!(a, b);
        }
    }
}
