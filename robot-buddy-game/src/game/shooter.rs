//! The Goyish Map number-bond shooter: launch, per-frame step, resolve.

use super::*;

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
        self.active_shooter = Some(ActiveShooter {
            session,
            complete_timer: 0.0,
            start_time: self.game_time,
            source_npc: source,
        });
        self.set_state(GameState::Shooter);
    }

    pub(super) fn step_shooter(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        // Ship glide speed in logical field units/sec (the field is 100 wide),
        // nudged up at a relaxed pace so aiming keeps up with thinking.
        let ship_speed = 70.0 * self.game_pace.ship_multiplier();

        // Bail out any time — no reward, no penalty. The kid can just walk away.
        if input.pressed(KeyCode::Escape) {
            self.active_shooter = None;
            self.set_state(GameState::Playing);
            return;
        }

        let prev_wave = self.active_shooter.as_ref().map(|a| a.session.wave).unwrap_or(0);
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
                let left = input.down(KeyCode::Left) || input.down(KeyCode::A);
                let right = input.down(KeyCode::Right) || input.down(KeyCode::D);
                if left && !right {
                    s = shooter_reducer(s, ShooterAction::MoveShip { dx: -ship_speed * dt });
                } else if right && !left {
                    s = shooter_reducer(s, ShooterAction::MoveShip { dx: ship_speed * dt });
                }
                if input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) {
                    s = shooter_reducer(s, ShooterAction::Fire);
                }
                // Click/tap to shoot: snap the ship to the tapped column and fire
                // from there. Lets a kid aim by pointing instead of nudging.
                if input.mouse_clicked {
                    let (mx, my) = input.mouse_pos;
                    if let Some(fx) = ui::shooter::field_x_at(screen, mx, my) {
                        let dx = fx - s.ship_x;
                        s = shooter_reducer(s, ShooterAction::MoveShip { dx });
                        s = shooter_reducer(s, ShooterAction::Fire);
                    }
                }
                s = shooter_reducer(s, ShooterAction::Tick { dt });
                a.session = s;
            }
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
                let representation = a.session.representation;
                let response_ms = self.elapsed_ms(a.start_time, 600000.0);

                // Stealth assessment: every pairing is a NumberBond data point.
                // The child never sees a score or "attempt" — this only feeds the
                // adaptive system.
                for i in 0..(hits + misses) {
                    let correct = i < hits;
                    self.profile = learner_reducer(self.profile.clone(), LearnerEvent::PuzzleAttempted {
                        correct,
                        operation: Operation::NumberBond,
                        sub_skill: None,
                        band: self.profile.math_band,
                        center_band: None,
                        response_time_ms: None,
                        hint_used: false,
                        told_me: false,
                        cra_level_shown: Some(representation),
                        timestamp: Some(self.game_time as f64 * 1000.0),
                    });
                }

                // Finishing the run pays out. A number-bond hunt naturally
                // involves trial-and-error, so misses don't void the reward.
                self.finish_puzzle(true, 0, GameEvent::ShooterResolved { waves, hits, misses, response_ms });
            }
        }
    }
}
