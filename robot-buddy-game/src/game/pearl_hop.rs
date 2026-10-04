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
use crate::ui::pearl_hop::{HopArt, HopInput, HopView, ShellyOutfit, WIN_PEARL_ARRIVES};

/// The key Shelly's first-time show-off is remembered under.
pub const PEARL_HOP_DEMO: &str = "pearl_hop";

/// A live Pearl Hop. Lives only while `GameState::PearlHop` is up.
pub struct ActivePearlHop {
    pub session: HopSession,
    /// Pearls one find is worth here, before the bonuses: the trench Shelly
    /// pays double, which is part of what the dive down is for.
    pub base: u32,
    /// Shelly's wearer id: whose swag she has on (the same as in the world).
    pub host: String,
    /// The pearls just won, still flying to the purse: the purse shows them
    /// only once they land.
    in_flight: u32,
    /// Stones counted out loud so far in the current landing (counting stage).
    tally_spoken: u16,
    /// The reaction (or the cheer) has been said for the current landing.
    reacted: bool,
    /// The win's caption, held until the count reaches the pearl.
    won_caption: String,
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
    /// Wall clock (Unix secs) from the last frame's input, for the attempt log.
    now: f64,
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

/// What Shelly says to open a round: the number, and the one rule.
fn opening_line(round: &HopRound) -> String {
    let p = round.pearl;
    match round.stage {
        HopStage::Count => format!("{p}! My pearl is {p} stones away. Count them!"),
        HopStage::SkipCount => format!("I always hop the same size! My pearl is on {p}!"),
        HopStage::Hops => format!("My pearl is on {p}. Get me there in {} hops!", round.hops),
    }
}

/// The few words on screen for a grown-up reading along. The dots, the
/// bubbles and Shelly's voice carry the round for the kid.
fn opening_caption(round: &HopRound) -> String {
    match round.stage {
        HopStage::Count => "Find my pearl!".to_string(),
        HopStage::SkipCount | HopStage::Hops => format!("Hop to {}!", round.pearl),
    }
}

/// An aim the demo misses with on purpose: past the pearl, so she gets to
/// splash. Falls back to any missing aim.
fn demo_miss_aim(round: &HopRound) -> u16 {
    let wins = round.winning_aims();
    let first = wins.first().copied().unwrap_or(round.min_aim);
    let over = (first..=round.max_aim).find(|&a| round.resolve(a).landing == Landing::Past);
    over.or_else(|| (round.min_aim..=round.max_aim).find(|&a| round.resolve(a).landing != Landing::Pearl))
        .unwrap_or(round.max_aim)
}

impl Game {
    /// Open Pearl Hop for Shelly (`host`, her wearer id), whose finds are
    /// worth `base` pearls.
    pub(super) fn start_pearl_hop(&mut self, base: u32, host: String) {
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
            caption: opening_caption(&round),
            session: HopSession::new(round),
            base,
            host,
            in_flight: 0,
            tally_spoken: 0,
            reacted: false,
            won_caption: String::new(),
            pull_to: None,
            demo,
            counted: 0,
            aim_started: self.game_time,
            toss_ms: 0.0,
            now: 0.0,
        });
        self.set_state(GameState::PearlHop);
    }

    /// What Pearl Hop shows this frame (step, render and tests all read it).
    pub fn pearl_hop_view(&self) -> Option<HopView<'_>> {
        let a = self.active_pearl_hop.as_ref()?;
        // The won pearls land in the purse when they arrive, not before.
        let arriving = a.session.win_clock().is_some_and(|c| c < WIN_PEARL_ARRIVES) || a.session.win_clock().is_none();
        let pearls = if a.session.phase == HopPhase::Won && arriving { self.pearls.saturating_sub(a.in_flight) } else { self.pearls };
        Some(HopView {
            session: &a.session,
            pearls,
            caption: &a.caption,
            outfit: ShellyOutfit { worn: self.wardrobe.worn_by(&a.host), color: self.outfit_color(&a.host) },
        })
    }

    /// The panel's layout for this frame.
    pub fn pearl_hop_layout(&self, screen: (f32, f32)) -> Option<ui::pearl_hop::PearlHopLayout> {
        Some(ui::pearl_hop::layout(&self.pearl_hop_view()?, screen))
    }

    /// What the scene shows right now (see `ui::pearl_hop::scene_model`).
    pub fn pearl_hop_scene(&self) -> Option<ui::pearl_hop::SceneModel> {
        Some(ui::pearl_hop::scene_model(&self.pearl_hop_view()?))
    }

    /// One frame of Pearl Hop.
    pub(super) fn step_pearl_hop(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        let Some(l) = self.pearl_hop_layout(screen) else { return };
        self.active_pearl_hop.as_mut().unwrap().now = input.now;
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
                let (base, host) = (a.base, a.host.clone());
                self.start_pearl_hop(base, host);
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
            // Let go on a release — or on a fresh press while she's held,
            // which is how a native Linux touchscreen "drops" (it reports no
            // motion and no release): press Shelly, then tap where she goes.
            let let_go = input.mouse_released || !input.mouse_down || input.mouse_clicked;
            if !let_go {
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
                let next = (s.aim as i32 + d).clamp(s.round.min_aim as i32, s.round.max_aim as i32) as u16;
                self.set_pearl_hop_aim(next);
            }
            Some(HopInput::Toss) => self.toss_shelly(),
            _ => {}
        }
    }

    /// Move the aim; Shelly counts the hop out loud as it snaps from stone to
    /// stone ("one… two… three…").
    fn set_pearl_hop_aim(&mut self, aim: u16) {
        let a = self.active_pearl_hop.as_mut().unwrap();
        let before = a.session.aim;
        a.session = hop_reducer(a.session.clone(), HopAction::Aim { at: aim });
        if a.session.aim != before {
            audio::tts::speak("Shelly", &a.session.aim.to_string());
        }
    }

    fn toss_shelly(&mut self) {
        let now = self.game_time;
        let a = self.active_pearl_hop.as_mut().unwrap();
        a.session = hop_reducer(a.session.clone(), HopAction::Toss);
        a.counted = 0;
        a.tally_spoken = 0;
        a.reacted = false;
        a.toss_ms = ((now - a.aim_started).max(0.0) * 1000.0) as f64;
        a.caption = "Wheee!".to_string();
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
                audio::tts::speak("Shelly", &(t.landings[a.counted]).to_string());
                a.counted += 1;
            }
        }

        // Counting stage: the stones she covered are counted out loud, one
        // per stone as they light up, before anything else is said.
        while a.tally_spoken < s.tallied() {
            a.tally_spoken += 1;
            audio::tts::speak("Shelly", &a.tally_spoken.to_string());
        }

        let after = s.phase;
        if before == HopPhase::Flying && after != HopPhase::Flying {
            self.pearl_hop_landed();
        }
        self.pearl_hop_react();
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
        self.log_pearl_hop_toss(&round, &toss, ms);

        if toss.landing == Landing::Pearl {
            // Base for this Shelly's pearl, +1 for getting it first toss, +1
            // more with Hermie's Diving Net. Paid now; it lands in the purse
            // when the pearl's flight does.
            let payout = domain_shop::pearl_payout(base, 1, clean, &self.upgrades);
            let paid = self.award_pearls(payout);
            self.events.push(GameEvent::PearlHopWon { stage, tosses, pearls: payout.total() });
            self.persist();
            let a = self.active_pearl_hop.as_mut().unwrap();
            a.in_flight = payout.total();
            // Shown once the count reaches the pearl (see pearl_hop_react).
            a.won_caption = format!("My pearl!  {paid}");
        }
    }

    /// Say the landing's line once its count is done: the cheer as the pearl
    /// pops, or the funny reaction to a miss. Never "wrong".
    fn pearl_hop_react(&mut self) {
        let a = self.active_pearl_hop.as_mut().unwrap();
        let s = &a.session;
        if a.reacted || !matches!(s.phase, HopPhase::Landed | HopPhase::Won) || s.clock < s.tally_secs() {
            return;
        }
        a.reacted = true;
        let water = s.lands_in_water();
        let won = a.won_caption.clone();
        let (said, shown) = match s.toss.as_ref().map(|t| t.landing) {
            Some(Landing::Pearl) => ("Yaaay! My pearl! You found it!", Some(won.as_str())),
            Some(Landing::Past) if water => ("Sploosh! Too far!", Some("Sploosh! Too far!")),
            Some(Landing::Past) => ("Whoa, too far!", Some("Whoa! Too far!")),
            _ => ("Boing! Not there yet!", Some("Boing! Not there yet!")),
        };
        if let Some(shown) = shown {
            a.caption = shown.to_string();
        }
        audio::tts::speak("Shelly", said);
    }

    /// Stealth assessment: every toss is a data point for the adaptive
    /// system and the parent log. The kid never sees any of it.
    ///
    /// Counting → Add/AddSingle (counting on from zero), concrete stones.
    /// Skip counting → Multiply (hop × hops), on the number path.
    /// X hops → Divide (the pearl split into X equal hops); abstract from
    /// band 5, where the numbers outgrow counting stones one by one.
    fn log_pearl_hop_toss(&mut self, round: &HopRound, toss: &Toss, ms: f64) {
        let correct = toss.landing == Landing::Pearl;
        let band = self.profile.math_band;
        let hops = toss.landings.len() as i32;
        let aim = (toss.aim) as i32;
        let pearl = round.pearl as i32;
        let (op_name, operation, sub_skill, cra) = match round.stage {
            HopStage::Count => ("count", Operation::Add, SubSkill::AddSingle, CraStage::Concrete),
            HopStage::SkipCount => {
                ("skip_count", Operation::Multiply, classify_multiplication(aim.max(1), hops.max(1)), CraStage::Representational)
            }
            HopStage::Hops => {
                let cra = if band >= 5 { CraStage::Abstract } else { CraStage::Representational };
                ("divide", Operation::Divide, classify_division(pearl, round.hops.max(1) as i32), cra)
            }
        };
        {
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
        // One record per toss in the parent log and the saved attempt log:
        // the toss's answer is where she came down (counting, skip counting)
        // or the hop size picked (X hops).
        {
            let end = toss.end() as i32;
            let (a, b, correct_answer, answer) = match round.stage {
                HopStage::Hops => (pearl, round.hops as i32, pearl / round.hops.max(1) as i32, aim),
                HopStage::SkipCount => (aim, hops, pearl, end),
                _ => (pearl, 0, pearl, end),
            };
            let record = AttemptRecord {
                at: self.active_pearl_hop.as_ref().map_or(0.0, |a| a.now),
                play_secs: self.play_time,
                source: "shelly".to_string(),
                operation,
                sub_skill: Some(sub_skill),
                a,
                b,
                format: format!("pearl_hop_{op_name}"),
                band,
                center_band: band,
                cra_stage: cra,
                correct_answer,
                answers: vec![AnswerAt { value: answer, ms: ms.min(u32::MAX as f64) as u32 }],
                correct,
                help: Help::None,
                help_ms: None,
                told_me: false,
                workspace: None,
            };
            self.session_log.record_challenge(record.clone());
            self.attempt_log = std::mem::take(&mut self.attempt_log).record(record);
        }
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
        a.tally_spoken = 0;
        a.reacted = false;
        a.aim_started = self.game_time;
        a.caption = opening_caption(&a.session.round);
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
