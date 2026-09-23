//! The grid and logic puzzles — KenKen, patterns, balance, sudoku — plus the
//! resolve tail every puzzle (the shooter too) shares.

use super::*;

impl Game {
    pub(super) fn step_kenken(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        // Intro overlay swallows all input until the kid taps past the last
        // step. Only on completion do we fire the profile event so this never
        // fires again.
        let mut intro_finished = false;
        if let Some(ref mut ak) = self.active_kenken {
            if let Some(step) = ak.intro_step {
                if input.mouse_clicked || input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) {
                    let next = step + 1;
                    if next >= ui::kenken::INTRO_STEPS {
                        ak.intro_step = None;
                        intro_finished = true;
                        // Reset start_time so the kid's intro reading time
                        // doesn't pollute the puzzle response measurement.
                        ak.start_time = self.game_time;
                    } else {
                        ak.intro_step = Some(next);
                    }
                }
                if !intro_finished {
                    return; // skip puzzle logic while intro is showing
                }
            }
        }
        if intro_finished {
            self.profile = learner_reducer(self.profile.clone(), LearnerEvent::KenKenIntroSeen);
        }

        let mut dismiss = false;
        if let Some(ref mut ak) = self.active_kenken {
            // Auto-dismiss timer once solved.
            if ak.session.phase == KenKenPhase::Complete {
                ak.complete_timer += dt;
                if ak.complete_timer >= 2.5 { dismiss = true; }
                // Accept any input to dismiss — Space/Enter or a click
                // anywhere on the panel. Keeps the celebration screen feeling
                // tap-friendly for kids.
                if input.pressed(KeyCode::Space)
                    || input.pressed(KeyCode::Enter)
                    || input.mouse_clicked
                {
                    dismiss = true;
                }
            }

            if !dismiss {
                let layout = ui::kenken::layout(&ak.session, screen);

                // Keyboard input (number 1..N to fill the selected cell).
                if let Some(intent) = ui::kenken::handle_key(&ak.session, input, ak.selected) {
                    apply_kenken_intent(ak, intent);
                }

                // Mouse click → select cell, place value, hint, or clear.
                if input.mouse_clicked {
                    let (mx, my) = input.mouse_pos;
                    if let Some(intent) = ui::kenken::handle_click(mx, my, &ak.session, &layout, ak.selected) {
                        apply_kenken_intent(ak, intent);
                    }
                }
            }
        }

        if dismiss {
            if let Some(ak) = self.active_kenken.take() {
                let was_correct = ak.session.phase == KenKenPhase::Complete;
                let response_ms = self.elapsed_ms(ak.start_time, 120000.0);
                let grid_size = ak.session.puzzle.grid_size;
                let hints_used = ak.session.hints_used;
                let violations = ak.session.constraint_violations;

                self.profile = learner_reducer(self.profile.clone(), LearnerEvent::KenKenAttempted {
                    correct: was_correct,
                    grid_size,
                    hints_used,
                    constraint_violations: violations,
                    response_time_ms: Some(response_ms),
                });

                // Same payout rule as every activity: earned only by a clean
                // solve. Violations = mistakes; hints stay reward-neutral
                // (asking for help is a behavior we want, not a grind).
                self.finish_puzzle(was_correct, violations as u32, GameEvent::KenKenResolved {
                    correct: was_correct,
                    grid_size,
                    hints_used,
                    constraint_violations: violations,
                    response_ms,
                });
            }
        }
    }

    /// Silent response time since `start_time`, in ms, capped at `cap_ms`.
    /// The kid never sees it (Invariant 4) — it only feeds the adaptive system.
    pub(super) fn elapsed_ms(&self, start_time: f32, cap_ms: f64) -> f64 {
        ((self.game_time - start_time) as f64 * 1000.0).min(cap_ms)
    }

    /// The shared tail of every resolved puzzle: pay out a clean solve,
    /// log the resolution, return to the overworld, save.
    pub(super) fn finish_puzzle(&mut self, correct: bool, mistakes: u32, resolved: GameEvent) {
        if let Some(reward) = rewards::determine_reward(correct, mistakes) {
            self.award_dum_dums(reward.amount);
        }
        self.events.push(resolved);
        self.set_state(GameState::Playing);
        self.persist();
    }

    pub(super) fn step_pattern(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        let mut dismiss = false;
        if let Some(ref mut ap) = self.active_pattern {
            if ap.session.phase == PatternPhase::Complete {
                // Celebrate, then auto-dismiss — or let any input move on.
                ap.complete_timer += dt;
                if ap.complete_timer >= 2.0 {
                    dismiss = true;
                }
                if input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) || input.mouse_clicked {
                    dismiss = true;
                }
            } else {
                let layout = ui::patterns::layout(&ap.session, screen);
                if let Some(ui::patterns::PatternInput::Action(action)) =
                    ui::patterns::handle_key(&ap.session, input)
                {
                    ap.session = patterns::pattern_reducer(ap.session.clone(), action);
                } else if input.mouse_clicked {
                    let (mx, my) = input.mouse_pos;
                    if let Some(ui::patterns::PatternInput::Action(action)) =
                        ui::patterns::handle_click(mx, my, &ap.session, &layout)
                    {
                        ap.session = patterns::pattern_reducer(ap.session.clone(), action);
                    }
                }
            }
        }

        if dismiss {
            if let Some(ap) = self.active_pattern.take() {
                let was_correct = ap.session.phase == PatternPhase::Complete;
                let response_ms = self.elapsed_ms(ap.start_time, 120000.0);
                let level = self.profile.pattern_level;
                let attempts = ap.session.attempts;

                self.profile = learner_reducer(self.profile.clone(), LearnerEvent::PatternAttempted {
                    correct: was_correct,
                    level,
                    attempts,
                    response_time_ms: Some(response_ms),
                });

                // `attempts` counts every guess including the right one, so
                // mistakes = attempts - 1. Guess-grinding pays nothing.
                let mistakes = attempts.saturating_sub(1) as u32;
                self.finish_puzzle(was_correct, mistakes, GameEvent::PatternResolved {
                    correct: was_correct,
                    level,
                    attempts,
                    response_ms,
                });
            }
        }
    }

    pub(super) fn step_balance(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        let mut dismiss = false;
        if let Some(ref mut ab) = self.active_balance {
            if ab.session.phase == BalancePhase::Complete {
                ab.complete_timer += dt;
                if ab.complete_timer >= 2.0 {
                    dismiss = true;
                }
                if input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) || input.mouse_clicked {
                    dismiss = true;
                }
            } else {
                let layout = ui::balance::layout(&ab.session, screen);
                if let Some(ui::balance::BalanceInput::Action(action)) =
                    ui::balance::handle_key(&ab.session, input)
                {
                    ab.session = balance::balance_reducer(ab.session.clone(), action);
                } else if input.mouse_clicked {
                    let (mx, my) = input.mouse_pos;
                    if let Some(ui::balance::BalanceInput::Action(action)) =
                        ui::balance::handle_click(mx, my, &ab.session, &layout)
                    {
                        ab.session = balance::balance_reducer(ab.session.clone(), action);
                    }
                }
            }
        }

        if dismiss {
            if let Some(ab) = self.active_balance.take() {
                let was_correct = ab.session.phase == BalancePhase::Complete;
                let response_ms = self.elapsed_ms(ab.start_time, 120000.0);
                let level = balance::balance_level_for_band(self.profile.math_band);
                let attempts = ab.session.attempts;

                // The balance scale is the grindiest of all — tap every number
                // until it levels. Only a first-guess balance pays.
                let mistakes = attempts.saturating_sub(1) as u32;
                self.finish_puzzle(was_correct, mistakes, GameEvent::BalanceResolved {
                    correct: was_correct,
                    level,
                    attempts,
                    response_ms,
                });
            }
        }
    }

    pub(super) fn step_sudoku(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        let mut dismiss = false;
        if let Some(ref mut asd) = self.active_sudoku {
            if asd.session.phase == SudokuPhase::Complete {
                asd.complete_timer += dt;
                if asd.complete_timer >= 2.5 {
                    dismiss = true;
                }
                if input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) || input.mouse_clicked {
                    dismiss = true;
                }
            } else {
                let layout = ui::sudoku::layout(&asd.session, screen);
                if let Some(intent) = ui::sudoku::handle_key(&asd.session, input, asd.selected) {
                    apply_sudoku_intent(asd, intent);
                }
                if input.mouse_clicked {
                    let (mx, my) = input.mouse_pos;
                    if let Some(intent) = ui::sudoku::handle_click(mx, my, &asd.session, &layout, asd.selected) {
                        apply_sudoku_intent(asd, intent);
                    }
                }
            }
        }

        if dismiss {
            if let Some(asd) = self.active_sudoku.take() {
                let was_correct = asd.session.phase == SudokuPhase::Complete;
                let response_ms = self.elapsed_ms(asd.start_time, 120000.0);
                let grid_size = asd.session.puzzle.grid_size;
                let violations = asd.session.constraint_violations;

                self.finish_puzzle(was_correct, violations as u32, GameEvent::SudokuResolved {
                    correct: was_correct,
                    grid_size,
                    constraint_violations: violations,
                    response_ms,
                });
            }
        }
    }
}

pub(super) fn start_kenken(rng: &mut SmallRng, profile: &LearnerProfile, game_time: f32, source: String) -> ActiveKenKen {
    let grid_size = profile.kenken_level.clamp(2, 4);
    let ops = cage_ops_for_band(profile.math_band);
    let puzzle = generate_kenken(grid_size, &ops, rng);
    let session = KenKenSession::new(puzzle);
    let intro_step = if profile.kenken_intro_seen { None } else { Some(0) };
    ActiveKenKen {
        session,
        selected: None,
        complete_timer: 0.0,
        start_time: game_time,
        source_npc: source,
        intro_step,
    }
}

pub(super) fn start_pattern(rng: &mut SmallRng, profile: &LearnerProfile, game_time: f32, source: String) -> ActivePattern {
    let level = profile.pattern_level.max(1);
    let puzzle = generate_for_level(level, rng);
    ActivePattern {
        session: PatternSession::new(puzzle),
        complete_timer: 0.0,
        start_time: game_time,
        source_npc: source,
    }
}

pub(super) fn start_balance(rng: &mut SmallRng, profile: &LearnerProfile, game_time: f32, source: String) -> ActiveBalance {
    // Balance difficulty rides the arithmetic band — it's the same math in a
    // different visual, so no separate level dial is needed.
    let puzzle = generate_balance_for_band(profile.math_band, rng);
    ActiveBalance {
        session: BalanceSession::new(puzzle),
        complete_timer: 0.0,
        start_time: game_time,
        source_npc: source,
    }
}

pub(super) fn start_sudoku(rng: &mut SmallRng, profile: &LearnerProfile, game_time: f32, source: String) -> ActiveSudoku {
    // Sudoku is pure logic; reuse the kenken level dial as a "logic grid" size
    // signal: a kid comfortable with bigger kenken grids gets the 6x6 board.
    let level = if profile.kenken_level >= 4 { 3 } else { 1 };
    let puzzle = generate_sudoku_for_level(level, rng);
    ActiveSudoku {
        session: SudokuSession::new(puzzle),
        selected: None,
        complete_timer: 0.0,
        start_time: game_time,
        source_npc: source,
    }
}

fn apply_sudoku_intent(asd: &mut ActiveSudoku, intent: ui::sudoku::SudokuInput) {
    use robot_buddy_domain::logic::sudoku::SudokuAction;
    match intent {
        ui::sudoku::SudokuInput::Action(action) => {
            asd.session = sudoku::sudoku_reducer(asd.session.clone(), action);
            // Drop selection after a clean placement; keep it on a conflict so
            // the kid can retry the same cell and the violation stays anchored.
            if let SudokuAction::CellPlaced { .. } = action {
                if asd.session.last_violation.is_none() {
                    asd.selected = None;
                }
            }
        }
        ui::sudoku::SudokuInput::SelectCell(r, c) => {
            asd.selected = Some((r, c));
            asd.session.last_violation = None;
        }
        ui::sudoku::SudokuInput::Deselect => {
            asd.selected = None;
            asd.session.last_violation = None;
        }
    }
}

fn apply_kenken_intent(ak: &mut ActiveKenKen, intent: ui::kenken::KenKenInput) {
    match intent {
        ui::kenken::KenKenInput::Action(action) => {
            ak.session = kenken::kenken_reducer(ak.session.clone(), action.clone());
            // After a valid placement, drop selection so the next picker click
            // doesn't accidentally overwrite the cell. After a rejected
            // placement (row/col conflict — see reducer), keep selection so
            // the kid can immediately try a different number on the same cell
            // and the violation highlight stays anchored.
            if let KenKenAction::CellPlaced { .. } = action {
                if ak.session.last_violation.is_none() {
                    ak.selected = None;
                }
            }
        }
        ui::kenken::KenKenInput::SelectCell(r, c) => {
            ak.selected = Some((r, c));
            // Clear stale violation feedback when changing selection — the
            // last_violation hint encodes a coord relative to the previously
            // selected cell, and would mis-render against a new selection.
            // Inline because last_violation doubles as a UI hint and selection
            // state lives outside the reducer.
            ak.session.last_violation = None;
        }
        ui::kenken::KenKenInput::Deselect => {
            ak.selected = None;
            ak.session.last_violation = None;
        }
    }
}
