//! Shelly's Pearl Hop: the slingshot minigame she runs. The rules are the
//! domain's `logic::pearl_hop` reducer; this file turns taps, drags and keys
//! into its actions, counts out loud, pays the pearl, and logs every toss for
//! the adaptive system (never shown to the kid).

use super::*;
use robot_buddy_domain::learning::challenge_generator::{classify_division, classify_multiplication};
use robot_buddy_domain::logic::pearl_hop::{
    generate_round, hop_reducer, HopAction, HopPhase, HopRound, HopSession, HopStage, Landing, Toss,
};
use robot_buddy_domain::types::SubSkill;
use crate::ui::pearl_hop::{HopArt, HopInput, HopView};

/// The key Shelly's first-time show-off is remembered under.
pub const PEARL_HOP_DEMO: &str = "pearl_hop";

/// A live Pearl Hop. Lives only while `GameState::PearlHop` is up.
pub struct ActivePearlHop {
    pub session: HopSession,
    /// Pearls one find is worth here, before the bonuses: the trench Shelly
    /// pays double, which is part of what the dive down is for.
    pub base: u32,
    /// The pointer while Shelly is being pulled back (the kid's, or the
    /// demo's pretend one).
    pub pull_to: Option<(f32, f32)>,
    /// Shelly's first-ever show-off, while it's playing.
    pub demo: Option<HopDemo>,
    /// Shelly's line, for a grown-up reading along. The kid hears it.
    pub caption: String,
    /// Landings already counted out loud for the toss in the air.
    counted: usize,
    /// When the kid started lining up this toss (silent response time).
    aim_started: f32,
    /// How long the toss in the air took to line up, in ms.
    toss_ms: f64,
}

/// Shelly's first-time demo: she flings herself (and misses, on purpose),
/// then a hand shows the kid the pull.
pub struct HopDemo {
    pub step: DemoStep,
    pub clock: f32,
    /// The aim she misses with.
    miss_aim: u16,
    /// The aim the hand pulls to.
    show_aim: u16,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DemoStep {
    /// Shelly scrunches herself back, counting, and lets go.
    SelfToss,
    /// Her toss plays out (it misses).
    Watching,
    /// A hand grabs her and shows the pull, without letting go.
    HandShows,
}

const SELF_PULL_SECS: f32 = 1.6;
const SELF_TOSS_AT: f32 = 2.0;
const HAND_SECS: f32 = 3.4;

/// What a find is worth where this Shelly lives.
pub(super) fn pearl_hop_base(home_map: &str) -> u32 {
    if home_map == "trench" { 2 } else { 1 }
}

/// Shelly's opener: where the pearl is and, past the first stage, the one
/// rule of the round. Spoken, and captioned for a reading grown-up.
fn opening_line(round: &HopRound) -> String {
    let p = round.pearl;
    match round.stage {
        HopStage::Count => format!("My pearl is on stone {p}! Fling me there!"),
        HopStage::SkipCount => format!("I always hop the same size! Get me to my pearl on {p}!"),
        HopStage::Hops => format!("Get me to my pearl on {p} in {} hops!", round.hops),
        HopStage::Estimate => format!("My pearl sank at {p}! The far rock is {}.", round.span),
    }
}

/// An aim the demo misses with on purpose: past the pearl, so she gets to
/// splash. Falls back to any missing aim.
fn demo_miss_aim(round: &HopRound) -> u16 {
    let wins = round.winning_aims();
    let first = wins.first().copied().unwrap_or(round.min_aim);
    let over = (first..=round.max_aim)
        .step_by(round.aim_step().max(1) as usize)
        .find(|&a| round.resolve(a).landing == Landing::Past);
    over.or_else(|| (round.min_aim..=round.max_aim).find(|&a| round.resolve(a).landing != Landing::Pearl))
        .unwrap_or(round.max_aim)
}

impl Game {
    /// Open Pearl Hop for a Shelly whose finds are worth `base` pearls.
    pub(super) fn start_pearl_hop(&mut self, base: u32) {
        let round = generate_round(self.profile.math_band, &mut self.rng);
        self.events.push(GameEvent::PearlHopStarted { stage: round.stage, pearl: round.pearl });
        let line = opening_line(&round);
        audio::tts::speak("Shelly", &line);
        let demo = (!self.seen_demos.contains(PEARL_HOP_DEMO)).then(|| {
            let miss_aim = demo_miss_aim(&round);
            let show_aim = round.min_aim + (round.max_aim - round.min_aim) / 3;
            HopDemo { step: DemoStep::SelfToss, clock: 0.0, miss_aim, show_aim }
        });
        self.active_pearl_hop = Some(ActivePearlHop {
            session: HopSession::new(round),
            base,
            pull_to: None,
            demo,
            caption: line,
            counted: 0,
            aim_started: self.game_time,
            toss_ms: 0.0,
        });
        self.set_state(GameState::PearlHop);
    }

    /// The panel's layout for this frame (step and render both use it).
    pub fn pearl_hop_layout(&self, screen: (f32, f32)) -> Option<ui::pearl_hop::PearlHopLayout> {
        let a = self.active_pearl_hop.as_ref()?;
        Some(ui::pearl_hop::layout(&HopView { session: &a.session, pearls: self.pearls, caption: &a.caption }, screen))
    }

    /// One frame of Pearl Hop.
    pub(super) fn step_pearl_hop(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        let Some(l) = self.pearl_hop_layout(screen) else { return };
        let a = self.active_pearl_hop.as_ref().unwrap();
        let (mx, my) = input.mouse_pos;
        let tap = if input.mouse_clicked { ui::pearl_hop::handle_click(mx, my, &l) } else { None };
        let key = ui::pearl_hop::handle_key(input, &a.session);

        // Leaving is always one tap (or ESC) away, whatever she's doing.
        if matches!(tap, Some(HopInput::Leave)) || matches!(key, Some(HopInput::Leave)) {
            self.leave_pearl_hop();
            return;
        }

        if a.demo.is_some() {
            // Any tap or key skips the show-off.
            if input.mouse_clicked || input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) {
                self.finish_pearl_hop_demo();
            } else {
                self.step_pearl_hop_demo(dt, &l);
            }
            return;
        }

        if a.session.phase == HopPhase::Won {
            if matches!(tap, Some(HopInput::Again)) || matches!(key, Some(HopInput::Again)) {
                let base = a.base;
                self.start_pearl_hop(base);
                return;
            }
            self.tick_pearl_hop(dt);
            return;
        }

        if a.session.phase == HopPhase::Aiming {
            self.aim_pearl_hop(input, key, &l);
        }
        self.tick_pearl_hop(dt);
    }

    /// Pull, nudge, or let go.
    fn aim_pearl_hop(&mut self, input: &FrameInput, key: Option<HopInput>, l: &ui::pearl_hop::PearlHopLayout) {
        let g = l.scene;
        let pos = input.mouse_pos;
        let pulling = self.active_pearl_hop.as_ref().unwrap().pull_to.is_some();

        if !pulling && input.mouse_clicked && g.grabs(pos.0, pos.1) {
            self.active_pearl_hop.as_mut().unwrap().pull_to = Some(pos);
            return;
        }
        if pulling {
            let aim = g.aim_for_pointer(pos);
            if input.mouse_down && !input.mouse_released {
                self.active_pearl_hop.as_mut().unwrap().pull_to = Some(pos);
                if let Some(aim) = aim {
                    self.set_pearl_hop_aim(aim);
                }
                return;
            }
            // Let go: a real pull tosses her; a tap on her does nothing.
            self.active_pearl_hop.as_mut().unwrap().pull_to = None;
            if let Some(aim) = aim {
                self.set_pearl_hop_aim(aim);
                self.toss_shelly();
            }
            return;
        }

        match key {
            Some(HopInput::Nudge(d)) => {
                let s = &self.active_pearl_hop.as_ref().unwrap().session;
                let step = s.round.aim_step() as i32;
                let next = (s.aim as i32 + d * step).clamp(s.round.min_aim as i32, s.round.max_aim as i32) as u16;
                self.set_pearl_hop_aim(next);
            }
            Some(HopInput::Toss) => self.toss_shelly(),
            _ => {}
        }
    }

    /// Move the aim; on the stones, Shelly counts the hop out loud as it
    /// snaps from stone to stone ("one… two… three…").
    fn set_pearl_hop_aim(&mut self, aim: u16) {
        let a = self.active_pearl_hop.as_mut().unwrap();
        let before = a.session.aim;
        a.session = hop_reducer(a.session.clone(), HopAction::Aim { at: aim });
        let s = &a.session;
        if s.aim != before && s.round.stage != HopStage::Estimate {
            audio::tts::speak("Shelly", &(s.aim / s.round.scale).to_string());
        }
    }

    fn toss_shelly(&mut self) {
        let now = self.game_time;
        let a = self.active_pearl_hop.as_mut().unwrap();
        a.session = hop_reducer(a.session.clone(), HopAction::Toss);
        a.counted = 0;
        a.toss_ms = ((now - a.aim_started).max(0.0) * 1000.0) as f64;
        audio::tts::speak("Shelly", "Wheee!");
    }

    /// Let the animation clock run, counting landings aloud, and react when
    /// she comes down.
    fn tick_pearl_hop(&mut self, dt: f32) {
        let now = self.game_time;
        let a = self.active_pearl_hop.as_mut().unwrap();
        let before = a.session.phase;
        a.session = hop_reducer(a.session.clone(), HopAction::Tick { dt });
        let s = &a.session;

        // Skip counting out loud: each touchdown's number as she lands on it.
        if let (Some(t), HopStage::SkipCount | HopStage::Hops) = (s.toss.as_ref(), s.round.stage) {
            let reached = match s.phase {
                HopPhase::Flying => s.flight().map_or(0, |(hop, _)| hop),
                HopPhase::Landed | HopPhase::Won => t.landings.len(),
                HopPhase::Aiming => a.counted,
            };
            while a.counted < reached {
                audio::tts::speak("Shelly", &(t.landings[a.counted] / s.round.scale).to_string());
                a.counted += 1;
            }
        }

        let after = s.phase;
        if before == HopPhase::Flying && after != HopPhase::Flying {
            self.pearl_hop_landed();
        }
        if after == HopPhase::Aiming && before != HopPhase::Aiming {
            let a = self.active_pearl_hop.as_mut().unwrap();
            a.aim_started = now;
        }
    }

    /// She came down. Log the toss; pay for the pearl, or react to the miss.
    fn pearl_hop_landed(&mut self) {
        let (stage, toss, tosses, ms, base, clean) = {
            let a = self.active_pearl_hop.as_ref().unwrap();
            let s = &a.session;
            (s.round.stage, s.toss.clone().unwrap(), s.tosses, a.toss_ms, a.base, s.was_clean())
        };
        let round = self.active_pearl_hop.as_ref().unwrap().session.round.clone();
        self.log_pearl_hop_toss(&round, &toss, tosses, ms);

        let line = match toss.landing {
            Landing::Pearl => {
                // Base for this Shelly's pearl, +1 for getting it first
                // toss, +1 more with Hermie's Diving Net.
                let payout = domain_shop::pearl_payout(base, 1, clean, &self.upgrades);
                let paid = self.award_pearls(payout);
                self.events.push(GameEvent::PearlHopWon { stage, tosses, pearls: payout.total() });
                audio::tts::speak("Shelly", "You found my pearl!");
                self.persist();
                format!("You found my pearl!  {paid}")
            }
            _ if toss.near => {
                audio::tts::speak("Shelly", "Ooh, so close!");
                "Ooh, SO close! Try again!".to_string()
            }
            Landing::Past => {
                audio::tts::speak("Shelly", "Sploosh! Just past it!");
                "Sploosh! Just past it. Try again!".to_string()
            }
            Landing::Short => {
                if stage == HopStage::Estimate {
                    audio::tts::speak("Shelly", "Sploosh! Not far enough!");
                    "Sploosh! Not quite that far. Try again!".to_string()
                } else {
                    audio::tts::speak("Shelly", "Boing! Not there yet!");
                    "Boing! Not there yet. Try again!".to_string()
                }
            }
        };
        self.active_pearl_hop.as_mut().unwrap().caption = line;
    }

    /// Stealth assessment: every toss is a data point for the adaptive
    /// system and the parent log. The kid never sees any of it.
    ///
    /// Counting → Add/AddSingle (counting on from zero), concrete stones.
    /// Skip counting → Multiply (hop × hops), on the number path.
    /// K hops → Divide (the pearl split into K equal hops).
    /// Estimation has no `Operation`; it goes to the parent log only, so it
    /// can't move the arithmetic band.
    fn log_pearl_hop_toss(&mut self, round: &HopRound, toss: &Toss, tosses: u8, ms: f64) {
        let correct = toss.landing == Landing::Pearl;
        let band = self.profile.math_band;
        let hops = toss.landings.len() as i32;
        let aim = (toss.aim / round.scale) as i32;
        let pearl = round.pearl as i32;
        let (op_name, assessed): (&str, Option<(Operation, SubSkill, CraStage)>) = match round.stage {
            HopStage::Count => ("count", Some((Operation::Add, SubSkill::AddSingle, CraStage::Concrete))),
            HopStage::SkipCount => (
                "skip_count",
                Some((Operation::Multiply, classify_multiplication(aim.max(1), hops.max(1)), CraStage::Representational)),
            ),
            HopStage::Hops => (
                "divide",
                Some((Operation::Divide, classify_division(pearl, round.hops.max(1) as i32), CraStage::Representational)),
            ),
            HopStage::Estimate => ("estimate", None),
        };
        if let Some((operation, sub_skill, cra)) = assessed {
            self.profile = learner_reducer(self.profile.clone(), LearnerEvent::PuzzleAttempted {
                correct,
                operation,
                sub_skill: Some(sub_skill),
                band,
                center_band: None,
                response_time_ms: Some(ms),
                hint_used: false,
                told_me: false,
                cra_level_shown: Some(cra),
                timestamp: Some(self.game_time as f64 * 1000.0),
            });
        }
        self.session_log.record_challenge(session::ChallengeRecord {
            question: format!("Pearl Hop ({op_name}): pearl at {pearl}"),
            correct_answer: pearl,
            player_answer: Some((toss.end() / round.scale) as i32),
            correct,
            operation: op_name.to_string(),
            band,
            sampled_band: band,
            hint_used: false,
            told_me: false,
            attempts: tosses as u32,
            source: "shelly".to_string(),
            play_time_at_event: self.play_time,
        });
        self.events.push(GameEvent::PearlHopTossed {
            stage: round.stage,
            aim: toss.aim,
            landed: toss.end(),
            hit: correct,
        });
    }

    fn leave_pearl_hop(&mut self) {
        self.active_pearl_hop = None;
        self.events.push(GameEvent::PearlHopLeft);
        self.set_state(GameState::Playing);
    }

    // ─── The first-time demo ────────────────────────────

    fn step_pearl_hop_demo(&mut self, dt: f32, l: &ui::pearl_hop::PearlHopLayout) {
        let g = l.scene;
        let (hx, hy) = g.home();
        let (step, clock, miss_aim, show_aim) = {
            let d = self.active_pearl_hop.as_mut().unwrap().demo.as_mut().unwrap();
            d.clock += dt;
            (d.step, d.clock, d.miss_aim, d.show_aim)
        };
        let toward = |aim: u16, u: f32| {
            let t = g.drag_point_for(aim);
            (hx + (t.0 - hx) * u, hy + (t.1 - hy) * u)
        };
        match step {
            DemoStep::SelfToss => {
                // She scrunches herself back, counting as she goes, then lets go.
                if clock >= SELF_TOSS_AT {
                    self.active_pearl_hop.as_mut().unwrap().pull_to = None;
                    self.set_pearl_hop_aim(miss_aim);
                    self.toss_shelly();
                    let d = self.active_pearl_hop.as_mut().unwrap().demo.as_mut().unwrap();
                    d.step = DemoStep::Watching;
                    d.clock = 0.0;
                } else {
                    let p = toward(miss_aim, (clock / SELF_PULL_SECS).clamp(0.0, 1.0));
                    self.active_pearl_hop.as_mut().unwrap().pull_to = Some(p);
                    if let Some(aim) = g.aim_for_pointer(p) {
                        self.set_pearl_hop_aim(aim);
                    }
                }
            }
            DemoStep::Watching => {
                let back = {
                    let s = &self.active_pearl_hop.as_ref().unwrap().session;
                    s.phase == HopPhase::Aiming && s.toss.is_some()
                };
                if !back {
                    self.tick_pearl_hop(dt);
                    return;
                }
                let a = self.active_pearl_hop.as_mut().unwrap();
                let d = a.demo.as_mut().unwrap();
                d.step = DemoStep::HandShows;
                d.clock = 0.0;
                a.caption = "Your turn! Pull me back and let go!".to_string();
                // Aim back at rest so the hand's pull starts from nothing.
                let rest = a.session.round.rest_aim();
                a.session = hop_reducer(a.session.clone(), HopAction::Aim { at: rest });
                audio::tts::speak("Shelly", "Your turn! Pull me back, and let go!");
            }
            DemoStep::HandShows => {
                let (u, _, _) = hand_timeline(clock);
                let pull = (u > 0.0).then(|| toward(show_aim, u));
                self.active_pearl_hop.as_mut().unwrap().pull_to = pull;
                if let Some(aim) = pull.and_then(|p| g.aim_for_pointer(p)) {
                    self.set_pearl_hop_aim(aim);
                }
                if clock >= HAND_SECS {
                    self.finish_pearl_hop_demo();
                }
            }
        }
    }

    /// Demo over (or skipped): forget her show-off toss so the kid's first
    /// real toss is their first try, and remember not to show it again.
    fn finish_pearl_hop_demo(&mut self) {
        let a = self.active_pearl_hop.as_mut().unwrap();
        a.demo = None;
        a.pull_to = None;
        a.session = hop_reducer(a.session.clone(), HopAction::Reset);
        a.counted = 0;
        a.aim_started = self.game_time;
        a.caption = opening_line(&a.session.round);
        if self.seen_demos.insert(PEARL_HOP_DEMO.to_string()) {
            self.events.push(GameEvent::PearlHopDemoSeen);
            self.persist();
        }
    }

    /// The art extras for this frame: the live pull, and the demo's hand.
    pub(super) fn pearl_hop_art(&self, l: &ui::pearl_hop::PearlHopLayout) -> HopArt {
        let Some(a) = self.active_pearl_hop.as_ref() else { return HopArt::default() };
        let mut art = HopArt { pull_to: a.pull_to, hand: None, hand_alpha: 0.0 };
        if let Some(d) = a.demo.as_ref().filter(|d| d.step == DemoStep::HandShows) {
            let (u, pressed, alpha) = hand_timeline(d.clock);
            let (hx, hy) = l.scene.home();
            let target = l.scene.drag_point_for(d.show_aim);
            let lift = if d.clock > 2.6 { (d.clock - 2.6) * 40.0 } else { 0.0 };
            art.hand = Some((hx + (target.0 - hx) * u, hy + (target.1 - hy) * u - lift, pressed));
            art.hand_alpha = alpha;
        }
        art
    }
}

/// The teaching hand's script: (pull 0..1, pressing, alpha) at `clock`.
/// Fade in on Shelly, press, drag back, hold, lift off without letting her
/// fly (the letting-go is the kid's to discover), fade.
fn hand_timeline(clock: f32) -> (f32, bool, f32) {
    let alpha = if clock < 0.4 { clock / 0.4 } else if clock > 2.8 { (1.0 - (clock - 2.8) / 0.6).max(0.0) } else { 1.0 };
    let pressed = (0.5..2.6).contains(&clock);
    let u = if clock < 0.6 { 0.0 } else { ((clock - 0.6) / 1.3).clamp(0.0, 1.0) };
    let u = if clock > 2.6 { 0.0 } else { u };
    (u, pressed, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::rand::rngs::SmallRng;

    #[test]
    fn the_demo_always_misses_on_purpose() {
        for band in 0..=7u8 {
            for seed in 0..40u64 {
                let r = generate_round(band, &mut SmallRng::seed_from_u64(seed));
                let aim = demo_miss_aim(&r);
                assert_ne!(r.resolve(aim).landing, Landing::Pearl, "band {band} seed {seed}: {r:?}");
            }
        }
    }

    #[test]
    fn the_deep_shelly_pays_double() {
        assert_eq!(pearl_hop_base("trench"), 2);
        assert_eq!(pearl_hop_base("reef"), 1);
    }
}
