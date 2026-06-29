//! Test harness for headless story-style integration tests.
//!
//! Wraps Game with primitive frame-driving helpers (press, type_chars,
//! advance, run_until) and a few story-level helpers built on top.
//! Render is never called, so no macroquad context is needed.

#![allow(dead_code)] // helpers are added as tests need them

use macroquad::prelude::KeyCode;
use robot_buddy_game::game::{Game, GameEvent, GameState};
use robot_buddy_game::input::FrameInput;
use robot_buddy_game::npc::NpcKind;
use robot_buddy_game::save::InMemoryBackend;
use robot_buddy_game::ui;

pub const SCREEN: (f32, f32) = (960.0, 720.0);
pub const DT: f32 = 1.0 / 60.0;

/// Default frame budget for `run_until`. A real loop runs ~60 frames/second,
/// so 600 = 10 seconds of in-game time. Loud panic if a wait exceeds this —
/// usually means the predicate will never trigger.
const DEFAULT_BUDGET: usize = 600;

pub struct Harness {
    pub game: Game,
}

impl Harness {
    /// Start a fresh game seeded for determinism. Each Harness owns its own
    /// in-memory save backend, so tests don't write /tmp and parallel tests
    /// don't share storage.
    pub fn new(seed: u64) -> Self {
        Harness {
            game: Game::with_backend(seed, Box::new(InMemoryBackend::default())),
        }
    }

    // ─── Primitive frame drivers ─────────────────────────

    /// One frame with the given input.
    pub fn step(&mut self, input: &FrameInput) {
        self.game.step(input, DT, SCREEN);
    }

    /// One frame with no input.
    pub fn idle(&mut self) {
        self.step(&FrameInput::empty());
    }

    /// Advance N frames with no input.
    pub fn advance(&mut self, n: usize) {
        for _ in 0..n {
            self.idle();
        }
    }

    /// One frame with `key` pressed.
    pub fn press(&mut self, key: KeyCode) {
        self.step(&FrameInput::empty().with_key_pressed(key));
    }

    /// One frame with `key` held down (used for movement).
    pub fn hold(&mut self, key: KeyCode) {
        self.step(&FrameInput::empty().with_key_down(key));
    }

    /// Type a string into whatever text field is active. One frame per char so
    /// behavior matches a real keyboard (the form caps name length on each frame).
    pub fn type_chars(&mut self, s: &str) {
        for c in s.chars() {
            self.step(&FrameInput::empty().with_char(c));
        }
    }

    /// One frame with a left-click at (x, y).
    pub fn click(&mut self, x: f32, y: f32) {
        self.step(&FrameInput::empty().with_mouse_click(x, y));
    }

    /// Step (with no input) until `pred(&Game)` returns true. Panics if the
    /// predicate is still false after `max_frames`.
    pub fn run_until<F: Fn(&Game) -> bool>(&mut self, pred: F, max_frames: usize) {
        for _ in 0..max_frames {
            if pred(&self.game) { return; }
            self.idle();
        }
        panic!("run_until exceeded {} frames; current state = {:?}", max_frames, self.game.state);
    }

    pub fn wait_until<F: Fn(&Game) -> bool>(&mut self, pred: F) {
        self.run_until(pred, DEFAULT_BUDGET);
    }

    // ─── Story helpers ───────────────────────────────────

    /// From Title: press Key1 to NEW slot 0, fill the form, press Enter to start.
    /// Lands in `Intake` (because Start always routes through intake unless using
    /// the dev-zone code).
    pub fn start_new_game(&mut self, name: &str) {
        assert_eq!(self.game.state, GameState::Title, "start_new_game expects Title");
        self.press(KeyCode::Key1);
        assert_eq!(self.game.state, GameState::NewGame);
        self.type_chars(name);
        self.press(KeyCode::Enter);
        assert_eq!(self.game.state, GameState::Intake);
    }

    /// Skip through new-game form + intake by typing the dev-zone cheat code.
    /// Lands in `Playing` on the dev map with 20 dum_dums and intake marked
    /// completed. Useful for tests that just need a populated game state to
    /// exercise gameplay flows.
    pub fn start_dev_game(&mut self) {
        self.press(KeyCode::Key1);
        assert_eq!(self.game.state, GameState::NewGame);
        self.type_chars("justinbailey");
        self.press(KeyCode::Enter);
        // Dev branch fires an intro dialogue ("Walk around, talk to everyone…")
        // then sits in Dialogue. Advance through it.
        self.wait_until(|g| g.state == GameState::Dialogue);
        self.finish_dialogue();
        self.wait_until(|g| g.state == GameState::Playing);
    }

    /// Press Space until the dialogue box is no longer active. Bounded so a
    /// stuck dialogue doesn't infinite-loop the test.
    pub fn finish_dialogue(&mut self) {
        for _ in 0..200 {
            if !self.game.is_dialogue_active() { return; }
            self.press(KeyCode::Space);
        }
        panic!("dialogue never finished after 200 advances");
    }

    /// Press the number key matching the correct answer, then Space to dismiss
    /// the post-answer celebration. Assumes a challenge is on screen.
    pub fn answer_challenge_correctly(&mut self) {
        let idx = self.game.correct_choice_index()
            .expect("answer_challenge_correctly: no active challenge");
        let key = match idx {
            0 => KeyCode::Key1,
            1 => KeyCode::Key2,
            2 => KeyCode::Key3,
            n => panic!("unexpected correct choice index {}", n),
        };
        self.press(key);
        // The reducer flips phase to Complete this same frame; press Space next
        // frame to skip the 2-second auto-dismiss.
        self.press(KeyCode::Space);
    }

    /// Walk through the entire intake quiz answering every question correctly.
    /// Lands in `Playing`.
    pub fn complete_intake_correctly(&mut self) {
        assert_eq!(self.game.state, GameState::Intake);

        // Advance Sparky's intro dialogue.
        self.finish_dialogue();

        // Intake length is adaptive, so answer questions until it ends (the
        // game leaves Intake for the "all done!" dialogue) rather than a fixed
        // count. Safety-bounded so a stuck intake fails loudly.
        for _ in 0..8 {
            if self.game.state != GameState::Intake {
                break;
            }
            // Either a question is on screen, or we're between questions.
            self.run_until(
                |g| g.correct_choice_index().is_some() || g.state != GameState::Intake,
                DEFAULT_BUDGET,
            );
            if self.game.state != GameState::Intake {
                break;
            }
            self.answer_challenge_correctly();
        }

        // After the last question we land in Dialogue ("all done!"). Advance it.
        self.wait_until(|g| g.state == GameState::Dialogue);
        self.finish_dialogue();
        self.wait_until(|g| g.state == GameState::Playing);
    }

    /// Walk the player to the given tile coordinates via BFS over the map.
    /// Holds the relevant arrow key each frame until the player advances one
    /// tile, then moves to the next step. Panics if there's no path.
    pub fn walk_to(&mut self, target_x: usize, target_y: usize) {
        let start = (self.game.player.tile_x, self.game.player.tile_y);
        if start == (target_x, target_y) { return; }
        let path = bfs(start, (target_x, target_y), &self.game)
            .unwrap_or_else(|| panic!("no path from {:?} to ({},{})", start, target_x, target_y));

        for (nx, ny) in path {
            let dx = nx as i32 - self.game.player.tile_x as i32;
            let dy = ny as i32 - self.game.player.tile_y as i32;
            let key = match (dx, dy) {
                (0, -1) => KeyCode::Up,
                (0, 1)  => KeyCode::Down,
                (-1, 0) => KeyCode::Left,
                (1, 0)  => KeyCode::Right,
                _ => panic!("non-cardinal step in path: ({},{})", dx, dy),
            };
            // ~16 frames at 60fps to cross one tile (TILE_SIZE 48 / MOVE_SPEED 200).
            // Budget 30 frames to be safe; break early once we've arrived.
            let mut arrived = false;
            for _ in 0..30 {
                if self.game.player.tile_x == nx
                    && self.game.player.tile_y == ny
                    && self.game.player_at_rest()
                {
                    arrived = true;
                    break;
                }
                self.hold(key);
            }
            if !arrived {
                panic!("walk_to: failed to reach tile ({},{}) — stuck at ({},{})",
                    nx, ny, self.game.player.tile_x, self.game.player.tile_y);
            }
        }
    }

    /// Walk to a tile adjacent to the given NPC kind, then face them. Combine
    /// with `interact()` to start a conversation.
    pub fn walk_to_npc(&mut self, kind: NpcKind) {
        let (nx, ny) = self.game.npcs.iter()
            .find(|n| n.kind == kind)
            .map(|n| (n.entity.tile_x, n.entity.tile_y))
            .unwrap_or_else(|| panic!("no NPC with kind {:?} on current map ('{}')", kind, self.game.map.id));

        // Pick a walkable adjacent tile.
        let candidates = [
            (nx.wrapping_sub(1), ny),
            (nx + 1, ny),
            (nx, ny.wrapping_sub(1)),
            (nx, ny + 1),
        ];
        let adj = candidates.iter().copied().find(|&(cx, cy)| {
            cx < self.game.map.width
                && cy < self.game.map.height
                && !self.game.map.is_solid(cx, cy)
                && !self.game.npcs.iter().any(|n| n.entity.tile_x == cx && n.entity.tile_y == cy)
        }).unwrap_or_else(|| panic!("no walkable tile adjacent to NPC {:?}", kind));

        self.walk_to(adj.0, adj.1);

        // Face the NPC by holding the arrow key toward them for one frame.
        // The game's movement code only starts a move if the destination is
        // walkable; pressing into an NPC just sets `dir`, which is what we want.
        let dx = nx as i32 - adj.0 as i32;
        let dy = ny as i32 - adj.1 as i32;
        let face_key = match (dx, dy) {
            (0, -1) => KeyCode::Up,
            (0, 1)  => KeyCode::Down,
            (-1, 0) => KeyCode::Left,
            (1, 0)  => KeyCode::Right,
            _ => unreachable!(),
        };
        self.hold(face_key);
    }

    /// Press Space — opens the interaction menu (or the pre-challenge dialogue
    /// for chests). Caller is expected to be standing adjacent to and facing
    /// the target.
    pub fn interact(&mut self) {
        self.press(KeyCode::Space);
    }

    /// In `InteractionMenu`, press the number key matching the option whose
    /// `option_type` equals `opt_type` (typically "talk", "give", "shop").
    pub fn select_option(&mut self, opt_type: &str) {
        assert_eq!(self.game.state, GameState::InteractionMenu,
            "select_option expects InteractionMenu, got {:?}", self.game.state);
        let key_no = self.game.menu_options.iter()
            .find(|o| o.option_type == opt_type)
            .map(|o| o.key)
            .unwrap_or_else(|| panic!("no '{}' option in menu (have: {:?})",
                opt_type,
                self.game.menu_options.iter().map(|o| &o.option_type).collect::<Vec<_>>()));
        let key = match key_no {
            1 => KeyCode::Key1,
            2 => KeyCode::Key2,
            3 => KeyCode::Key3,
            4 => KeyCode::Key4,
            5 => KeyCode::Key5,
            n => panic!("unsupported option key {}", n),
        };
        self.press(key);
    }

    /// Convenience alias matching the user's example wording.
    pub fn answer_correctly(&mut self) {
        self.answer_challenge_correctly();
    }

    /// Hold a direction key until the player's current map id changes to
    /// `dest_map`. Use when stepping onto a portal tile — `walk_to` panics
    /// in that case because the player lands on the destination map (so the
    /// arrival check against the source coords never fires).
    pub fn step_through_portal(&mut self, dir: KeyCode, dest_map: &str) {
        for _ in 0..60 {
            if self.game.map.id == dest_map {
                return;
            }
            self.hold(dir);
        }
        panic!("never transitioned to map '{}' (still on '{}')", dest_map, self.game.map.id);
    }

    // ─── KenKen helpers ──────────────────────────────────

    /// Place `value` at (row, col) in the active KenKen by clicking the cell
    /// then clicking the matching number picker. Mirrors how a real player
    /// would interact: cell select → value pick. Auto-skips intro if showing.
    pub fn place_kenken_cell(&mut self, row: u8, col: u8, value: u8) {
        self.skip_kenken_intro();
        let (cell_x, cell_y, picker_x, picker_y) = {
            let ak = self.game.active_kenken().expect("place_kenken_cell: no active KenKen");
            let layout = ui::kenken::layout(&ak.session, SCREEN);
            let cell = layout.cells[row as usize][col as usize];
            let picker = &layout.pickers[(value - 1) as usize].rect;
            (
                cell.x + cell.w / 2.0,
                cell.y + cell.h / 2.0,
                picker.x + picker.w / 2.0,
                picker.y + picker.h / 2.0,
            )
        };
        self.click(cell_x, cell_y);
        self.click(picker_x, picker_y);
    }

    /// Click the Hint button on the active KenKen. Auto-skips intro if showing.
    pub fn request_kenken_hint(&mut self) {
        self.skip_kenken_intro();
        let (x, y) = {
            let ak = self.game.active_kenken().expect("request_kenken_hint: no active KenKen");
            let layout = ui::kenken::layout(&ak.session, SCREEN);
            (layout.hint_btn.x + layout.hint_btn.w / 2.0,
             layout.hint_btn.y + layout.hint_btn.h / 2.0)
        };
        self.click(x, y);
    }

    /// If the first-time intro overlay is showing, tap Space until it
    /// finishes. No-op once `kenken_intro_seen` is set on the profile.
    pub fn skip_kenken_intro(&mut self) {
        // Bound the loop — INTRO_STEPS plus slack — so a stuck intro can't
        // hang the test forever.
        for _ in 0..(ui::kenken::INTRO_STEPS as usize + 4) {
            let still_in_intro = self
                .game
                .active_kenken()
                .map(|ak| ak.intro_step.is_some())
                .unwrap_or(false);
            if !still_in_intro {
                return;
            }
            self.press(macroquad::prelude::KeyCode::Space);
        }
        panic!("kenken intro never finished after enough advances");
    }

    /// Fill in every empty cell with the puzzle's known solution. After the
    /// final cell, presses Space to dismiss the celebration screen and lands
    /// back in `Playing`. Auto-skips the first-time intro overlay if showing.
    pub fn solve_kenken_correctly(&mut self) {
        self.skip_kenken_intro();

        let fills = {
            let ak = self.game.active_kenken().expect("solve_kenken_correctly: no active KenKen");
            let n = ak.session.puzzle.grid_size as usize;
            let mut fills: Vec<(u8, u8, u8)> = Vec::new();
            for r in 0..n {
                for c in 0..n {
                    if ak.session.grid[r][c].is_none() {
                        fills.push((r as u8, c as u8, ak.session.puzzle.solution[r][c]));
                    }
                }
            }
            fills
        };
        for (r, c, v) in fills {
            self.place_kenken_cell(r, c, v);
        }
        // Dismiss completion screen.
        self.press(KeyCode::Space);
        self.wait_until(|g| g.state == GameState::Playing);
    }

    // ─── Pattern helpers ─────────────────────────────────

    /// Click the choice at `index` in the active pattern puzzle.
    pub fn select_pattern_choice(&mut self, index: usize) {
        let (x, y) = {
            let ap = self.game.active_pattern().expect("select_pattern_choice: no active pattern");
            let layout = ui::patterns::layout(&ap.session, SCREEN);
            let rect = layout.choices.iter().find(|c| c.index == index)
                .expect("select_pattern_choice: choice index out of range").rect;
            (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0)
        };
        self.click(x, y);
    }

    /// Pick a deliberately wrong choice (no-op if the puzzle has only the
    /// answer, which never happens). Lets tests exercise the bounce-back path.
    pub fn select_wrong_pattern_choice(&mut self) {
        let wrong = {
            let ap = self.game.active_pattern().expect("select_wrong_pattern_choice: no active pattern");
            let correct = ap.session.puzzle.correct_choice();
            (0..ap.session.puzzle.choices.len()).find(|&i| i != correct)
                .expect("pattern must have at least one wrong choice")
        };
        self.select_pattern_choice(wrong);
    }

    /// Solve the active pattern by clicking the correct choice, then dismiss the
    /// celebration and land back in Playing.
    pub fn solve_pattern_correctly(&mut self) {
        let correct = {
            let ap = self.game.active_pattern().expect("solve_pattern_correctly: no active pattern");
            ap.session.puzzle.correct_choice()
        };
        self.select_pattern_choice(correct);
        self.press(KeyCode::Space);
        self.wait_until(|g| g.state == GameState::Playing);
    }

    // ─── Balance helpers ─────────────────────────────────

    /// Click the choice whose value equals `value` in the active balance puzzle.
    pub fn select_balance_value(&mut self, value: i32) {
        let (x, y) = {
            let ab = self.game.active_balance().expect("select_balance_value: no active balance");
            let layout = ui::balance::layout(&ab.session, SCREEN);
            let rect = layout.choices.iter().find(|c| c.value == value)
                .unwrap_or_else(|| panic!("select_balance_value: no choice {value}")).rect;
            (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0)
        };
        self.click(x, y);
    }

    /// Pick a deliberately wrong value to exercise the tip-and-retry path.
    pub fn select_wrong_balance_value(&mut self) {
        let wrong = {
            let ab = self.game.active_balance().expect("select_wrong_balance_value: no active balance");
            let answer = ab.session.puzzle.correct_answer;
            ab.session.puzzle.choices.iter().copied().find(|&v| v != answer)
                .expect("balance must have a wrong choice")
        };
        self.select_balance_value(wrong);
    }

    /// Solve the active balance by guessing the correct answer, then dismiss the
    /// celebration and land back in Playing.
    pub fn solve_balance_correctly(&mut self) {
        let answer = {
            let ab = self.game.active_balance().expect("solve_balance_correctly: no active balance");
            ab.session.puzzle.correct_answer
        };
        self.select_balance_value(answer);
        self.press(KeyCode::Space);
        self.wait_until(|g| g.state == GameState::Playing);
    }

    // ─── Sudoku helpers ──────────────────────────────────

    /// Place `value` at (row, col) in the active Sudoku by clicking the cell
    /// then the matching picker — same cell-select → value-pick flow as KenKen.
    pub fn place_sudoku_cell(&mut self, row: u8, col: u8, value: u8) {
        let (cell_x, cell_y, picker_x, picker_y) = {
            let asd = self.game.active_sudoku().expect("place_sudoku_cell: no active Sudoku");
            let layout = ui::sudoku::layout(&asd.session, SCREEN);
            let cell = layout.cells[row as usize][col as usize];
            let picker = &layout.pickers[(value - 1) as usize].rect;
            (cell.x + cell.w / 2.0, cell.y + cell.h / 2.0,
             picker.x + picker.w / 2.0, picker.y + picker.h / 2.0)
        };
        self.click(cell_x, cell_y);
        self.click(picker_x, picker_y);
    }

    /// Fill every blank cell with the known solution, then dismiss the
    /// celebration and land back in Playing.
    pub fn solve_sudoku_correctly(&mut self) {
        let fills = {
            let asd = self.game.active_sudoku().expect("solve_sudoku_correctly: no active Sudoku");
            let n = asd.session.puzzle.grid_size as usize;
            let mut fills: Vec<(u8, u8, u8)> = Vec::new();
            for r in 0..n {
                for c in 0..n {
                    if asd.session.grid[r][c].is_none() {
                        fills.push((r as u8, c as u8, asd.session.puzzle.solution[r][c]));
                    }
                }
            }
            fills
        };
        for (r, c, v) in fills {
            self.place_sudoku_cell(r, c, v);
        }
        self.press(KeyCode::Space);
        self.wait_until(|g| g.state == GameState::Playing);
    }

    // ─── Shop helpers ────────────────────────────────────

    /// Click the catalog row for the item with `item_id` in the open shop.
    pub fn select_shop_item(&mut self, item_id: &str) {
        let (x, y) = {
            let ash = self.game.active_shop().expect("select_shop_item: shop not open");
            // Browsing view (no item selected yet).
            let view = ui::shop::ShopView::Browsing;
            let layout = ui::shop::layout(&ash.catalog, &view, SCREEN);
            let idx = ash.catalog.iter().position(|i| i.id == item_id)
                .unwrap_or_else(|| panic!("no shop item {item_id}"));
            let row = layout.items.iter().find(|r| r.index == idx)
                .expect("item row not visible (already buying?)").rect;
            (row.x + row.w / 2.0, row.y + row.h / 2.0)
        };
        self.click(x, y);
    }

    /// Tap the answer tile with value `value` during a purchase subtraction.
    pub fn answer_shop_math(&mut self, value: u32) {
        let (x, y) = {
            let ash = self.game.active_shop().expect("answer_shop_math: shop not open");
            let i = ash.selected.expect("answer_shop_math: not currently buying");
            let view = ui::shop::ShopView::Buying {
                item: &ash.catalog[i],
                balance: ash.balance_before,
                cost: ash.cost,
                choices: &ash.choices,
            };
            let layout = ui::shop::layout(&ash.catalog, &view, SCREEN);
            let tile = layout.answers.iter().find(|t| t.value == value)
                .unwrap_or_else(|| panic!("no answer tile {value}")).rect;
            (tile.x + tile.w / 2.0, tile.y + tile.h / 2.0)
        };
        self.click(x, y);
    }

    /// Buy `item_id` end-to-end: select it, then tap the correct remainder.
    pub fn buy_shop_item(&mut self, item_id: &str) {
        self.select_shop_item(item_id);
        let answer = {
            let ash = self.game.active_shop().expect("buy_shop_item: shop not open");
            ash.answer
        };
        self.answer_shop_math(answer);
    }

    /// Click the shop's "Done" button to leave and return to Playing.
    pub fn close_shop(&mut self) {
        let (x, y) = {
            let ash = self.game.active_shop().expect("close_shop: shop not open");
            let view = match ash.selected {
                Some(i) => ui::shop::ShopView::Buying {
                    item: &ash.catalog[i], balance: ash.balance_before, cost: ash.cost, choices: &ash.choices,
                },
                None => ui::shop::ShopView::Browsing,
            };
            let layout = ui::shop::layout(&ash.catalog, &view, SCREEN);
            (layout.close_btn.x + layout.close_btn.w / 2.0, layout.close_btn.y + layout.close_btn.h / 2.0)
        };
        self.click(x, y);
        self.wait_until(|g| g.state == GameState::Playing);
    }

    // ─── CRA manipulative helpers ────────────────────────

    fn click_manip_button(&mut self, label: &str) {
        let (x, y) = {
            let am = self.game.active_manipulative().expect("click_manip_button: no manipulative");
            let layout = ui::manipulative::layout(&am.manip, SCREEN);
            let b = layout.buttons.iter().find(|b| b.label == label)
                .unwrap_or_else(|| panic!("no manip button '{label}'"));
            (b.rect.x + b.rect.w / 2.0, b.rect.y + b.rect.h / 2.0)
        };
        self.click(x, y);
    }

    /// Drive the active CRA manipulative to completion with the correct moves,
    /// then dismiss and land back in Playing.
    pub fn solve_manipulative(&mut self) {
        use robot_buddy_game::ui::manipulative::Manip;
        use robot_buddy_domain::logic::manipulate_concrete::ConcreteKind;
        let labels: Vec<&'static str> = {
            let am = self.game.active_manipulative().expect("solve_manipulative: no manipulative");
            match &am.manip {
                Manip::Concrete(s) => match s.puzzle.kind {
                    ConcreteKind::AddGroups => {
                        let mut v = vec!["+ Red"; s.puzzle.a as usize];
                        v.extend(std::iter::repeat("+ Blue").take(s.puzzle.b as usize));
                        v
                    }
                    ConcreteKind::Count | ConcreteKind::BuildTower => {
                        vec!["Add one"; s.puzzle.target as usize]
                    }
                    ConcreteKind::TakeAway => vec!["Take one"; s.puzzle.b as usize],
                },
            }
        };
        for label in labels {
            self.click_manip_button(label);
        }
        self.press(KeyCode::Space);
        self.wait_until(|g| g.state == GameState::Playing);
    }

    // ─── Settings / parent overlay helpers ──────────────

    /// Open the settings overlay (the in-game gear; T key).
    pub fn open_settings(&mut self) {
        self.press(KeyCode::T);
    }

    /// Click the center of map tile `(col, row)` — inverts the camera transform
    /// (`screen = world - camera`) so it lands on that tile for click-to-walk.
    pub fn click_tile(&mut self, col: usize, row: usize) {
        const TILE: f32 = 48.0;
        let sx = (col as f32 + 0.5) * TILE - self.game.camera.x;
        let sy = (row as f32 + 0.5) * TILE - self.game.camera.y;
        self.click(sx, sy);
    }

    /// Click the "Parent options" reveal row in the open settings overlay.
    pub fn click_parent_options(&mut self) {
        let (x, y) = robot_buddy_game::ui::settings_overlay::parent_toggle_center(SCREEN);
        self.click(x, y);
    }

    /// Click a feature toggle in the (revealed) parent section.
    pub fn toggle_feature_in_settings(&mut self, feature: robot_buddy_game::ui::settings_overlay::Feature) {
        let (x, y) = robot_buddy_game::ui::settings_overlay::feature_toggle_center(SCREEN, feature);
        self.click(x, y);
    }

    // ─── Quest helpers ───────────────────────────────────

    /// Play the active quest to completion: tap Continue through narrative /
    /// travel / reward beats, and answer each embedded MathPuzzle correctly
    /// (picking the answer's tile by number key). Lands back in Playing.
    pub fn play_quest(&mut self) {
        use robot_buddy_domain::quest::QuestStep;
        for _ in 0..80 {
            // Number-key index to press: the answer tile for a MathPuzzle, or
            // the first option for a Choice. `None` = a Continue beat (Space).
            let key_idx = {
                let Some(aq) = self.game.active_quest() else { break };
                match aq.session.current_step() {
                    Some(QuestStep::MathPuzzle { .. }) => {
                        let p = aq.puzzle.as_ref().expect("puzzle step must have generated choices");
                        Some(p.choices.iter().position(|&c| c == p.answer)
                            .expect("answer must be among the choices"))
                    }
                    Some(QuestStep::Choice { .. }) => Some(0), // pick the first option
                    Some(_) => None,                            // narrative / travel / reward
                    None => break,
                }
            };
            match key_idx {
                Some(0) => self.press(KeyCode::Key1),
                Some(1) => self.press(KeyCode::Key2),
                Some(2) => self.press(KeyCode::Key3),
                Some(3) => self.press(KeyCode::Key4),
                Some(_) => panic!("unexpected tile index"),
                None => self.press(KeyCode::Space),
            }
        }
        self.wait_until(|g| g.state == GameState::Playing);
    }

    // ─── Event-log access ────────────────────────────────

    /// Snapshot the event log length. Capture before an action, then read
    /// the events that action produced via `events_since(mark)`.
    pub fn mark(&self) -> usize {
        self.game.event_mark()
    }

    /// Events emitted since the given mark.
    pub fn events_since(&self, mark: usize) -> &[GameEvent] {
        self.game.events_since(mark)
    }
}

// ─── BFS pathfinding over the map grid ──────────────────

fn bfs(
    start: (usize, usize),
    goal: (usize, usize),
    game: &Game,
) -> Option<Vec<(usize, usize)>> {
    use std::collections::{HashMap, VecDeque};

    let mut q: VecDeque<(usize, usize)> = VecDeque::new();
    let mut prev: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
    q.push_back(start);
    prev.insert(start, start);

    while let Some(p) = q.pop_front() {
        if p == goal {
            let mut path = Vec::new();
            let mut cur = goal;
            while cur != start {
                path.push(cur);
                cur = prev[&cur];
            }
            path.reverse();
            return Some(path);
        }
        let (px, py) = p;
        let cands = [
            (px as i32, py as i32 - 1),
            (px as i32, py as i32 + 1),
            (px as i32 - 1, py as i32),
            (px as i32 + 1, py as i32),
        ];
        for (nx, ny) in cands {
            if nx < 0 || ny < 0 { continue; }
            let np = (nx as usize, ny as usize);
            if np.0 >= game.map.width || np.1 >= game.map.height { continue; }
            if prev.contains_key(&np) { continue; }
            // Block solid tiles and NPCs. Sparky is mobile and can be pushed past;
            // we don't model him here.
            if game.map.is_solid(np.0, np.1) { continue; }
            if game.npcs.iter().any(|n| n.entity.tile_x == np.0 && n.entity.tile_y == np.1) { continue; }
            prev.insert(np, p);
            q.push_back(np);
        }
    }
    None
}
