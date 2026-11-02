//! Sockbot's Sock Shop — robots line up with bare feet, and the kid hands
//! over socks from Sockbot's drawer. Pure logic, no rendering.
//!
//! Socks only come in pairs (rolled up in balls), and robots come with any
//! number of feet. So the one question every round asks is "how many pairs
//! does this order need?", and the answer IS the pairing maths:
//!
//! | stage   | the order                                   | the maths                     |
//! |---------|---------------------------------------------|-------------------------------|
//! | `Feet`  | one robot, 2–5 feet; each pair goes straight onto two feet | 1:1, counting in twos, odd and even |
//! | `Group` | a line of 2–4 robots, every foot on show; pairs pile up on the counter until they're sent | doubling, halving, odd and even |
//! | `Order` | an order slip ("6 spider-bots"), one robot shows its feet  | multiplying, halving          |
//!
//! An odd number of feet always leaves one sock of the last pair over: the
//! lonely sock. Sockbot keeps it, delighted ("for my collection!"). That's
//! the joke he's built around, and it's what makes odd and even something
//! you see rather than something you're told.
//!
//! Nothing here can fail. Too few pairs and a robot shivers on a bare foot;
//! too many and the spares bounce back onto the counter. Either way the
//! socks come back and the kid tries again, with the count they had. Tries
//! are unlimited and untimed; `tries` only exists so the game can pay a
//! first-try bonus and the adaptive system can read the attempt.
//!
//! Public surface mirrors the other logic modules:
//!   - `generate_round(band, &mut impl Rng)` → a round solvable by construction
//!   - `SockSession::new(round)` → fresh session
//!   - `sock_reducer(session, action)` → new session

use rand::Rng;
use serde::{Deserialize, Serialize};

/// Seconds for one sock to fly onto its foot when an order is sent. Big
/// orders go faster so a hand-over never drags past `MAX_DRESS_SECS`.
pub const SOCK_SECS: f32 = 0.32;
pub const MAX_DRESS_SECS: f32 = 4.0;
/// The Feet stage's hand-over: the socks are on already, so just a beat.
pub const LIVE_DRESS_SECS: f32 = 0.8;
/// After the socks are on: a shiver on a bare foot, or the spares bouncing
/// back. Then the socks come home and the kid tries again.
pub const REACT_SECS: f32 = 2.2;
/// The win: the robots dance off in their new socks.
pub const WIN_SECS: f32 = 2.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SockStage {
    /// One robot. Every pair added goes straight onto two of its feet, so the
    /// kid can watch the feet fill up.
    Feet,
    /// A line of the same robot. The feet are all on show, but pairs pile up
    /// on the counter until the order is sent.
    Group,
    /// An order slip: "N of this robot". One robot stands in for the lot.
    Order,
}

impl SockStage {
    /// Which stage a learner at `band` plays.
    pub fn for_band(band: u8) -> SockStage {
        match band {
            0 | 1 => SockStage::Feet,
            2 | 3 => SockStage::Group,
            _ => SockStage::Order,
        }
    }

    /// Does each pair go straight onto the robot as it's added?
    pub fn live(self) -> bool {
        self == SockStage::Feet
    }
}

/// The customers. Each is a robot body with a fixed number of feet; the
/// game draws them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Walker {
    /// Two feet, like you.
    Biped,
    /// Three feet. Always leaves a lonely sock.
    Tripod,
    /// Four paws.
    DogBot,
    /// Five feet, one on each arm.
    StarBot,
    /// Six legs.
    BugBot,
    /// Eight legs.
    SpiderBot,
    /// Ten little feet.
    CentiBot,
}

impl Walker {
    pub const ALL: [Walker; 7] = [
        Walker::Biped, Walker::Tripod, Walker::DogBot, Walker::StarBot,
        Walker::BugBot, Walker::SpiderBot, Walker::CentiBot,
    ];

    pub fn feet(self) -> u16 {
        match self {
            Walker::Biped => 2,
            Walker::Tripod => 3,
            Walker::DogBot => 4,
            Walker::StarBot => 5,
            Walker::BugBot => 6,
            Walker::SpiderBot => 8,
            Walker::CentiBot => 10,
        }
    }

    /// What Sockbot calls one, for his lines ("3 tripods").
    pub fn name(self, n: u16) -> &'static str {
        let one = n == 1;
        match self {
            Walker::Biped => if one { "two-foot bot" } else { "two-foot bots" },
            Walker::Tripod => if one { "tripod" } else { "tripods" },
            Walker::DogBot => if one { "dog-bot" } else { "dog-bots" },
            Walker::StarBot => if one { "star-bot" } else { "star-bots" },
            Walker::BugBot => if one { "bug-bot" } else { "bug-bots" },
            Walker::SpiderBot => if one { "spider-bot" } else { "spider-bots" },
            Walker::CentiBot => if one { "centi-bot" } else { "centi-bots" },
        }
    }

    fn with_feet(feet: u16) -> Walker {
        Walker::ALL.into_iter().find(|w| w.feet() == feet).unwrap_or(Walker::Biped)
    }
}

/// One order's worth of puzzle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SockRound {
    pub stage: SockStage,
    pub walker: Walker,
    /// How many of `walker` are in the order.
    pub robots: u16,
    /// The most pairs the drawer holds. The band's, not the round's, so its
    /// size never hints at the answer.
    pub max_pairs: u16,
}

impl SockRound {
    /// Every foot in the order.
    pub fn feet(&self) -> u16 {
        self.robots * self.walker.feet()
    }

    /// The pairs that sock every foot with none spare: half the feet,
    /// rounded up (an odd foot still needs a whole pair).
    pub fn pairs_needed(&self) -> u16 {
        self.feet().div_ceil(2)
    }

    /// Will the right order leave one lonely sock over?
    pub fn lonely_sock(&self) -> bool {
        self.feet() % 2 == 1
    }

    /// What handing over `pairs` does. Pure function of the count.
    pub fn fit(&self, pairs: u16) -> Fit {
        let need = self.pairs_needed();
        match pairs.cmp(&need) {
            std::cmp::Ordering::Equal => Fit::Snug,
            // Every missing pair is two bare feet, except the last one of an
            // odd order, which covers just the one.
            std::cmp::Ordering::Less => Fit::Short { bare: self.feet() - (pairs * 2).min(self.feet()) },
            std::cmp::Ordering::Greater => Fit::Spare { pairs: pairs - need },
        }
    }

    /// The step buttons the drawer offers: ones always, tens once an order
    /// can run past twenty pairs.
    pub fn steps(&self) -> &'static [u16] {
        if self.max_pairs > 20 { &[1, 10] } else { &[1] }
    }
}

/// How an order fits, decided the moment it's sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Fit {
    /// Every foot socked, nothing spare (bar the lonely sock of an odd order).
    Snug,
    /// Not enough: `bare` feet go without.
    Short { bare: u16 },
    /// Too many: `pairs` whole pairs bounce back.
    Spare { pairs: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SockPhase {
    /// Picking how many pairs to hand over.
    Choosing,
    /// The socks are flying onto the feet.
    Dressing,
    /// Shivering on a bare foot, or catching the spares. Then back to
    /// Choosing. Never an ending.
    Reacting,
    /// Every foot socked. The round is over until it's reset.
    Won,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SockSession {
    pub round: SockRound,
    /// The pairs on the counter (or, at the Feet stage, on the robot). Kept
    /// between tries, so after a miss the kid adjusts from where they were.
    pub pairs: u16,
    pub phase: SockPhase,
    /// Orders sent this round. First-try success is the clean bonus.
    pub tries: u8,
    /// How the order in flight (or just sent) fits.
    pub sent: Option<Fit>,
    /// Seconds into the current phase. Animation only.
    pub clock: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SockAction {
    /// Add (or, negative, take back) pairs. Clamped to the drawer; ignored
    /// unless choosing.
    Add { n: i32 },
    /// Hand the order over. Ignored unless choosing, or with nothing on the
    /// counter.
    Send,
    /// Let `dt` seconds of animation pass.
    Tick { dt: f32 },
    /// Back to a fresh start on the same round.
    Reset,
}

impl SockSession {
    pub fn new(round: SockRound) -> Self {
        SockSession { round, pairs: 0, phase: SockPhase::Choosing, tries: 0, sent: None, clock: 0.0 }
    }

    /// Won on the first try.
    pub fn was_clean(&self) -> bool {
        self.phase == SockPhase::Won && self.tries == 1
    }

    /// Socks that fly when the order is sent: two per pair, but never more
    /// than there are feet (the spares stay in their balls).
    pub fn socks_flying(&self) -> u16 {
        (self.pairs * 2).min(self.round.feet())
    }

    /// Seconds each sock takes to fly on.
    pub fn sock_secs(&self) -> f32 {
        SOCK_SECS.min(MAX_DRESS_SECS / self.socks_flying().max(1) as f32)
    }

    /// How long the hand-over takes. At the Feet stage the socks are on
    /// already, so it's just a beat of wiggling toes.
    pub fn dress_secs(&self) -> f32 {
        if self.round.stage.live() {
            LIVE_DRESS_SECS
        } else {
            self.socks_flying() as f32 * self.sock_secs()
        }
    }

    /// Feet with a sock on right now. While choosing at the Feet stage the
    /// pairs are already on; while dressing they go on one by one; after
    /// that, every sock that flew is on.
    pub fn socked(&self) -> u16 {
        let feet = self.round.feet();
        match self.phase {
            SockPhase::Choosing if self.round.stage.live() => (self.pairs * 2).min(feet),
            SockPhase::Choosing => 0,
            SockPhase::Dressing if self.round.stage.live() => self.socks_flying(),
            SockPhase::Dressing => ((self.clock / self.sock_secs()).floor() as u16).min(self.socks_flying()),
            SockPhase::Reacting | SockPhase::Won => self.socks_flying(),
        }
    }

    /// The order's lonely sock is out: an odd order, every foot socked.
    pub fn shows_lonely_sock(&self) -> bool {
        self.round.lonely_sock() && self.socked() == self.round.feet()
            && (self.round.stage.live() || matches!(self.phase, SockPhase::Reacting | SockPhase::Won))
    }

    /// The win's celebration is over: time to offer "Next!".
    pub fn win_done(&self) -> bool {
        self.phase == SockPhase::Won && self.clock >= WIN_SECS
    }
}

pub fn sock_reducer(state: SockSession, action: SockAction) -> SockSession {
    let mut s = state;
    match action {
        SockAction::Add { n } => {
            if s.phase == SockPhase::Choosing {
                s.pairs = (s.pairs as i32 + n).clamp(0, s.round.max_pairs as i32) as u16;
            }
        }
        SockAction::Send => {
            if s.phase == SockPhase::Choosing && s.pairs > 0 {
                s.sent = Some(s.round.fit(s.pairs));
                s.tries = s.tries.saturating_add(1);
                s.phase = SockPhase::Dressing;
                s.clock = 0.0;
            }
        }
        SockAction::Tick { dt } => {
            // A negative or NaN dt is nonsense; treat it as no time passing.
            let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
            s.clock += dt;
            match s.phase {
                SockPhase::Dressing if s.clock >= s.dress_secs() => {
                    s.phase = if s.sent == Some(Fit::Snug) { SockPhase::Won } else { SockPhase::Reacting };
                    s.clock = 0.0;
                }
                SockPhase::Reacting if s.clock >= REACT_SECS => {
                    s.phase = SockPhase::Choosing;
                    s.clock = 0.0;
                }
                _ => {}
            }
        }
        SockAction::Reset => {
            s = SockSession::new(s.round);
        }
    }
    s
}

/// Build an order for `band`. Every round has an answer inside the drawer.
pub fn generate_round(band: u8, rng: &mut impl Rng) -> SockRound {
    match SockStage::for_band(band) {
        SockStage::Feet => {
            // One robot, 2 to 5 feet: odd ones included from the start, so
            // the lonely sock turns up early and often.
            let walker = Walker::with_feet(rng.gen_range(2..=5));
            SockRound { stage: SockStage::Feet, walker, robots: 1, max_pairs: 6 }
        }
        SockStage::Group => {
            // A short line of the same robot, every foot on show. Band 2
            // keeps to small feet; band 3 brings the six-legged bug-bots.
            let feet = if band <= 2 { rng.gen_range(2..=4) } else { [2, 3, 4, 5, 6][rng.gen_range(0..5)] };
            let robots = rng.gen_range(2..=if feet >= 5 { 3 } else { 4 });
            SockRound { stage: SockStage::Group, walker: Walker::with_feet(feet), robots, max_pairs: 20 }
        }
        SockStage::Order => {
            // An order slip. Even feet only, so the order is a clean
            // "N robots × pairs each"; bigger orders as the band grows.
            let (feet, robots, max_pairs): (&[u16], std::ops::RangeInclusive<u16>, u16) = match band {
                4 => (&[2, 4, 6], 3..=6, 20),
                5 => (&[4, 6, 8], 3..=8, 40),
                _ => (&[4, 6, 8, 10], 4..=10, 60),
            };
            let walker = Walker::with_feet(feet[rng.gen_range(0..feet.len())]);
            SockRound { stage: SockStage::Order, walker, robots: rng.gen_range(robots), max_pairs }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn round(stage: SockStage, walker: Walker, robots: u16) -> SockRound {
        SockRound { stage, walker, robots, max_pairs: 20 }
    }

    fn give(s: SockSession, pairs: i32) -> SockSession {
        let s = sock_reducer(s, SockAction::Add { n: pairs });
        sock_reducer(s, SockAction::Send)
    }

    fn play_out(mut s: SockSession) -> SockSession {
        for _ in 0..400 {
            if matches!(s.phase, SockPhase::Reacting | SockPhase::Won) {
                return s;
            }
            s = sock_reducer(s, SockAction::Tick { dt: 0.05 });
        }
        s
    }

    fn settle(mut s: SockSession) -> SockSession {
        for _ in 0..400 {
            if s.phase == SockPhase::Choosing || s.phase == SockPhase::Won {
                return s;
            }
            s = sock_reducer(s, SockAction::Tick { dt: 0.05 });
        }
        s
    }

    #[test]
    fn half_the_feet_rounded_up_socks_everyone() {
        let s = play_out(give(SockSession::new(round(SockStage::Group, Walker::DogBot, 3)), 6));
        assert_eq!(s.phase, SockPhase::Won);
        assert!(s.was_clean());
        assert_eq!(s.socked(), 12);
    }

    #[test]
    fn an_odd_number_of_feet_needs_a_whole_extra_pair_and_leaves_a_lonely_sock() {
        let r = round(SockStage::Group, Walker::Tripod, 3);
        assert_eq!(r.feet(), 9);
        assert_eq!(r.pairs_needed(), 5);
        assert!(r.lonely_sock());
        let s = play_out(give(SockSession::new(r.clone()), 5));
        assert_eq!(s.phase, SockPhase::Won);
        assert!(s.shows_lonely_sock(), "Sockbot gets his lonely sock");
        // Four pairs is one bare foot short.
        assert_eq!(r.fit(4), Fit::Short { bare: 1 });
    }

    #[test]
    fn too_few_pairs_leaves_bare_feet_then_the_kid_tries_again_from_there() {
        let s = play_out(give(SockSession::new(round(SockStage::Group, Walker::DogBot, 2)), 3));
        assert_eq!(s.phase, SockPhase::Reacting, "a miss is a beat, not an ending");
        assert_eq!(s.sent, Some(Fit::Short { bare: 2 }));
        let s = settle(s);
        assert_eq!(s.phase, SockPhase::Choosing);
        assert_eq!(s.pairs, 3, "the count is remembered so the kid adjusts from it");
        let s = play_out(give(s, 1));
        assert_eq!(s.phase, SockPhase::Won);
        assert!(!s.was_clean(), "second try: socked, but no first-try bonus");
    }

    #[test]
    fn too_many_pairs_bounce_back_as_spares() {
        let r = round(SockStage::Group, Walker::Biped, 2);
        assert_eq!(r.fit(4), Fit::Spare { pairs: 2 });
        let s = play_out(give(SockSession::new(r), 4));
        assert_eq!(s.phase, SockPhase::Reacting);
        assert_eq!(s.socked(), 4, "only real feet get socks; the spares stay rolled up");
    }

    #[test]
    fn at_the_feet_stage_each_pair_goes_straight_on() {
        let s = SockSession::new(round(SockStage::Feet, Walker::StarBot, 1));
        let s = sock_reducer(s, SockAction::Add { n: 2 });
        assert_eq!(s.socked(), 4, "two pairs, four feet, while still choosing");
        assert!(!s.shows_lonely_sock());
        let s = sock_reducer(s, SockAction::Add { n: 1 });
        assert_eq!(s.socked(), 5);
        assert!(s.shows_lonely_sock(), "the fifth foot's pair has one sock over");
    }

    #[test]
    fn later_stages_keep_the_pairs_on_the_counter_until_sent() {
        let s = sock_reducer(SockSession::new(round(SockStage::Group, Walker::DogBot, 2)), SockAction::Add { n: 4 });
        assert_eq!(s.socked(), 0);
        let s = sock_reducer(s, SockAction::Send);
        let dt = s.sock_secs() * 3.0 + 0.01;
        let s = sock_reducer(s, SockAction::Tick { dt });
        assert_eq!(s.socked(), 3, "they go on one at a time");
    }

    #[test]
    fn nothing_on_the_counter_sends_nothing() {
        let s = sock_reducer(SockSession::new(round(SockStage::Group, Walker::DogBot, 2)), SockAction::Send);
        assert_eq!(s.phase, SockPhase::Choosing);
        assert_eq!(s.tries, 0);
    }

    #[test]
    fn the_drawer_has_a_bottom_and_a_top() {
        let s = sock_reducer(SockSession::new(round(SockStage::Group, Walker::DogBot, 2)), SockAction::Add { n: -3 });
        assert_eq!(s.pairs, 0);
        let s = sock_reducer(s, SockAction::Add { n: 99 });
        assert_eq!(s.pairs, 20);
    }

    #[test]
    fn choosing_is_frozen_while_the_socks_fly() {
        let s = give(SockSession::new(round(SockStage::Group, Walker::DogBot, 2)), 4);
        let s = sock_reducer(s, SockAction::Add { n: 1 });
        assert_eq!(s.pairs, 4);
    }

    #[test]
    fn a_big_order_still_dresses_quickly() {
        let r = SockRound { stage: SockStage::Order, walker: Walker::CentiBot, robots: 10, max_pairs: 60 };
        let s = give(SockSession::new(r), 50);
        assert!(s.dress_secs() <= MAX_DRESS_SECS + 0.001);
    }

    #[test]
    fn nonsense_time_is_no_time() {
        let s = give(SockSession::new(round(SockStage::Group, Walker::DogBot, 2)), 4);
        let s = sock_reducer(s, SockAction::Tick { dt: f32::NAN });
        let s = sock_reducer(s, SockAction::Tick { dt: -5.0 });
        assert_eq!(s.clock, 0.0);
    }

    #[test]
    fn stages_grow_with_the_band() {
        assert_eq!(SockStage::for_band(1), SockStage::Feet);
        assert_eq!(SockStage::for_band(2), SockStage::Group);
        assert_eq!(SockStage::for_band(3), SockStage::Group);
        assert_eq!(SockStage::for_band(4), SockStage::Order);
        assert_eq!(SockStage::for_band(9), SockStage::Order);
    }

    #[test]
    fn every_generated_order_fits_in_the_drawer() {
        for band in 0..=9u8 {
            for seed in 0..60u64 {
                let r = generate_round(band, &mut SmallRng::seed_from_u64(seed));
                assert_eq!(r.stage, SockStage::for_band(band));
                assert!(r.pairs_needed() >= 1, "band {band} seed {seed}: {r:?}");
                assert!(r.pairs_needed() <= r.max_pairs, "band {band} seed {seed}: {r:?}");
                if r.stage == SockStage::Order {
                    assert!(!r.lonely_sock(), "order slips are whole pairs: {r:?}");
                }
                if r.stage == SockStage::Feet {
                    assert_eq!(r.robots, 1);
                }
            }
        }
    }

    #[test]
    fn odd_orders_turn_up_early() {
        let odd = (0..100u64)
            .filter(|&seed| generate_round(1, &mut SmallRng::seed_from_u64(seed)).lonely_sock())
            .count();
        assert!(odd > 20, "the lonely sock should be a regular, not a rarity ({odd}/100)");
    }

    #[test]
    fn the_same_seed_makes_the_same_order() {
        let a = generate_round(5, &mut SmallRng::seed_from_u64(7));
        let b = generate_round(5, &mut SmallRng::seed_from_u64(7));
        assert_eq!(a, b);
    }
}
