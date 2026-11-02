//! Sockbot's Sock Shop: the pairing minigame he runs in Robot Land. The
//! rules are the domain's `logic::socks` reducer; this file turns taps and
//! keys into its actions, gives Sockbot his lines, pays out Dum Dums, and
//! logs every order for the adaptive system (never shown to the kid).

use super::*;
use robot_buddy_domain::learning::challenge_generator::classify_multiplication;
use robot_buddy_domain::logic::socks::{generate_round, sock_reducer, Fit, SockAction, SockPhase, SockRound, SockSession, SockStage};
use robot_buddy_domain::types::SubSkill;
use crate::ui::socks::{order_line, SockInput, SockView};

/// Dum Dums one socked order pays, and the bonus for getting it first try.
pub const SOCK_PAY: u32 = 1;
pub const SOCK_CLEAN_BONUS: u32 = 1;

/// A live Sock Shop. Lives only while `GameState::SockShop` is up.
pub struct ActiveSocks {
    pub session: SockSession,
    /// Sockbot's line, for a grown-up reading along. The kid hears it.
    pub caption: String,
    /// The reaction (or the cheer) has been said for the current hand-over.
    reacted: bool,
    /// When the kid started on this try (silent response time).
    try_started: f32,
    /// How long the order in flight took to put together, in ms.
    try_ms: f64,
    /// Wall clock (Unix secs) from the last frame's input, for the attempt log.
    now: f64,
}

/// What Sockbot says to open an order: the order, and (the first stage)
/// the one rule.
fn opening_line(round: &SockRound) -> String {
    let order = order_line(round);
    match round.stage {
        SockStage::Feet => format!("{order} Socks come in pairs. Tap plus one to put a pair on!"),
        SockStage::Group => format!("{order} How many pairs of socks do they need?"),
        SockStage::Order => {
            let w = round.walker;
            format!("Order up! {} {}, {} feet each. How many pairs?", round.robots, w.name(round.robots), w.feet())
        }
    }
}

/// The few words under the scene for a grown-up reading along: the rule,
/// without repeating the order over it.
fn opening_caption(round: &SockRound) -> String {
    match round.stage {
        SockStage::Feet => "Socks come in pairs. Tap +1 to put a pair on!".to_string(),
        SockStage::Group => "How many pairs of socks do they need?".to_string(),
        SockStage::Order => format!("{} feet each. How many pairs?", round.walker.feet()),
    }
}

/// Sockbot's line for a hand-over that didn't fit. Never "wrong": a cold
/// foot, or socks to spare.
fn miss_line(round: &SockRound, fit: Fit) -> String {
    match fit {
        Fit::Short { bare: 1 } => "Brrr! One foot is still cold! Just a little more.".to_string(),
        Fit::Short { bare } if round.stage == SockStage::Order => format!("Brrr! {bare} cold feet! We need more pairs."),
        Fit::Short { .. } => "Brrr! Some feet are still cold! We need more pairs.".to_string(),
        Fit::Spare { pairs: 1 } => "Ooh, one pair left over! We gave too many.".to_string(),
        Fit::Spare { pairs } => format!("Ooh, {pairs} pairs left over! We gave too many."),
        Fit::Snug => String::new(),
    }
}

fn win_line(round: &SockRound) -> String {
    if round.lonely_sock() {
        "Every foot is cozy! And one lonely sock for ME! My collection grows!".to_string()
    } else if round.robots == 1 {
        "Every foot is cozy! Look at those toes dance!".to_string()
    } else {
        "Every foot is cozy! Look at them all dance!".to_string()
    }
}

impl Game {
    /// Open the Sock Shop: Sockbot's next customer, sized to the kid's band.
    pub(super) fn start_sock_shop(&mut self) {
        let round = generate_round(self.profile.math_band, &mut self.rng);
        self.events.push(GameEvent::SockShopStarted { stage: round.stage, pairs: round.pairs_needed() });
        let line = opening_line(&round);
        audio::tts::speak("Sockbot", &line);
        self.activity.socks = Some(ActiveSocks {
            caption: opening_caption(&round),
            session: SockSession::new(round),
            reacted: false,
            try_started: self.game_time,
            try_ms: 0.0,
            now: 0.0,
        });
        self.set_state(GameState::SockShop);
    }

    /// Sockbot's Sock Shop, while it's up.
    pub fn active_socks(&self) -> Option<&ActiveSocks> {
        self.activity.socks.as_ref()
    }

    /// What the Sock Shop panel shows this frame.
    pub(super) fn sock_view(&self) -> Option<SockView> {
        let a = self.activity.socks.as_ref()?;
        Some(SockView::new(&a.session, &a.caption))
    }

    /// One frame of the Sock Shop.
    pub(super) fn step_sock_shop(&mut self, input: &FrameInput, dt: f32) {
        let Some(a) = self.activity.socks.as_mut() else { return };
        a.now = input.now;
        let intent = ui::socks::handle_click(input).or_else(|| ui::socks::handle_key(input, &a.session));
        match intent {
            // Leaving is always one tap away, whatever's going on.
            Some(SockInput::Leave) => {
                self.activity.socks = None;
                self.events.push(GameEvent::SockShopLeft);
                self.set_state(GameState::Playing);
                return;
            }
            Some(SockInput::Next) if a.session.win_done() => {
                self.start_sock_shop();
                return;
            }
            Some(SockInput::Add(n)) => {
                let before = a.session.pairs;
                a.session = sock_reducer(a.session.clone(), SockAction::Add { n });
                if a.session.pairs != before {
                    // Counting the pairs out loud as they come out of the drawer.
                    audio::tts::speak("Sockbot", &a.session.pairs.to_string());
                }
            }
            Some(SockInput::Send) if a.session.phase == SockPhase::Choosing && a.session.pairs > 0 => {
                a.try_ms = ((self.game_time - a.try_started).max(0.0) * 1000.0) as f64;
                a.session = sock_reducer(a.session.clone(), SockAction::Send);
                a.reacted = false;
                a.caption = "Here come the socks!".to_string();
                audio::tts::speak("Sockbot", "Here come the socks!");
                self.log_sock_order();
            }
            _ => {}
        }
        self.tick_sock_shop(dt);
    }

    fn tick_sock_shop(&mut self, dt: f32) {
        let now = self.game_time;
        let a = self.activity.socks.as_mut().unwrap();
        let before = a.session.phase;
        a.session = sock_reducer(a.session.clone(), SockAction::Tick { dt });
        let after = a.session.phase;
        if before == SockPhase::Dressing && after != SockPhase::Dressing && !a.reacted {
            a.reacted = true;
            let round = a.session.round.clone();
            let line = match a.session.sent {
                Some(Fit::Snug) => win_line(&round),
                Some(fit) => miss_line(&round, fit),
                None => String::new(),
            };
            a.caption = line.clone();
            audio::tts::speak("Sockbot", &line);
            if after == SockPhase::Won {
                let (tries, clean, stage) = (a.session.tries, a.session.was_clean(), round.stage);
                let pay = SOCK_PAY + if clean { SOCK_CLEAN_BONUS } else { 0 };
                self.award_dum_dums(pay);
                self.events.push(GameEvent::SockShopWon { stage, tries, dum_dums: pay });
                self.persist();
            }
        }
        if after == SockPhase::Choosing && before != SockPhase::Choosing {
            let a = self.activity.socks.as_mut().unwrap();
            a.try_started = now;
            a.caption = "Let's try again! How many pairs?".to_string();
        }
    }

    /// Stealth assessment: every order handed over is a data point on
    /// pairing, for the profile and the parent log. The kid never sees it.
    ///
    /// The record names the arithmetic the stage leans on, for the parent
    /// log's detail: one robot → counting in twos (Add/AddSingle), a line of
    /// robots → doubling and halving (Multiply), an order slip → robots ×
    /// pairs each (Multiply). The report files them all under pairing.
    fn log_sock_order(&mut self) {
        let Some(a) = self.activity.socks.as_ref() else { return };
        let s = &a.session;
        let round = &s.round;
        let correct = s.sent == Some(Fit::Snug);
        let band = self.profile.math_band;
        let (feet, robots) = (round.walker.feet() as i32, round.robots as i32);
        let (stage_name, operation, sub_skill, cra) = match round.stage {
            SockStage::Feet => ("feet", Operation::Add, SubSkill::AddSingle, CraStage::Concrete),
            SockStage::Group => ("group", Operation::Multiply, classify_multiplication(robots.max(1), feet.max(1)), CraStage::Representational),
            SockStage::Order => ("order", Operation::Multiply, classify_multiplication(robots.max(1), (feet / 2).max(1)), CraStage::Abstract),
        };
        // Every order, misses included, is pairing evidence, and only that: it
        // never moves the math band or another operation's stats.
        self.profile = learner_reducer(self.profile.clone(), LearnerEvent::PairsAttempted {
            correct,
            response_time_ms: Some(a.try_ms),
        });
        let record = AttemptRecord {
            at: a.now,
            play_secs: self.play_time,
            source: "sockbot".to_string(),
            operation,
            sub_skill: Some(sub_skill),
            a: robots,
            b: feet,
            format: format!("socks_{stage_name}"),
            band,
            center_band: band,
            cra_stage: cra,
            correct_answer: round.pairs_needed() as i32,
            answers: vec![AnswerAt { value: s.pairs as i32, ms: a.try_ms.min(u32::MAX as f64) as u32 }],
            correct,
            help: Help::None,
            help_ms: None,
            told_me: false,
            workspace: None,
        };
        self.events.push(GameEvent::SockOrderSent { stage: round.stage, pairs: s.pairs, needed: round.pairs_needed(), snug: correct });
        self.session_log.record_challenge(record.clone());
        self.attempt_log = std::mem::take(&mut self.attempt_log).record(record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robot_buddy_domain::logic::socks::Walker;

    #[test]
    fn sockbot_never_says_wrong() {
        let r = SockRound { stage: SockStage::Order, walker: Walker::SpiderBot, robots: 3, max_pairs: 20 };
        for fit in [Fit::Short { bare: 1 }, Fit::Short { bare: 8 }, Fit::Spare { pairs: 1 }, Fit::Spare { pairs: 3 }] {
            let line = miss_line(&r, fit).to_lowercase();
            assert!(!line.contains("wrong") && !line.contains("no!"), "{line}");
            assert!(!line.is_empty());
        }
    }

    #[test]
    fn every_stage_opens_with_the_order() {
        for stage in [SockStage::Feet, SockStage::Group, SockStage::Order] {
            let r = SockRound { stage, walker: Walker::Tripod, robots: 2, max_pairs: 20 };
            assert!(opening_line(&r).contains("tripod"), "{stage:?}");
        }
    }
}
