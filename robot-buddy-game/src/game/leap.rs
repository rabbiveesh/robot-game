//! Shelly's pearl leaps: the number-track stones, the leap-size choice, and
//! the stepping-stone drawing.

use super::*;

/// How long one of Shelly's leaps takes, whatever its size — so a leap reads
/// as a jump rather than a long swim, and a big leap still feels like one hop.
const LEAP_SECONDS: f32 = 0.4;

impl Game {
    /// Shelly's pearl leaps. Her stones sit a leap apart with rip current in
    /// between, so the path can't be walked: the kid commits to ONE leap size
    /// on the launch stone and then leaps it out. Land on the pearl's stone and
    /// it pops (+payout, +1 more if the size was right first time); sail past
    /// and you swim back and pick again. Picking the size IS the arithmetic —
    /// skip-counting when Shelly names the size, partitioning when she names
    /// the number of leaps.
    ///
    /// The session only lives while the kid is standing on the stone it thinks
    /// they're on. Walking off the path (or around it, over the sea floor)
    /// drops the trip, so a pearl can never be strolled into.
    pub(super) fn check_number_track_landing(&mut self, _dt: f32) {
        let track = match number_track::track_for_map(self.map.id) {
            Some(t) => t,
            None => {
                self.leap_session = None;
                return;
            }
        };
        let here = track.index_of((self.player.tile_x, self.player.tile_y));

        match here {
            // Off the stones entirely — the trip is over, no harm done.
            None => self.leap_session = None,
            Some(0) if self.leap_session.is_none() => {
                // Standing on the launch stone with no trip going: Shelly sets
                // one up. Generating here (rather than on a timer) means every
                // visit to the stone is a fresh puzzle.
                let puzzle = generate_leap(self.profile.math_band, track.max_mark(), &mut self.rng);
                self.events.push(GameEvent::LeapTripOffered {
                    pearl: puzzle.pearl,
                    size: puzzle.size,
                    count: puzzle.count,
                });
                let call = leap_call(&puzzle);
                audio::tts::speak("Shelly", &call);
                self.track_toast = Some((call, 3.0));
                self.leap_session = Some(LeapSession::new(puzzle));
            }
            Some(i) => {
                // On a stone the trip doesn't account for — they walked round.
                // Drop it rather than pretending they leapt here.
                if self.leap_session.as_ref().is_some_and(|s| s.position as usize != i) {
                    self.leap_session = None;
                }
            }
        }
    }

    /// Turn a keyboard/tap intent into a leap while the kid is on the stones.
    /// Returns true when the input was spent on the pearl path, so the normal
    /// walk resolver leaves it alone.
    pub(super) fn handle_leap_input(&mut self, input: &FrameInput, screen: (f32, f32)) -> bool {
        let Some(track) = number_track::track_for_map(self.map.id) else { return false };
        if self.leap_session.is_none() || self.player.moving {
            return false;
        }

        // Taps on Shelly's panel do the same three things the keys do — and a
        // tap that lands on the panel is never also a click-to-walk.
        let (tap, on_panel) = {
            let s = self.leap_session.as_ref().unwrap();
            let layout = ui::leap::layout(s, screen);
            let (mx, my) = input.mouse_pos;
            if input.mouse_clicked {
                (ui::leap::handle_click(mx, my, &layout), ui::leap::absorbs_click(mx, my, &layout))
            } else {
                (None, false)
            }
        };

        // Pick a leap size: the number keys line up with Shelly's offered
        // sizes, cheapest first, same as every other menu in the game.
        let choice = {
            let s = self.leap_session.as_ref().unwrap();
            let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
            keys.iter().take(s.puzzle.choices.len()).enumerate()
                .find(|(_, k)| input.pressed(**k))
                .map(|(i, _)| s.puzzle.choices[i])
                .or(match tap {
                    Some(ui::leap::LeapInput::Pick(n)) => Some(n),
                    _ => None,
                })
        };
        if let Some(size) = choice {
            let s = self.leap_session.take().unwrap();
            let s = leap_reducer(s, LeapAction::Choose { size });
            if s.chosen == Some(size) {
                audio::tts::speak("Shelly", &format!("Leaping by {size}! Go!"));
                self.track_toast = Some((format!("Leaping by {size}s — jump east!"), 2.0));
            }
            self.leap_session = Some(s);
            return true;
        }

        let forward = input.pressed(KeyCode::Right) || input.pressed(KeyCode::D)
            || matches!(tap, Some(ui::leap::LeapInput::Leap));
        let back = input.pressed(KeyCode::Left) || input.pressed(KeyCode::A)
            || matches!(tap, Some(ui::leap::LeapInput::SwimBack));
        if !forward && !back {
            // Swallow a tap that hit the panel but no button, so it doesn't
            // send the kid walking off the stones.
            return on_panel;
        }

        if back {
            // Swim back to the launch stone and think again. Always free.
            let s = leap_reducer(self.leap_session.take().unwrap(), LeapAction::SwimBack);
            let (col, row) = track.tiles[0];
            self.player.dir = Dir::Left;
            self.player.start_leap(col, row, LEAP_SECONDS);
            self.snap_follower_to_player();
            self.leap_session = Some(s);
            return true;
        }

        // Forward: one leap of the committed size.
        let before = self.leap_session.as_ref().unwrap().clone();
        if before.chosen.is_none() {
            // Nothing picked yet — nudge rather than shuffling them into the
            // current, which they can't swim anyway.
            self.track_toast = Some(("Pick how big your leaps are first!".to_string(), 1.6));
            return true;
        }
        let after = leap_reducer(before.clone(), LeapAction::Leap);
        if after.position == before.position {
            return true; // overshot already; the only way on is back
        }
        let (col, row) = track.tiles[after.position as usize];
        self.player.dir = Dir::Right;
        self.player.start_leap(col, row, LEAP_SECONDS);
        self.snap_follower_to_player();

        match after.phase {
            LeapPhase::Found => {
                // Base rate for the path, +1 for getting the leap size right
                // first try, +1 more if they've bought Hermie's Diving Net.
                let payout = domain_shop::pearl_payout(track.payout, 1, after.was_clean(), &self.upgrades);
                let line = self.award_pearls(payout);
                self.events.push(GameEvent::PearlFound {
                    stone: after.puzzle.pearl,
                    size: after.puzzle.size,
                    leaps: after.leaps,
                    resets: after.resets,
                    pearls: payout.total(),
                });
                // award_pearls names the net every time it pays, so the kid
                // can see the twenty pearls still working for them.
                let cheer = if after.was_clean() {
                    format!("Right on it! The pearl was under stone {}!  {line}", after.puzzle.pearl)
                } else {
                    format!("You found it! Stone {}.  {line}", after.puzzle.pearl)
                };
                audio::tts::speak("Shelly", "You found my pearl!");
                self.track_toast = Some((cheer, 2.4));
                // Shelly hides it again — a fresh trip next time they launch.
                self.leap_session = None;
                self.persist();
            }
            LeapPhase::Overshot => {
                let msg = format!(
                    "Whoosh — stone {}! That's past my pearl. Swim back and try a different leap!",
                    after.position,
                );
                audio::tts::speak("Shelly", "Ooh, too far!");
                self.track_toast = Some((msg, 2.6));
                self.leap_session = Some(after);
            }
            _ => {
                self.leap_session = Some(after);
            }
        }
        true
    }
}


/// Shelly's call: the stone her pearl is under, plus the one clue she gives.
/// Younger kids get the leap size and skip-count it out; older ones get the
/// number of leaps and have to work the size out.
pub(super) fn leap_call(puzzle: &LeapPuzzle) -> String {
    match puzzle.clue {
        Clue::Size { n } => format!(
            "My pearl's under stone {}! Leap by {n}s to reach it!", puzzle.pearl,
        ),
        Clue::Count { n } => format!(
            "My pearl's under stone {}! You get there in {n} leaps — how big is each one?",
            puzzle.pearl,
        ),
    }
}

/// Draw the ambient number-line stepping-stones in world space (under the
/// sprites), plus Shelly's callout bubble naming the goal stone. Stones up to
/// the kid's current stone are lit; the pearl stays hidden until the kid
/// stands on the called-out stone. `here` is the kid's mark, if on the path.
pub(super) fn draw_number_track(
    track: &number_track::NumberTrack,
    here: Option<usize>,
    session: Option<&LeapSession>,
    time: f32,
) {
    let outline = Color::from_rgba(94, 122, 60, 200);
    let pearl_stone = session.map(|s| s.puzzle.pearl as usize);
    let next_stone = session.and_then(|s| s.next_stone()).map(|n| n as usize);

    for (i, &(col, row)) in track.tiles.iter().enumerate() {
        let cx = (col as f32 + 0.5) * TILE_SIZE;
        let cy = (row as f32 + 0.5) * TILE_SIZE;
        // Stones behind the diver are lit; the launch stone always glows so
        // the kid can find their way back to it.
        let lit = here.map_or(i == 0, |h| i <= h);
        let base = if lit {
            Color::from_rgba(255, 236, 179, 235)
        } else {
            Color::from_rgba(176, 190, 197, 170)
        };
        draw_circle(cx, cy, TILE_SIZE * 0.40, base);
        draw_circle_lines(cx, cy, TILE_SIZE * 0.40, 2.0, outline);

        // Where the next leap would land — the preview that makes a wrong
        // size visible BEFORE committing to the jump.
        if next_stone == Some(i) && here.is_some() {
            let pulse = (time * 4.0).sin() * 0.5 + 0.5;
            draw_circle_lines(cx, cy, TILE_SIZE * 0.46 + pulse * 3.0, 3.0,
                Color::new(0.45, 0.95, 0.75, 0.85));
        }
        // Shelly's called-out stone is marked; the pearl under it is not.
        if pearl_stone == Some(i) {
            let pulse = (time * 3.0).sin() * 0.5 + 0.5;
            draw_circle_lines(cx, cy, TILE_SIZE * 0.52 + pulse * 2.0, 3.0,
                Color::new(1.0, 0.84, 0.30, 0.8));
        }

        let label = format!("{i}");
        let tw = measure_text(&label, None, 22, 1.0).width;
        draw_text(&label, cx - tw / 2.0, cy + 7.0, 22.0, Color::from_rgba(40, 52, 30, 240));
    }
}
