//! The Goyish Map number-bond shooter: launch, per-frame step, resolve.

use super::*;
use robot_buddy_domain::logic::shooter::HopDir;

/// The cheer when a wave pairs off with no wrong pairs. It only ever shows for
/// a clean wave; any other wave just clears (no "you missed", Invariant 7).
pub const CLEAN_WAVE_CHEER: &str = "Perfect wave!";

impl Game {
    /// Launch the number-bond space shooter. Difficulty rides the math band and
    /// the numbers are drawn per the learner's NumberBond CRA stage — both picked
    /// silently, the kid never sees them (Invariant 6).
    pub(super) fn start_shooter(&mut self, source: String) {
        let cra_stage = self.profile.cra_stages
            .get(&Operation::NumberBond).copied()
            .unwrap_or(CraStage::Concrete);
        let session = ShooterSession::new(
            self.profile.math_band, cra_stage, self.game_pace, &mut self.rng);
        self.events.push(GameEvent::ShooterStarted {
            band: self.profile.math_band,
            source: source.clone(),
        });
        let ship_draw_x = session.ship_x;
        self.active_shooter = Some(ActiveShooter {
            session,
            ship_draw_x,
            complete_timer: 0.0,
            start_time: self.game_time,
            source_npc: source,
        });
        self.set_state(GameState::Shooter);
    }

    pub(super) fn step_shooter(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        // Bail out any time — no reward, no penalty. The kid can just walk away,
        // by key or by tapping Leave (a tablet has no ESC).
        let tapped_leave = input.mouse_clicked
            && ui::shooter::leave_hit(screen, input.mouse_pos.0, input.mouse_pos.1);
        if input.pressed(KeyCode::Escape) || tapped_leave {
            self.active_shooter = None;
            self.set_state(GameState::Playing);
            return;
        }

        let prev_wave = self.active_shooter.as_ref().map(|a| a.session.wave).unwrap_or(0);
        let prev_cleared = self.active_shooter.as_ref().map_or(0, |a| a.session.cleared_waves.len());
        let mut finished = false;

        if let Some(a) = self.active_shooter.as_mut() {
            if a.session.phase == ShooterPhase::Complete {
                // Victory beat, then dismiss on a tap or after a short pause.
                a.complete_timer += dt;
                if a.complete_timer >= 2.5
                    || input.pressed(KeyCode::Space)
                    || input.pressed(KeyCode::Enter)
                    || input.mouse_clicked
                {
                    finished = true;
                }
            } else {
                // Reducers are pure (state in, state out); run the frame's
                // actions through a detached session, then store the result.
                let mut s = a.session.clone();
                // Arrows hop the ship one alien at a time — one press, one hop,
                // so every keyboard shot is aimed at exactly one number.
                let left = input.pressed(KeyCode::Left) || input.pressed(KeyCode::A);
                let right = input.pressed(KeyCode::Right) || input.pressed(KeyCode::D);
                if left && !right {
                    s = shooter_reducer(s, ShooterAction::Hop { dir: HopDir::Left });
                } else if right && !left {
                    s = shooter_reducer(s, ShooterAction::Hop { dir: HopDir::Right });
                }
                if input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) {
                    s = shooter_reducer(s, ShooterAction::FireFrom { source: ShotSource::Keys });
                }
                // Click/tap to shoot: snap the ship to the tapped column and fire
                // from there. Lets a kid aim by pointing instead of nudging.
                if input.mouse_clicked {
                    let (mx, my) = input.mouse_pos;
                    if let Some(fx) = ui::shooter::field_x_at(screen, mx, my) {
                        let dx = fx - s.ship_x;
                        s = shooter_reducer(s, ShooterAction::MoveShip { dx });
                        s = shooter_reducer(s, ShooterAction::FireFrom { source: ShotSource::Tap });
                    }
                }
                s = shooter_reducer(s, ShooterAction::Tick { dt });
                a.session = s;
            }
            // The domain snaps lane to lane; the drawn ship glides there fast.
            let gap = a.session.ship_x - a.ship_draw_x;
            a.ship_draw_x = if gap.abs() < 0.05 {
                a.session.ship_x
            } else {
                a.ship_draw_x + gap * (1.0 - (-18.0 * dt).exp())
            };
        }

        // A wave just paired off with no wrong pairs: cheer, right as it clears.
        let clean_now = self.active_shooter.as_ref().is_some_and(|a| {
            a.session.cleared_waves.len() > prev_cleared
                && a.session.cleared_waves.last().is_some_and(|w| w.is_clean())
        });
        if clean_now {
            self.track_toast = Some((CLEAN_WAVE_CHEER.to_string(), 1.8));
        }

        // A wave just cleared (index advanced but the run isn't over yet).
        if let Some(a) = self.active_shooter.as_ref() {
            if a.session.wave > prev_wave && a.session.phase == ShooterPhase::Playing {
                self.events.push(GameEvent::ShooterWaveCleared { wave: prev_wave as u8 });
            }
        }

        if finished {
            if let Some(a) = self.active_shooter.take() {
                let waves = a.session.wave as u8;
                let hits = a.session.hits;
                let misses = a.session.misses;
                let clean_waves = rewards::clean_waves(&a.session.cleared_waves) as u8;
                let payout = rewards::shooter_payout(&a.session.cleared_waves);
                let representation = a.session.representation;
                let response_ms = self.elapsed_ms(a.start_time, 600000.0);

                // Stealth assessment: every pairing is a NumberBond data point,
                // fed in the order it happened so the frustration window sees
                // the kid's real run, not all hits then all misses. The child
                // never sees a score or "attempt" — this only feeds the
                // adaptive system.
                for att in &a.session.attempts {
                    self.profile = learner_reducer(self.profile.clone(), LearnerEvent::PuzzleAttempted {
                        correct: att.correct,
                        operation: Operation::NumberBond,
                        sub_skill: None,
                        band: self.profile.math_band,
                        center_band: None,
                        response_time_ms: Some((att.think_secs * 1000.0) as f64),
                        hint_used: false,
                        told_me: false,
                        cra_level_shown: Some(representation),
                        timestamp: Some(self.game_time as f64 * 1000.0),
                    });
                }

                // Finishing the run pays out (a bond hunt is trial-and-error,
                // so misses don't void it), plus one per clean wave — the
                // domain's rule, paid once through the shared tail.
                self.finish_puzzle_paying(payout, GameEvent::ShooterResolved {
                    waves, hits, misses, clean_waves, response_ms,
                });
            }
        }
    }
}
