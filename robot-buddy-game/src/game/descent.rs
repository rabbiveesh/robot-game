//! Inkwell's dive shaft: the descent minigame that gates the trench.

use super::*;

impl Game {
    /// The dive shaft leading down from this map, if it has one. There's no
    /// tile to step on any more — Inkwell is the way down.
    pub(super) fn dive_portal(&self) -> Option<tilemap::Portal> {
        tilemap::all_portals().iter().copied()
            .find(|p| p.dive && p.from_map == self.map.id)
    }

    /// Open the descent: generate a shaft for the kid's band and hand them the
    /// kicks. Nothing is spent and nothing is lost if they swim back up.
    pub(super) fn start_descent(&mut self) {
        let puzzle = generate_dive(self.profile.math_band, &mut self.rng);
        let optimal = puzzle.optimal_kicks();
        let door = puzzle.door;
        self.events.push(GameEvent::DescentStarted { door, optimal });
        let speaker = self.current_buddy_name();
        audio::tts::speak(&speaker, &format!("The trench door is {door} marks down!"));
        self.active_descent = Some(ActiveDescent {
            session: DiveSession::new(puzzle),
            landed_timer: 0.0,
            message: None,
        });
        self.set_state(GameState::Descent);
    }

    /// One frame of the dive. Kicks run through the pure reducer; landing on
    /// the door holds a short beat, pays a pearl for a clean dive, and then
    /// lets the shaft portal do its normal job.
    pub(super) fn step_descent(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        let Some(ad) = self.active_descent.as_ref() else { return };
        let layout = ui::descent::layout(&ad.session, screen);

        // Landed: hold the beat, then descend for real.
        if ad.session.phase == DivePhase::Landed {
            let done = {
                let ad = self.active_descent.as_mut().unwrap();
                ad.landed_timer += dt;
                ad.landed_timer >= 1.4 || input.pressed(KeyCode::Space) || input.mouse_clicked
            };
            if done {
                self.resolve_descent();
            }
            return;
        }

        let intent = if input.mouse_clicked {
            let (mx, my) = input.mouse_pos;
            ui::descent::handle_click(mx, my, &layout)
        } else {
            ui::descent::handle_key(input, &ad.session)
        };
        let Some(intent) = intent else { return };

        let action = match intent {
            // Bailing is always free — swim up and the shaft is still there.
            ui::descent::DescentInput::Leave => {
                self.active_descent = None;
                self.set_state(GameState::Playing);
                return;
            }
            ui::descent::DescentInput::Sink(n) => DiveAction::Sink { n },
            ui::descent::DescentInput::Rise(n) => DiveAction::Rise { n },
        };

        let ad = self.active_descent.as_mut().unwrap();
        ad.session = dive_reducer(ad.session.clone(), action);
        ad.message = None;

        // One line of buddy chatter per beat — a nudge, never a verdict.
        let (speaker, line) = (self.current_buddy_name(), {
            let s = &self.active_descent.as_ref().unwrap().session;
            match s.phase {
                DivePhase::Landed => Some("We made it! The trench door is open!".to_string()),
                _ => match s.nudge {
                    DiveNudge::Bumped => Some("Bonk! That ledge won't hold us.".to_string()),
                    DiveNudge::Bottomed => Some("That's the bottom! Kick back up.".to_string()),
                    DiveNudge::None => None,
                },
            }
        });
        if let Some(line) = line {
            audio::tts::speak(&speaker, &line);
        }
    }

    /// The dive landed: pay for a clean one, then run the shaft portal the kid
    /// is still standing on so the normal transfer (and arrival speech) fires.
    pub(super) fn resolve_descent(&mut self) {
        let Some(ad) = self.active_descent.take() else { return };
        let optimal = ad.session.puzzle.optimal_kicks();
        self.events.push(GameEvent::DescentLanded {
            door: ad.session.puzzle.door,
            kicks: ad.session.kicks_used,
            optimal,
        });
        if ad.session.was_clean() {
            // A tidy decomposition is worth a pearl. A scenic one costs
            // nothing — it still opened the door.
            let bonus = if self.has_diving_net() { shop::DIVING_NET_BONUS } else { 0 };
            let payout = 1 + bonus;
            self.pearls = self.pearls.saturating_add(payout);
            self.pearl_hud.flash();
            let mut cheer = format!("Perfect dive!  +{payout} pearl");
            if payout > 1 { cheer.push('s'); }
            if bonus > 0 { cheer.push_str("  (your net caught one!)"); }
            self.track_toast = Some((cheer, 2.0));
        }
        self.set_state(GameState::Playing);
        if let Some(portal) = self.dive_portal() {
            self.take_portal(portal);
        }
    }
}
