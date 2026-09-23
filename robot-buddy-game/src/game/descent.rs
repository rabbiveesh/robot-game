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
        let line = if self.dive_portal().is_some() {
            format!("The trench door is {door} marks down!")
        } else {
            format!("The bottom is {door} marks down!")
        };
        audio::tts::speak(&speaker, &line);
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
                DivePhase::Landed if self.dive_portal().is_some() =>
                    Some("We made it! The trench door is open!".to_string()),
                DivePhase::Landed => Some("We made it! Down we go!".to_string()),
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
    /// A map with no shaft sends the diver to the Dogfish House instead.
    pub(super) fn resolve_descent(&mut self) {
        let Some(ad) = self.active_descent.take() else { return };
        let optimal = ad.session.puzzle.optimal_kicks();
        self.events.push(GameEvent::DescentLanded {
            door: ad.session.puzzle.door,
            kicks: ad.session.kicks_used,
            optimal,
        });
        // A tidy decomposition is worth a pearl. A scenic one costs nothing —
        // it still opened the door.
        let payout = domain_shop::pearl_payout(0, 1, ad.session.was_clean(), &self.upgrades);
        if payout.total() > 0 {
            let line = self.award_pearls(payout);
            self.track_toast = Some((format!("Perfect dive!  {line}"), 2.0));
        }
        self.set_state(GameState::Playing);
        match self.dive_portal() {
            Some(portal) => self.take_portal(portal),
            None => self.dive_to_dogfish_house(),
        }
    }

    /// Inkwell's buddy dive from a map with no shaft: remember exactly where the
    /// kid jumped in, then land them in the Dogfish House. The follower comes
    /// along through the normal warp (`take_portal` snaps it alongside).
    ///
    /// Diving again from *inside* the house doesn't nest: it lands back on the
    /// arrival tile and keeps the original return point, so the bubble column
    /// still goes home to where the first dive began. (The dive itself — and
    /// its clean-dive pearl — is the same as anywhere.)
    fn dive_to_dogfish_house(&mut self) {
        let here = self.map.id;
        if here != tilemap::DOGFISH_HOUSE {
            self.dive_return = Some(DiveReturn {
                map_id: here.to_string(),
                tile_x: self.player.tile_x,
                tile_y: self.player.tile_y,
            });
        }
        let (to_x, to_y) = tilemap::DOGFISH_ARRIVAL;
        self.take_portal(tilemap::Portal {
            from_map: here,
            from_x: self.player.tile_x,
            from_y: self.player.tile_y,
            to_map: tilemap::DOGFISH_HOUSE,
            to_x,
            to_y,
            dir: Dir::Down,
            // Secret, so the first arrival plays its little speech.
            secret: true,
            cost: 0,
            fuel_cost: 0,
            dive: true,
        });
    }

    /// The Dogfish House's bubble column, aimed at the spot the dive began.
    /// Spends the return point. With none on record (or one that no longer
    /// names a real map tile), `fallback` — the column's static portal — is
    /// used, which is a safe spot at home.
    pub(super) fn dogfish_exit(&mut self, fallback: tilemap::Portal) -> tilemap::Portal {
        let Some(ret) = self.dive_return.take() else { return fallback };
        let dest = Map::by_id(&ret.map_id);
        let valid = dest.id == ret.map_id
            && dest.id != tilemap::DOGFISH_HOUSE
            && ret.tile_x < dest.width
            && ret.tile_y < dest.height;
        if !valid {
            return fallback;
        }
        tilemap::Portal {
            to_map: dest.id,
            to_x: ret.tile_x,
            to_y: ret.tile_y,
            dir: Dir::Down,
            ..fallback
        }
    }
}
