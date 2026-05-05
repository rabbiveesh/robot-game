//! The Game struct: all state, all logic, all rendering.
//!
//! Production: `main()` captures input from macroquad, calls `step()`, awaits next_frame.
//! Tests: build a `FrameInput` synthetically and call `step()` directly. (Tests still need
//! a macroquad window today because draw calls run unconditionally — Phase 4 will split.)

use macroquad::prelude::*;
use ::rand::{Rng, SeedableRng};
use ::rand::rngs::SmallRng;
use std::collections::HashMap;

use robot_buddy_domain::challenge::challenge_state::{
    ChallengeState, DisplaySpeech, RenderHint, VoiceState,
    challenge_reducer,
};
use robot_buddy_domain::learning::challenge_generator::{
    Challenge, ChallengeProfile, generate_challenge,
};
use robot_buddy_domain::learning::learner_profile::{
    LearnerProfile, LearnerEvent, learner_reducer,
};
use robot_buddy_domain::learning::frustration_detector::{
    BehaviorSignal, detect_frustration,
};
use robot_buddy_domain::learning::intake_assessor::{
    IntakeAnswer, generate_intake_question, process_intake_results, next_intake_band,
};
use robot_buddy_domain::economy::give;
use robot_buddy_domain::economy::interaction_options::{self, NpcInfo, PlayerState};
use robot_buddy_domain::logic::kenken::{
    self, KenKenAction, KenKenPhase, KenKenSession, cage_ops_for_band, generate_kenken,
};
use robot_buddy_domain::types::{Phase, CraStage, FrustrationLevel};
use robot_buddy_domain::world::movement::{
    Direction, EntityId, EntityState, GridDims, MoveIntent, MoveResolution,
    Solidity, resolve_moves,
};

use crate::tilemap::{self, Map, TILE_SIZE};
use crate::sprites::{self, Dir};
use crate::npc;
use crate::ui;
use crate::ui::dialogue::{DialogueBox, DialogueLine};
use crate::ui::challenge::{ChoiceBound, ScaffoldBounds};
use crate::ui::title_screen::{TitleAction, NewGameAction, NewGameForm};
use crate::ui::hud::{DumDumHud, DebugOverlay};
use crate::ui::interaction_menu::MenuOption;
use crate::save::{self, SaveBackend, SaveData, SaveSlots, Gender};
use crate::audio;
use crate::session;
use crate::input::FrameInput;

pub const GAME_W: f32 = 960.0;
pub const GAME_H: f32 = 720.0;
const MOVE_SPEED: f32 = 200.0;
const INTAKE_QUESTION_COUNT: usize = 5;

// ─── Top-level state machine ────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GameState {
    Title,
    NewGame,
    Intake,
    Playing,
    InteractionMenu,
    Dialogue,
    Challenge,
    KenKen,
}

#[derive(PartialEq, Debug)]
enum IntakePhase {
    Intro,
    Question,
    Transition,
    Complete,
}

struct IntakeState {
    question_index: usize,
    current_band: u8,
    configured_band: u8,
    answers: Vec<IntakeAnswer>,
    challenge: Option<ActiveChallenge>,
    phase: IntakePhase,
    text_skipped_count: usize,
}

impl IntakeState {
    fn new(configured_band: u8) -> Self {
        IntakeState {
            question_index: 0,
            current_band: configured_band.max(1).min(10),
            configured_band,
            answers: Vec::new(),
            challenge: None,
            phase: IntakePhase::Intro,
            text_skipped_count: 0,
        }
    }
}

struct ActiveChallenge {
    state: ChallengeState,
    challenge: Challenge,
    choice_bounds: Vec<ChoiceBound>,
    scaffold: ScaffoldBounds,
    complete_timer: f32,
    start_time: f32,
}

pub struct ActiveKenKen {
    pub session: KenKenSession,
    pub selected: Option<(u8, u8)>,
    pub complete_timer: f32,
    pub start_time: f32,
    pub source_npc: String,
    /// `Some(n)` while the first-time intro overlay is showing on step `n`.
    /// `None` once the kid has tapped past the last step (or already saw the
    /// intro previously). Domain reducer never sees this — it's UI-only state.
    pub intro_step: Option<u8>,
}

// ─── Sprites/movement ───────────────────────────────────

#[derive(Clone)]
pub struct Entity {
    pub x: f32,
    pub y: f32,
    pub tile_x: usize,
    pub tile_y: usize,
    pub target_x: f32,
    pub target_y: f32,
    pub moving: bool,
    pub dir: Dir,
    pub frame: u32,
}

impl Entity {
    pub fn new(tile_x: usize, tile_y: usize) -> Self {
        Entity {
            x: tile_x as f32 * TILE_SIZE,
            y: tile_y as f32 * TILE_SIZE,
            tile_x,
            tile_y,
            target_x: tile_x as f32 * TILE_SIZE,
            target_y: tile_y as f32 * TILE_SIZE,
            moving: false,
            dir: Dir::Down,
            frame: 0,
        }
    }

    pub fn move_toward_target(&mut self, dt: f32) -> bool {
        if !self.moving { return false; }
        let dx = self.target_x - self.x;
        let dy = self.target_y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = MOVE_SPEED * dt;
        // Clamp to the remaining distance. Without this, a single huge dt
        // (e.g. browser tab regaining focus after being backgrounded) sends
        // pixel position thousands of px past the target, and subsequent
        // normal-dt frames "ghost walk" the entity slowly back toward its
        // tile target. Treat any step that would reach or pass target as
        // arrival.
        if step >= dist || dist < 2.0 {
            self.x = self.target_x;
            self.y = self.target_y;
            self.moving = false;
            self.frame += 1;
            return true;
        }
        self.x += dx / dist * step;
        self.y += dy / dist * step;
        false
    }

    pub fn start_move(&mut self, nx: usize, ny: usize) {
        self.tile_x = nx;
        self.tile_y = ny;
        self.target_x = nx as f32 * TILE_SIZE;
        self.target_y = ny as f32 * TILE_SIZE;
        self.moving = true;
    }
}

pub struct Sparky {
    pub entity: Entity,
    follow_queue: Vec<(usize, usize)>,
}

impl Sparky {
    fn new(tile_x: usize, tile_y: usize) -> Self {
        Sparky {
            entity: Entity::new(tile_x, tile_y),
            follow_queue: Vec::new(),
        }
    }

    fn record_player_pos(&mut self, tx: usize, ty: usize) {
        if self.follow_queue.last() != Some(&(tx, ty)) {
            self.follow_queue.push((tx, ty));
        }
    }

    /// Pixel-level interpolation toward the current target. Pure animation,
    /// no movement decisions. The decision lives in `next_intent`.
    fn animate(&mut self, dt: f32) {
        self.entity.move_toward_target(dt);
    }

    /// Decide what Sparky wants to do this frame. Called once per frame while
    /// stationary; returns `Stay` if mid-step or queue-empty. Sets `dir` as a
    /// side-effect so Sparky faces the player even when not moving (or when
    /// the resolver later denies the move).
    ///
    /// Pops the queue ONLY when returning a Move intent — if the resolver
    /// denies the move, the apply phase doesn't pop, so Sparky retries next
    /// frame.
    fn next_intent(&mut self, player_tx: usize, player_ty: usize) -> MoveIntent {
        if self.entity.moving { return MoveIntent::Stay; }
        if self.follow_queue.is_empty() { return MoveIntent::Stay; }

        // Already adjacent: drop the queue, just face the player.
        let dx_abs = (self.entity.tile_x as i32 - player_tx as i32).abs();
        let dy_abs = (self.entity.tile_y as i32 - player_ty as i32).abs();
        if dx_abs + dy_abs <= 1 {
            self.follow_queue.clear();
            let fdx = player_tx as i32 - self.entity.tile_x as i32;
            let fdy = player_ty as i32 - self.entity.tile_y as i32;
            if fdx < 0 { self.entity.dir = Dir::Left; }
            else if fdx > 0 { self.entity.dir = Dir::Right; }
            else if fdy < 0 { self.entity.dir = Dir::Up; }
            else if fdy > 0 { self.entity.dir = Dir::Down; }
            return MoveIntent::Stay;
        }

        // Peek the next queue entry. Don't pop -- the apply phase pops on grant.
        let (nx, ny) = self.follow_queue[0];
        if nx == player_tx && ny == player_ty {
            // Next step would land on the player. Skip it and try again next frame.
            self.follow_queue.remove(0);
            return MoveIntent::Stay;
        }
        let dx = nx as i32 - self.entity.tile_x as i32;
        let dy = ny as i32 - self.entity.tile_y as i32;
        let dir = match (dx.signum(), dy.signum()) {
            (-1, 0) => Direction::Left,
            ( 1, 0) => Direction::Right,
            (0, -1) => Direction::Up,
            (0,  1) => Direction::Down,
            _ => return MoveIntent::Stay,
        };
        self.entity.dir = match dir {
            Direction::Up => Dir::Up,
            Direction::Down => Dir::Down,
            Direction::Left => Dir::Left,
            Direction::Right => Dir::Right,
        };
        MoveIntent::Move(dir)
    }
}

pub struct GameCamera {
    pub x: f32,
    pub y: f32,
}

impl GameCamera {
    fn follow(&mut self, target_x: f32, target_y: f32, map: &Map, view_w: f32, view_h: f32) {
        self.x = target_x - view_w / 2.0 + TILE_SIZE / 2.0;
        self.y = target_y - view_h / 2.0 + TILE_SIZE / 2.0;
        self.x = self.x.max(0.0).min((map.pixel_width() - view_w).max(0.0));
        self.y = self.y.max(0.0).min((map.pixel_height() - view_h).max(0.0));
    }
}

// ─── Diagnostic events ──────────────────────────────────
//
// step() pushes events for state transitions and notable side-effects. These are
// the assertion surface for tests + diagnostics for failures. Add cases as
// tests demand them — don't speculatively enumerate.

#[derive(Clone, Debug)]
#[allow(dead_code)] // payloads consumed by the upcoming test harness
pub enum GameEvent {
    StateChanged { from: GameState, to: GameState },
    DialogueStarted { speaker: String, text: String },
    DialogueAdvanced,
    ChallengeStarted { question: String },
    ChallengeResolved { correct: bool, response_ms: f64 },
    GiftGiven { recipient_id: String, total: u32 },
    DumDumsAwarded { amount: u32 },
    MapTransitioned { from: String, to: String },
    IntakeCompleted { math_band: u8 },
    KenKenStarted { grid_size: u8, source: String },
    KenKenResolved {
        correct: bool,
        grid_size: u8,
        hints_used: u8,
        constraint_violations: u8,
        response_ms: f64,
    },
}

// ─── The Game ───────────────────────────────────────────

pub struct Game {
    // World
    pub map: Map,
    pub player: Entity,
    pub sparky: Sparky,
    pub camera: GameCamera,
    pub npcs: Vec<npc::Npc>,
    /// NPCs that belong to maps the player isn't on right now. Wandering NPCs
    /// who stepped through a portal accumulate here under the destination map
    /// id; on map change we swap the current `npcs` vec with whatever's stashed
    /// for the new map (or fall back to `npcs_for_map`'s default roster on
    /// first visit). Reset on new game / load — saves only persist the current
    /// map's NPC layout, off-map wanderers snap back to defaults.
    pub npcs_offstage: HashMap<String, Vec<npc::Npc>>,
    pub dreaming: bool,

    // Time
    pub game_time: f32,
    pub play_time: f32,

    // State machine
    pub state: GameState,
    intake: Option<IntakeState>,
    active_challenge: Option<ActiveChallenge>,
    active_kenken: Option<ActiveKenKen>,
    pending_challenge: bool,
    new_game_form: Option<NewGameForm>,

    // Save / persistence
    pub player_name: String,
    pub player_gender: Gender,
    pub dum_dums: u32,
    pub gifts_given: HashMap<String, u32>,
    save_slots: SaveSlots,
    active_slot: usize,
    auto_save_timer: f32,
    save_backend: Box<dyn SaveBackend>,

    // Profile / learning
    pub profile: LearnerProfile,
    behavior_signals: Vec<BehaviorSignal>,

    // UI / overlays
    dialogue: DialogueBox,
    pub menu_options: Vec<MenuOption>,
    menu_target_id: String,
    menu_target_name: String,
    menu_can_challenge: bool,
    dum_dum_hud: DumDumHud,
    debug_overlay: DebugOverlay,
    settings_open: bool,

    // Soft-block pressure per entity (driver of `Solidity::SoftAfter`).
    // Today only Sparky uses it; pressure accumulates while the player walks
    // into him and clears once the player either changes direction or moves.
    pressure: HashMap<EntityId, f32>,

    // Diagnostics + RNG
    rng: SmallRng,
    pub events: Vec<GameEvent>,
    pub session_log: session::SessionLog,
}

impl Game {
    /// Construct a fresh game using the production save backend (browser
    /// localStorage on WASM, /tmp file on native dev). Does not touch storage
    /// at construction; production callers follow up with `refresh_save_slots()`
    /// to populate the title screen. Tests skip that and start empty.
    pub fn new(seed: u64) -> Self {
        Self::with_backend(seed, Box::new(save::LocalStorageBackend))
    }

    /// Construct a fresh game with a caller-supplied save backend. Tests pass
    /// `InMemoryBackend` so each game owns isolated storage with no /tmp races
    /// and no cross-test contamination.
    pub fn with_backend(seed: u64, save_backend: Box<dyn SaveBackend>) -> Self {
        let map = Map::overworld();
        let npcs = npc::npcs_for_map(map.id);
        Game {
            map,
            player: Entity::new(14, 12),
            sparky: Sparky::new(14, 13),
            camera: GameCamera { x: 0.0, y: 0.0 },
            npcs,
            npcs_offstage: HashMap::new(),
            dreaming: false,
            game_time: 0.0,
            play_time: 0.0,
            state: GameState::Title,
            intake: None,
            active_challenge: None,
            active_kenken: None,
            pending_challenge: false,
            new_game_form: None,
            player_name: String::new(),
            player_gender: Gender::Boy,
            dum_dums: 0,
            gifts_given: HashMap::new(),
            save_slots: [None, None, None],
            active_slot: 0,
            auto_save_timer: 0.0,
            save_backend,
            profile: LearnerProfile::new(),
            behavior_signals: Vec::new(),
            dialogue: DialogueBox::new(),
            menu_options: Vec::new(),
            menu_target_id: String::new(),
            menu_target_name: String::new(),
            menu_can_challenge: false,
            dum_dum_hud: DumDumHud::new(),
            debug_overlay: DebugOverlay::new(),
            settings_open: false,
            pressure: HashMap::new(),
            rng: SmallRng::seed_from_u64(seed),
            events: Vec::new(),
            session_log: session::SessionLog::new(),
        }
    }

    /// Reload save slots from persistent storage. Called from production main()
    /// at startup so the title screen reflects what's on disk.
    pub fn refresh_save_slots(&mut self) {
        self.save_slots = self.save_backend.load_all();
    }

    // ─── Test-friendly accessors ────────────────────────
    //
    // Read-only views into private state. Tests use these to assert and to
    // implement story helpers (e.g. "press the key for the correct answer").

    /// True iff a dialogue box is currently active (typewriter running or waiting
    /// for the player to advance).
    pub fn is_dialogue_active(&self) -> bool {
        self.dialogue.active
    }

    /// True iff the player has finished any in-progress tile-to-tile slide.
    /// Movement on this game is grid-locked: each input direction starts a slide
    /// from one tile to the next, and inputs are ignored mid-slide.
    pub fn player_at_rest(&self) -> bool {
        !self.player.moving
    }

    /// Index (0-based) of the correct choice in the currently-active challenge,
    /// be it intake or normal. None if no challenge is on screen.
    pub fn correct_choice_index(&self) -> Option<usize> {
        let ch = self.active_challenge.as_ref()
            .map(|ac| &ac.challenge)
            .or_else(|| self.intake.as_ref().and_then(|iq| iq.challenge.as_ref().map(|ac| &ac.challenge)))?;
        ch.choices.iter().position(|c| c.correct)
    }

    /// Phase of the active challenge (intake or normal). None if no challenge.
    pub fn challenge_phase(&self) -> Option<Phase> {
        self.active_challenge.as_ref()
            .map(|ac| ac.state.phase)
            .or_else(|| self.intake.as_ref().and_then(|iq| iq.challenge.as_ref().map(|ac| ac.state.phase)))
    }

    /// Read-only view of the active KenKen session (None if no puzzle is on screen).
    /// Tests use this with `ui::kenken::layout` to compute click targets.
    pub fn active_kenken(&self) -> Option<&ActiveKenKen> {
        self.active_kenken.as_ref()
    }

    /// Snapshot of the event log length. Pair with `events_since(mark)` to
    /// read events emitted by a specific action — the basic assertion pattern
    /// for tests that care about *what just happened*, not just end-state.
    pub fn event_mark(&self) -> usize {
        self.events.len()
    }

    /// Events appended since the given mark. Slice is borrowed from the log,
    /// so callers can iterate or `matches!` against it without cloning.
    pub fn events_since(&self, mark: usize) -> &[GameEvent] {
        &self.events[mark..]
    }

    fn set_state(&mut self, new_state: GameState) {
        if self.state != new_state {
            self.events.push(GameEvent::StateChanged { from: self.state, to: new_state });
            self.state = new_state;
        }
    }

    fn start_dialogue(&mut self, lines: Vec<DialogueLine>) {
        if let Some(first) = lines.first() {
            self.events.push(GameEvent::DialogueStarted {
                speaker: first.speaker.clone(),
                text: first.text.clone(),
            });
        }
        self.dialogue.start(lines);
    }

    /// Run one frame of pure logic — no rendering, no macroquad calls. Tests
    /// can call this without a window. Production main calls step() then render().
    pub fn step(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        self.game_time += dt;

        let early_exit = if self.settings_open {
            false
        } else {
            self.dispatch_state(input, dt, screen)
        };
        if early_exit { return; }

        // P key: toggle debug overlay (any gameplay state)
        if !self.settings_open && input.pressed(KeyCode::P)
            && self.state != GameState::Title && self.state != GameState::NewGame
        {
            self.debug_overlay.toggle();
        }

        // ESC in dev map → title
        if !self.settings_open && self.map.id == "dev"
            && self.state == GameState::Playing && input.pressed(KeyCode::Escape)
        {
            self.set_state(GameState::Title);
            self.dialogue.active = false;
            self.active_challenge = None;
            self.active_kenken = None;
            self.pending_challenge = false;
        }
        self.dum_dum_hud.update(dt);

        // Time tracking + auto-save
        if !self.settings_open && self.state != GameState::Title && self.state != GameState::NewGame {
            self.play_time += dt;
            self.auto_save_timer += dt;
            if (self.auto_save_timer >= 30.0 || self.save_backend.is_page_hidden()) && self.map.id != "dev" {
                self.auto_save_timer = 0.0;
                let save_data = self.gather_save_data();
                self.save_backend.save_to(self.active_slot, &save_data);
            }
        }

        // Pixel-level interpolation only (no movement decisions). Tile-grid
        // decisions live in the resolver, dispatched from step_playing.
        // Capture which NPCs *just* finished their slide this frame so the
        // portal handler only fires once per arrival, not every frame they
        // sit on a portal tile waiting to wander again.
        let mut arrived_npcs: Vec<usize> = Vec::new();
        let arrived = if self.settings_open {
            false
        } else {
            let a = self.player.move_toward_target(dt);
            self.sparky.animate(dt);
            for (i, n) in self.npcs.iter_mut().enumerate() {
                if n.animate(dt) { arrived_npcs.push(i); }
            }
            self.dialogue.update(dt);
            a
        };

        // Portal check after arrival
        if arrived && self.state == GameState::Playing {
            let prev_map = self.map.id;
            self.handle_portal();
            // Only drop the NPC arrival list if the player actually swapped
            // maps — otherwise the indices are still valid, and a co-arriving
            // pushed NPC (player + pushee start their slides on the same
            // frame, so they finish on the same frame) would lose its own
            // portal trigger.
            if self.map.id != prev_map {
                arrived_npcs.clear();
            }
        }

        if self.state == GameState::Playing && !arrived_npcs.is_empty() {
            self.handle_npc_portals(&arrived_npcs);
        }

        self.camera.follow(self.player.x, self.player.y, &self.map, GAME_W, GAME_H);

        // Interaction menu input (layout from step-side; render() draws separately)
        if self.state == GameState::InteractionMenu {
            self.handle_interaction_menu(input, screen);
        }

        // Settings overlay input
        self.handle_settings_input(input, screen);

        // Debug-overlay export (uses last-frame's stashed button rect, or E key).
        if self.debug_overlay.is_export_clicked(input)
            || (self.debug_overlay.visible && input.pressed(KeyCode::E))
        {
            let json = session::build_export(
                &self.player_name, &self.session_log, &self.gifts_given,
                self.dum_dums, self.play_time, &self.profile, self.map.id,
            );
            let filename = format!("robot-buddy-session-{}.json", self.play_time as u64);
            session::download_json(&json, &filename);
        }
    }

    fn dispatch_state(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) -> bool {
        match self.state {
            GameState::Title => { self.step_title(input, screen); true }
            GameState::NewGame => { self.step_new_game(input, dt, screen); true }
            GameState::Intake => { self.step_intake(input, dt, screen); false }
            GameState::Playing => { self.step_playing(input, dt); false }
            GameState::InteractionMenu => false,
            GameState::Dialogue => { self.step_dialogue(input); false }
            GameState::Challenge => { self.step_challenge(input, dt, screen); false }
            GameState::KenKen => { self.step_kenken(input, dt, screen); false }
        }
    }

    // ─── State arms ─────────────────────────────────────

    fn step_title(&mut self, input: &FrameInput, screen: (f32, f32)) {
        let layout = ui::title_screen::layout_title(&self.save_slots, screen);
        let action = ui::title_screen::handle_title_input(&layout, input);
        if let Some(action) = action {
            match action {
                TitleAction::NewGame(slot) => {
                    self.new_game_form = Some(NewGameForm::new(slot));
                    self.set_state(GameState::NewGame);
                }
                TitleAction::LoadGame(slot) => {
                    if let Some(save_ref) = self.save_slots[slot].clone() {
                        self.load_from_save(&save_ref);
                        self.active_slot = slot;
                        self.auto_save_timer = 0.0;

                        if !self.profile.intake_completed {
                            self.intake = Some(IntakeState::new(self.profile.math_band));
                            self.start_dialogue(vec![DialogueLine {
                                speaker: "Sparky".into(),
                                text: "BEEP BOOP! Let's finish those warm-up puzzles real quick!".into(),
                            }]);
                            self.set_state(GameState::Intake);
                        } else {
                            self.start_dialogue(vec![DialogueLine {
                                speaker: "Sparky".into(),
                                text: format!("BEEP BOOP! Welcome back, {}! I missed you!", save_ref.name),
                            }]);
                            self.set_state(GameState::Dialogue);
                        }
                    }
                }
                TitleAction::DeleteSlot(slot) => {
                    self.save_backend.delete(slot);
                    self.save_slots = self.save_backend.load_all();
                }
            }
        }
    }

    fn step_new_game(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        // Take ownership of the form briefly so we can mutate self in branches.
        let mut form = match self.new_game_form.take() {
            Some(f) => f,
            None => return,
        };
        form.update(dt, input);
        let layout = ui::title_screen::layout_form(&form, screen);
        form.handle_form_clicks(&layout, input);
        let action = form.handle_action(&layout, input);
        // Put it back unless we're transitioning away
        let mut keep_form = true;

        if let Some(action) = action {
            match action {
                NewGameAction::Start => {
                    if is_dev_zone_code(&form.name) {
                        self.player_name = "Dev".into();
                        self.player_gender = form.gender;
                        self.profile = LearnerProfile::new();
                        self.profile.math_band = 5;
                        self.profile.intake_completed = true;
                        self.dum_dums = 20;
                        self.play_time = 0.0;
                        self.behavior_signals.clear();

                        self.map = Map::by_id("dev");
                        self.npcs = npc::npcs_for_map(self.map.id);
                        self.npcs_offstage.clear();
                        self.player = Entity::new(7, 10);
                        self.player.dir = Dir::Up;
                        self.sparky = Sparky::new(8, 10);
                        self.camera = GameCamera { x: 0.0, y: 0.0 };

                        self.start_dialogue(vec![DialogueLine {
                            speaker: "Sparky".into(),
                            text: "BEEP BOOP! Dev zone! Walk around, talk to everyone, open chests. ESC to exit!".into(),
                        }]);
                        keep_form = false;
                        self.set_state(GameState::Dialogue);
                    } else {
                        let slot = form.slot;
                        self.player_name = form.name.clone();
                        self.player_gender = form.gender;
                        self.profile = LearnerProfile::new();
                        self.profile.math_band = form.math_band;
                        self.dum_dums = 0;
                        self.play_time = 0.0;
                        self.active_slot = slot;
                        self.behavior_signals.clear();

                        self.map = Map::overworld();
                        self.player = Entity::new(14, 12);
                        self.sparky = Sparky::new(14, 13);
                        self.npcs = npc::npcs_for_map(self.map.id);
                        self.npcs_offstage.clear();
                        self.camera = GameCamera { x: 0.0, y: 0.0 };

                        let save_data = self.gather_save_data();
                        self.save_backend.save_to(slot, &save_data);
                        self.save_slots = self.save_backend.load_all();
                        self.auto_save_timer = 0.0;

                        self.intake = Some(IntakeState::new(form.math_band));
                        self.start_dialogue(vec![
                            DialogueLine { speaker: "Sparky".into(),
                                text: format!("BEEP BOOP! Hi {}! I'm Sparky, your robot buddy!", self.player_name) },
                            DialogueLine { speaker: "Sparky".into(),
                                text: "Before we go on our adventure, let me see what kind of math puzzles you like!".into() },
                            DialogueLine { speaker: "Sparky".into(),
                                text: "Don't worry, there's no wrong answers here! Just try your best! BEEP BOOP!".into() },
                        ]);
                        keep_form = false;
                        self.set_state(GameState::Intake);
                    }
                }
                NewGameAction::Back => {
                    keep_form = false;
                    self.set_state(GameState::Title);
                }
            }
        }

        if keep_form {
            self.new_game_form = Some(form);
        } else {
            self.new_game_form = None;
        }
    }

    fn step_intake(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        let mut iq = match self.intake.take() {
            Some(s) => s,
            None => return,
        };

        // Populate hit-test bounds from the pure layout fn so step doesn't depend on render.
        if let Some(ref mut ac) = iq.challenge {
            let (bounds, scaffold) = ui::challenge::layout(&ac.state, &ac.challenge, screen);
            ac.choice_bounds = bounds;
            ac.scaffold = scaffold;
        }

        match iq.phase {
            IntakePhase::Intro => {
                if input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) {
                    if self.dialogue.is_typewriting() {
                        iq.text_skipped_count += 1;
                    }
                    self.dialogue.advance();
                    if !self.dialogue.active {
                        let challenge = generate_intake_question(
                            iq.current_band, iq.question_index, &mut self.rng,
                        );
                        let ac = start_intake_challenge(challenge, iq.current_band, self.game_time);
                        self.events.push(GameEvent::ChallengeStarted {
                            question: ac.challenge.display_text.clone(),
                        });
                        audio::tts::speak("Sparky", &ac.challenge.speech_text);
                        iq.challenge = Some(ac);
                        iq.phase = IntakePhase::Question;
                    }
                }
            }
            IntakePhase::Question => {
                let mut dismiss = false;
                if let Some(ref mut ac) = iq.challenge {
                    if ac.state.phase == Phase::Complete && ac.state.correct == Some(true) {
                        ac.complete_timer += dt;
                        if ac.complete_timer >= 2.0 { dismiss = true; }
                    }

                    if let Some(action) = ui::challenge::handle_key(&ac.state, &ac.challenge, input) {
                        ac.state = challenge_reducer(ac.state.clone(), action);
                        speak_challenge_feedback(&ac.state);
                    } else if ac.state.phase == Phase::Complete
                        && (input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter))
                    {
                        dismiss = true;
                    }

                    if !dismiss && input.mouse_clicked {
                        let (mx, my) = input.mouse_pos;
                        if let Some(action) = ui::challenge::handle_click(
                            mx, my, &ac.state, &ac.challenge,
                            &ac.choice_bounds, &ac.scaffold,
                        ) {
                            ac.state = challenge_reducer(ac.state.clone(), action);
                            speak_challenge_feedback(&ac.state);
                        } else if ac.state.phase == Phase::Complete {
                            dismiss = true;
                        }
                    }
                }

                if dismiss {
                    if let Some(ref ac) = iq.challenge {
                        let was_correct = ac.state.correct == Some(true);
                        let response_ms = ((self.game_time - ac.start_time) as f64 * 1000.0).min(30000.0);

                        iq.answers.push(IntakeAnswer {
                            band: iq.current_band,
                            correct: was_correct,
                            response_time_ms: Some(response_ms),
                            skipped_text: false,
                        });

                        let ceiling = (iq.configured_band as u16 + 2).min(10) as u8;
                        iq.current_band = next_intake_band(iq.current_band, was_correct, ceiling);
                        iq.question_index += 1;
                    }
                    iq.challenge = None;

                    let save_data = self.gather_save_data();
                    self.save_backend.save_to(self.active_slot, &save_data);
                    self.auto_save_timer = 0.0;

                    if iq.question_index >= INTAKE_QUESTION_COUNT {
                        iq.phase = IntakePhase::Complete;
                    } else {
                        iq.phase = IntakePhase::Transition;
                    }
                }
            }
            IntakePhase::Transition => {
                let challenge = generate_intake_question(
                    iq.current_band, iq.question_index, &mut self.rng,
                );
                let ac = start_intake_challenge(challenge, iq.current_band, self.game_time);
                self.events.push(GameEvent::ChallengeStarted {
                    question: ac.challenge.display_text.clone(),
                });
                audio::tts::speak("Sparky", &ac.challenge.speech_text);
                iq.challenge = Some(ac);
                iq.phase = IntakePhase::Question;
            }
            IntakePhase::Complete => {
                let skipped = iq.text_skipped_count >= 2;
                for a in iq.answers.iter_mut() { a.skipped_text = skipped; }

                let result = process_intake_results(&iq.answers, Some(iq.configured_band));

                let band = result.math_band;
                self.profile = learner_reducer(self.profile.clone(), LearnerEvent::IntakeCompleted {
                    math_band: result.math_band,
                    pace: result.pace,
                    scaffolding: result.scaffolding,
                    promote_threshold: result.promote_threshold,
                    stretch_threshold: result.stretch_threshold,
                    text_speed: result.text_speed,
                });
                self.events.push(GameEvent::IntakeCompleted { math_band: band });

                self.start_dialogue(vec![
                    DialogueLine { speaker: "Sparky".into(),
                        text: "BEEP BOOP! All done! That was AWESOME!".into() },
                    DialogueLine { speaker: "Sparky".into(),
                        text: "I know just the right puzzles for you now! Let's go on our ADVENTURE!".into() },
                ]);

                self.intake = None;
                self.set_state(GameState::Dialogue);
                return; // intake consumed; don't put it back
            }
        }

        self.intake = Some(iq);
    }

    fn step_playing(&mut self, input: &FrameInput, dt: f32) {
        // ── Movement: collect intents, resolve, apply ───────────────────
        let player_intent = read_player_intent(input, &mut self.player);
        let sparky_intent = if self.sparky.entity.moving {
            MoveIntent::Stay
        } else {
            self.sparky.next_intent(self.player.tile_x, self.player.tile_y)
        };

        // Soft-block / push pressure: figure out which entity (if any) sits on
        // the tile the player is trying to walk into this frame, and accumulate
        // pressure on just that entity. Switching targets resets — pressure
        // belongs to the lean you're holding right now.
        let pressing_target: Option<EntityId> = match player_intent {
            MoveIntent::Move(d) => {
                let (dx, dy) = d.delta();
                let nx = self.player.tile_x as i32 + dx;
                let ny = self.player.tile_y as i32 + dy;
                if nx < 0 || ny < 0 {
                    None
                } else {
                    let (nx, ny) = (nx as usize, ny as usize);
                    if self.sparky.entity.tile_x == nx && self.sparky.entity.tile_y == ny {
                        Some(EntityId::Sparky)
                    } else {
                        self.npcs.iter().enumerate()
                            .find(|(_, n)| n.entity.tile_x == nx && n.entity.tile_y == ny)
                            .map(|(i, _)| EntityId::Npc(i as u32))
                    }
                }
            }
            MoveIntent::Stay => None,
        };
        match pressing_target {
            Some(id) => {
                let prev = self.pressure.get(&id).copied().unwrap_or(0.0);
                self.pressure.clear();
                self.pressure.insert(id, prev + dt);
            }
            None => self.pressure.clear(),
        }

        let states = self.snapshot_entities();
        let mut intents: Vec<(EntityId, MoveIntent)> = Vec::with_capacity(2 + self.npcs.len());
        intents.push((EntityId::Player, player_intent));
        intents.push((EntityId::Sparky, sparky_intent));
        // Snapshot the camera rect once so the wander gate doesn't re-borrow
        // self mid-iteration. Off-screen wanderers freeze: no cooldown tick,
        // no random direction roll. The kid you can't see isn't burning RNG.
        let cam = (self.camera.x, self.camera.y);
        for (i, n) in self.npcs.iter_mut().enumerate() {
            let intent = if npc_in_camera(cam, n) {
                n.next_intent(dt, &mut self.rng)
            } else {
                MoveIntent::Stay
            };
            intents.push((EntityId::Npc(i as u32), intent));
        }

        let map = &self.map;
        let resolutions = resolve_moves(
            &states,
            &intents,
            GridDims { width: map.width, height: map.height },
            |x, y| map.is_solid(x, y),
            &self.pressure,
        );

        for res in &resolutions {
            match res {
                MoveResolution::Granted { entity: EntityId::Player, to, .. } => {
                    self.sparky.record_player_pos(self.player.tile_x, self.player.tile_y);
                    self.player.start_move(to.0, to.1);
                    self.pressure.clear();
                }
                MoveResolution::Granted { entity: EntityId::Sparky, to, .. } => {
                    if !self.sparky.follow_queue.is_empty() {
                        self.sparky.follow_queue.remove(0);
                    }
                    self.sparky.entity.start_move(to.0, to.1);
                }
                MoveResolution::Granted { entity: EntityId::Npc(i), to, .. } => {
                    if let Some(n) = self.npcs.get_mut(*i as usize) {
                        n.entity.start_move(to.0, to.1);
                    }
                }
                _ => {}
            }
        }

        // Space: interact
        if input.pressed(KeyCode::Space) && !self.player.moving {
            let facing = facing_tile(self.player.tile_x, self.player.tile_y, self.player.dir);
            let facing_chest = facing.0 < self.map.width && facing.1 < self.map.height
                && self.map.tiles[facing.1][facing.0] == tilemap::Tile::Chest;

            if facing_chest {
                self.menu_target_id = "chest".into();
                self.menu_target_name = "Sparky".into();
                self.start_dialogue(vec![DialogueLine {
                    speaker: "Sparky".into(),
                    text: "OOOOH a treasure chest! But it has a LOCK! We need to solve the puzzle to open it!".into(),
                }]);
                self.pending_challenge = true;
                self.set_state(GameState::Dialogue);
            } else if let Some(target) = npc::get_interact_target(
                self.player.tile_x, self.player.tile_y, self.player.dir, &self.npcs
            ).map(|n| (n.kind, n.can_receive_gifts, n.never_challenge, n.is_puzzler, n)) {
                let (target_kind, can_receive_gifts, never_challenge, is_puzzler, target_ref) = target;
                let target_id = target_kind.as_str().to_string();
                let target_name = target_kind.display_name().to_string();

                // Dev knob bay NPCs short-circuit the normal interaction flow.
                // Each ctrl_* kind maps to one effect -- cycle a profile field,
                // reset a flag, or fire a fresh puzzle.
                if target_kind.is_dev_control() {
                    self.apply_dev_control(target_kind);
                    return;
                }

                let npc_info = NpcInfo {
                    id: target_id.clone(),
                    can_receive_gifts: Some(can_receive_gifts),
                    has_shop: None,
                    is_puzzler: Some(is_puzzler),
                };
                let player_st = PlayerState { dum_dums: self.dum_dums };
                let opts = interaction_options::get_interaction_options(&npc_info, &player_st);

                self.menu_target_id = target_id;
                self.menu_target_name = target_name;
                self.menu_can_challenge = !never_challenge;

                if opts.len() == 1 {
                    let lines = npc_dialogue_lines(target_ref, &mut self.rng);
                    if self.menu_can_challenge && self.rng.gen::<f32>() < 0.4 {
                        self.pending_challenge = true;
                    }
                    self.start_dialogue(lines);
                    self.set_state(GameState::Dialogue);
                } else {
                    self.menu_options = opts.iter().enumerate().map(|(i, o)| MenuOption {
                        option_type: o.option_type.clone(),
                        label: o.label.clone(),
                        key: i + 1,
                    }).collect();
                    self.set_state(GameState::InteractionMenu);
                }
            } else if npc::is_facing_sparky(
                self.player.tile_x, self.player.tile_y, self.player.dir,
                self.sparky.entity.tile_x, self.sparky.entity.tile_y,
            ) {
                let npc_info = NpcInfo {
                    id: "sparky".to_string(),
                    can_receive_gifts: Some(true),
                    has_shop: None,
                    is_puzzler: Some(false),
                };
                let player_st = PlayerState { dum_dums: self.dum_dums };
                let opts = interaction_options::get_interaction_options(&npc_info, &player_st);
                self.menu_target_id = "sparky".into();
                self.menu_target_name = "Sparky".into();
                self.menu_can_challenge = true;

                if opts.len() == 1 {
                    if self.rng.gen::<f32>() < 0.5 {
                        self.pending_challenge = true;
                    }
                    let lines = sparky_dialogue_lines(&mut self.rng);
                    self.start_dialogue(lines);
                    self.set_state(GameState::Dialogue);
                } else {
                    self.menu_options = opts.iter().enumerate().map(|(i, o)| MenuOption {
                        option_type: o.option_type.clone(),
                        label: o.label.clone(),
                        key: i + 1,
                    }).collect();
                    self.set_state(GameState::InteractionMenu);
                }
            }
        }
    }

    /// Build the per-frame snapshot the resolver consumes. Player and Sparky
    /// always present; NPCs follow in `Vec` order so `EntityId::Npc(i)`
    /// matches the index in `self.npcs`.
    fn snapshot_entities(&self) -> Vec<EntityState> {
        let mut v = Vec::with_capacity(2 + self.npcs.len());
        v.push(entity_state(EntityId::Player, &self.player, Solidity::Solid));
        v.push(entity_state(EntityId::Sparky, &self.sparky.entity, Solidity::SoftAfter(0.12)));
        for (i, n) in self.npcs.iter().enumerate() {
            // Wanderers are loose creatures who shuffle around — leaning into
            // them shoves them aside. Stationary "rooted" NPCs (Mommy, Sage,
            // shopkeeper, dev knobs) stay solid; pushing them around would feel
            // off-character.
            let solidity = if n.wanders {
                Solidity::PushableAfter(0.18)
            } else {
                Solidity::Solid
            };
            v.push(entity_state(EntityId::Npc(i as u32), &n.entity, solidity));
        }
        v
    }

    fn step_dialogue(&mut self, input: &FrameInput) {
        if input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) {
            if self.dialogue.is_typewriting() {
                self.behavior_signals.push(BehaviorSignal {
                    signal: "text_skipped".into(),
                    timestamp: Some(self.game_time as f64 * 1000.0),
                });
                self.profile = learner_reducer(self.profile.clone(), LearnerEvent::Behavior {
                    signal: "text_skipped".into(),
                });
            }
            self.dialogue.advance();
            self.events.push(GameEvent::DialogueAdvanced);
            if !self.dialogue.active {
                if self.pending_challenge {
                    self.pending_challenge = false;
                    let ac = start_challenge(&mut self.rng, &self.profile, self.game_time);
                    self.events.push(GameEvent::ChallengeStarted {
                        question: ac.challenge.display_text.clone(),
                    });
                    audio::tts::speak("Sparky", &ac.challenge.speech_text);
                    self.active_challenge = Some(ac);
                    self.set_state(GameState::Challenge);
                } else {
                    self.set_state(GameState::Playing);
                }
            }
        }
    }

    fn step_challenge(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        // Populate hit-test bounds from the pure layout fn.
        if let Some(ref mut ac) = self.active_challenge {
            let (bounds, scaffold) = ui::challenge::layout(&ac.state, &ac.challenge, screen);
            ac.choice_bounds = bounds;
            ac.scaffold = scaffold;
        }

        let mut dismiss = false;
        if let Some(ref mut ac) = self.active_challenge {
            if ac.state.phase == Phase::Complete && ac.state.correct == Some(true) {
                ac.complete_timer += dt;
                if ac.complete_timer >= 2.5 { dismiss = true; }
            }

            if let Some(action) = ui::challenge::handle_key(&ac.state, &ac.challenge, input) {
                ac.state = challenge_reducer(ac.state.clone(), action);
                speak_challenge_feedback(&ac.state);
            } else if ac.state.phase == Phase::Complete
                && (input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter))
            {
                dismiss = true;
            }

            if !dismiss && input.mouse_clicked {
                let (mx, my) = input.mouse_pos;
                if let Some(action) = ui::challenge::handle_click(
                    mx, my, &ac.state, &ac.challenge,
                    &ac.choice_bounds, &ac.scaffold,
                ) {
                    ac.state = challenge_reducer(ac.state.clone(), action);
                    speak_challenge_feedback(&ac.state);
                } else if ac.state.phase == Phase::Complete {
                    dismiss = true;
                }
            }
        }
        if dismiss {
            if let Some(ac) = self.active_challenge.take() {
                let was_correct = ac.state.correct == Some(true);
                let response_ms = ((self.game_time - ac.start_time) as f64 * 1000.0).min(30000.0);

                self.session_log.record_challenge(session::ChallengeRecord {
                    question: ac.challenge.display_text.clone(),
                    correct_answer: ac.challenge.correct_answer,
                    player_answer: None,
                    correct: was_correct,
                    operation: ac.challenge.numbers.op.clone(),
                    band: ac.challenge.band,
                    sampled_band: ac.challenge.sampled_band,
                    hint_used: ac.state.hint_used,
                    told_me: ac.state.told_me,
                    attempts: ac.state.attempts,
                    source: self.menu_target_id.clone(),
                    play_time_at_event: self.play_time,
                });

                let event = LearnerEvent::PuzzleAttempted {
                    correct: was_correct,
                    operation: ac.challenge.operation,
                    sub_skill: ac.challenge.sub_skill,
                    band: ac.challenge.sampled_band,
                    center_band: Some(ac.challenge.center_band),
                    response_time_ms: Some(response_ms),
                    hint_used: ac.state.hint_used,
                    told_me: ac.state.told_me,
                    cra_level_shown: Some(ac.state.render_hint.cra_stage),
                    timestamp: Some(self.game_time as f64 * 1000.0),
                };
                self.profile = learner_reducer(self.profile.clone(), event);

                if response_ms < 1000.0 && !was_correct {
                    let sig = BehaviorSignal {
                        signal: "rapid_clicking".into(),
                        timestamp: Some(self.game_time as f64 * 1000.0),
                    };
                    self.behavior_signals.push(sig);
                    self.profile = learner_reducer(self.profile.clone(), LearnerEvent::Behavior {
                        signal: "rapid_clicking".into(),
                    });
                }

                let frustration = detect_frustration(
                    &self.profile.rolling_window, &self.behavior_signals,
                );
                if frustration.level == FrustrationLevel::High {
                    self.profile = learner_reducer(self.profile.clone(), LearnerEvent::FrustrationDetected {
                        level: "high".into(),
                    });
                }

                if let Some(ref reward) = ac.state.reward {
                    self.dum_dums += reward.amount;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: reward.amount });
                }

                self.events.push(GameEvent::ChallengeResolved {
                    correct: was_correct, response_ms,
                });
            }
            self.set_state(GameState::Playing);

            if self.map.id != "dev" {
                let save_data = self.gather_save_data();
                self.save_backend.save_to(self.active_slot, &save_data);
                self.auto_save_timer = 0.0;
            }
        }
    }

    /// Dev knob effects. Exhaustive match on the dev-control NpcKind variants.
    /// Direct profile mutation here is intentional -- these are debugging
    /// tools, not gameplay events, and going through the learner reducer would
    /// mean inventing fake events for every knob. The `dev` map (and its child
    /// `control` map) is the only place dev-control NPCs exist, so this can't
    /// fire from a real game.
    fn apply_dev_control(&mut self, kind: npc::NpcKind) {
        use npc::NpcKind::*;
        let line = |text: &str| DialogueLine {
            speaker: "Knob".into(),
            text: text.into(),
        };
        match kind {
            CtrlBand => {
                self.profile.math_band = if self.profile.math_band >= 10 { 1 } else { self.profile.math_band + 1 };
                self.start_dialogue(vec![line(&format!("BEEP. Math band is now {}.", self.profile.math_band))]);
                self.set_state(GameState::Dialogue);
            }
            CtrlKenkenLevel => {
                self.profile.kenken_level = match self.profile.kenken_level {
                    2 => 3,
                    3 => 4,
                    _ => 2,
                };
                let n = self.profile.kenken_level;
                self.start_dialogue(vec![line(&format!("BEEP. KenKen grid is now {}x{}.", n, n))]);
                self.set_state(GameState::Dialogue);
            }
            CtrlCraReset => {
                for stage in self.profile.cra_stages.values_mut() {
                    *stage = CraStage::Concrete;
                }
                self.start_dialogue(vec![line("All operation CRA stages reset to Concrete.")]);
                self.set_state(GameState::Dialogue);
            }
            CtrlIntroReset => {
                self.profile.kenken_intro_seen = false;
                self.start_dialogue(vec![line("KenKen intro flag cleared. Next puzzle replays the tutorial.")]);
                self.set_state(GameState::Dialogue);
            }
            CtrlTriggerKenken => {
                let source = kind.as_str().to_string();
                let ak = start_kenken(&mut self.rng, &self.profile, self.game_time, source.clone());
                self.events.push(GameEvent::KenKenStarted {
                    grid_size: ak.session.puzzle.grid_size,
                    source,
                });
                self.active_kenken = Some(ak);
                self.set_state(GameState::KenKen);
            }
            CtrlTriggerChallenge => {
                let ac = start_challenge(&mut self.rng, &self.profile, self.game_time);
                self.events.push(GameEvent::ChallengeStarted {
                    question: ac.challenge.display_text.clone(),
                });
                audio::tts::speak("Sparky", &ac.challenge.speech_text);
                self.active_challenge = Some(ac);
                self.set_state(GameState::Challenge);
            }
            // Non-dev kinds shouldn't reach here -- caller gates on is_dev_control.
            other => {
                self.start_dialogue(vec![line(&format!("Unknown control: {}", other.as_str()))]);
                self.set_state(GameState::Dialogue);
            }
        }
    }

    fn step_kenken(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
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
                // Accept any input to dismiss — Space/Enter for keyboard, plus
                // mouse click anywhere on the panel. macroquad on native
                // occasionally drops single key-press events the same way it
                // drops mouse-press events, so we forgive both.
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
                let response_ms = ((self.game_time - ak.start_time) as f64 * 1000.0).min(120000.0);
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

                if was_correct {
                    // Same reward shape as a correct arithmetic challenge: 1 Dum Dum.
                    let award = 1u32;
                    self.dum_dums += award;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: award });
                }

                self.events.push(GameEvent::KenKenResolved {
                    correct: was_correct,
                    grid_size,
                    hints_used,
                    constraint_violations: violations,
                    response_ms,
                });
            }
            self.set_state(GameState::Playing);

            if self.map.id != "dev" {
                let save_data = self.gather_save_data();
                self.save_backend.save_to(self.active_slot, &save_data);
                self.auto_save_timer = 0.0;
            }
        }
    }

    fn handle_interaction_menu(&mut self, input: &FrameInput, screen: (f32, f32)) {
        let layout = ui::interaction_menu::layout(&self.menu_options, screen);
        let action = ui::interaction_menu::handle_input(&layout, input);
        let Some(action) = action else { return };
        match action {
            ui::interaction_menu::MenuAction::Select(opt_type) => match opt_type.as_str() {
                "talk" => {
                    if self.menu_target_id == "sparky" {
                        if self.menu_can_challenge && self.rng.gen::<f32>() < 0.5 {
                            self.pending_challenge = true;
                        }
                        let lines = sparky_dialogue_lines(&mut self.rng);
                        self.start_dialogue(lines);
                    } else {
                        // Pull lines first to free the borrow before start_dialogue.
                        let lines = self.npcs.iter().find(|n| n.id_str() == self.menu_target_id)
                            .map(|target| {
                                let lines = npc_dialogue_lines(target, &mut self.rng);
                                lines
                            });
                        if let Some(lines) = lines {
                            if self.menu_can_challenge && self.rng.gen::<f32>() < 0.4 {
                                self.pending_challenge = true;
                            }
                            self.start_dialogue(lines);
                        }
                    }
                    self.set_state(GameState::Dialogue);
                }
                "puzzle" => {
                    let source = self.menu_target_id.clone();
                    let ak = start_kenken(&mut self.rng, &self.profile, self.game_time, source);
                    self.events.push(GameEvent::KenKenStarted {
                        grid_size: ak.session.puzzle.grid_size,
                        source: ak.source_npc.clone(),
                    });
                    self.active_kenken = Some(ak);
                    self.set_state(GameState::KenKen);
                }
                "give" => {
                    if !give::can_give(self.dum_dums) {
                        self.set_state(GameState::Playing);
                    } else if let Some(result) = give::process_give(
                        self.dum_dums, &self.menu_target_id, &self.gifts_given,
                    ) {
                        self.session_log.record_give(session::GiveRecord {
                            recipient_id: self.menu_target_id.clone(),
                            recipient_name: self.menu_target_name.clone(),
                            dum_dums_before: self.dum_dums,
                            play_time_at_event: self.play_time,
                        });
                        self.dum_dums = result.new_dum_dums;
                        self.gifts_given = result.new_total_gifts;
                        self.dum_dum_hud.flash();

                        let total = *self.gifts_given.get(&self.menu_target_id).unwrap_or(&0);
                        self.events.push(GameEvent::GiftGiven {
                            recipient_id: self.menu_target_id.clone(),
                            total,
                        });

                        let save_data = self.gather_save_data();
                        self.save_backend.save_to(self.active_slot, &save_data);
                        self.auto_save_timer = 0.0;

                        let reaction = give_reaction_dialogue(
                            &self.menu_target_id, &self.menu_target_name,
                            &result.milestone, &mut self.rng,
                        );
                        self.start_dialogue(reaction);
                        self.set_state(GameState::Dialogue);
                    } else {
                        self.set_state(GameState::Playing);
                    }
                }
                _ => { self.set_state(GameState::Playing); }
            },
            ui::interaction_menu::MenuAction::Dismiss => {
                self.set_state(GameState::Playing);
            }
        }
    }

    fn handle_settings_input(&mut self, input: &FrameInput, screen: (f32, f32)) {
        if self.settings_open {
            if let Some(result) = ui::settings_overlay::handle_input(input, screen) {
                self.settings_open = false;
                match result {
                    ui::settings_overlay::SettingsResult::Close => {}
                    ui::settings_overlay::SettingsResult::BackToTitle => {
                        audio::tts::cancel();
                        self.dialogue.active = false;
                        self.active_challenge = None;
                        self.active_kenken = None;
                        self.pending_challenge = false;
                        self.set_state(GameState::Title);
                    }
                }
            }
        } else if self.state != GameState::Title && self.state != GameState::NewGame
            && input.pressed(KeyCode::T)
        {
            self.settings_open = true;
        }
    }

    /// Bounce any NPC currently on `(x, y)` of the current map to the nearest
    /// free tile. Used after the player teleports onto a tile so a wanderer
    /// that drifted onto the entry point doesn't end up standing on the
    /// player. No-op if the tile is already clear.
    fn displace_npcs_at(&mut self, x: usize, y: usize) {
        let map_w = self.map.width;
        let map_h = self.map.height;
        for i in 0..self.npcs.len() {
            if self.npcs[i].entity.tile_x != x || self.npcs[i].entity.tile_y != y {
                continue;
            }
            let map = &self.map;
            let player = (self.player.tile_x, self.player.tile_y);
            let sparky = (self.sparky.entity.tile_x, self.sparky.entity.tile_y);
            // Borrow-checker dance: snapshot the other NPCs' tiles so the
            // closure below doesn't reborrow self.npcs.
            let others: Vec<(usize, usize)> = self.npcs.iter().enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, n)| (n.entity.tile_x, n.entity.tile_y))
                .collect();
            let (nx, ny) = npc::find_npc_spawn_spot(
                x, y, map_w, map_h,
                |cx, cy| map.is_solid(cx, cy),
                |cx, cy| (cx, cy) == player || (cx, cy) == sparky
                    || others.iter().any(|t| *t == (cx, cy)),
            );
            // If find_npc_spawn_spot couldn't find a free tile (returned the
            // original) we leave them in place — better than overlapping
            // someone else.
            if (nx, ny) != (x, y) {
                let n = &mut self.npcs[i];
                n.entity.tile_x = nx;
                n.entity.tile_y = ny;
                n.entity.x = nx as f32 * TILE_SIZE;
                n.entity.y = ny as f32 * TILE_SIZE;
                n.entity.target_x = n.entity.x;
                n.entity.target_y = n.entity.y;
                n.entity.moving = false;
                n.home_tx = nx;
                n.home_ty = ny;
            }
        }
    }

    /// Walk the just-arrived NPCs and teleport any that landed on a portal
    /// tile to the portal's destination. Called from `step` after pixel
    /// animation finishes for the frame.
    ///
    /// Indices in `arrived` are valid for the *current* `self.npcs` ordering
    /// at the start of this method. Removing entries shifts indices, so we
    /// process them in descending order and re-check the index bound.
    fn handle_npc_portals(&mut self, arrived: &[usize]) {
        // Highest-index first so removals don't invalidate earlier indices.
        let mut sorted: Vec<usize> = arrived.iter().copied().collect();
        sorted.sort_unstable_by(|a, b| b.cmp(a));

        for i in sorted {
            if i >= self.npcs.len() { continue; }
            // Dev controls don't migrate — they're knobs bolted to the floor.
            if self.npcs[i].kind.is_dev_control() { continue; }
            let (tx, ty) = (self.npcs[i].entity.tile_x, self.npcs[i].entity.tile_y);
            let portal = match tilemap::check_portal(self.map.id, tx, ty) {
                Some(p) => p,
                None => continue,
            };
            self.transfer_npc_through_portal(i, portal);
        }
    }

    /// Move NPC at index `i` of `self.npcs` to the portal's destination map
    /// + tile. Resolves blocking by spiraling outward via
    /// `npc::find_npc_spawn_spot`. If the destination is the current map the
    /// NPC stays in `self.npcs`; otherwise it goes into
    /// `npcs_offstage[dest_map]` to be picked up next time the player visits.
    fn transfer_npc_through_portal(&mut self, i: usize, portal: &tilemap::Portal) {
        let dest_map = portal.to_map;
        let target_x = portal.to_x;
        let target_y = portal.to_y;

        // Resolve a non-blocking landing tile on the destination map.
        let (dest_x, dest_y) = if dest_map == self.map.id {
            // Same map (rare — most portals jump). Avoid landing on the
            // player, Sparky, or another NPC already here.
            let map = &self.map;
            let player = (self.player.tile_x, self.player.tile_y);
            let sparky = (self.sparky.entity.tile_x, self.sparky.entity.tile_y);
            let others: Vec<(usize, usize)> = self.npcs.iter().enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, n)| (n.entity.tile_x, n.entity.tile_y))
                .collect();
            npc::find_npc_spawn_spot(
                target_x, target_y, map.width, map.height,
                |cx, cy| map.is_solid(cx, cy),
                |cx, cy| (cx, cy) == player || (cx, cy) == sparky
                    || others.iter().any(|t| *t == (cx, cy)),
            )
        } else {
            // Different map: load it briefly to inspect terrain. The only
            // entities on a non-current map are whatever's stashed in
            // npcs_offstage[dest_map] (no player, no Sparky there).
            let dest_geometry = Map::by_id(dest_map);
            let empty_vec: Vec<npc::Npc> = Vec::new();
            let occupants = self.npcs_offstage.get(dest_map).unwrap_or(&empty_vec);
            let occupant_tiles: Vec<(usize, usize)> = occupants.iter()
                .map(|n| (n.entity.tile_x, n.entity.tile_y))
                .collect();
            npc::find_npc_spawn_spot(
                target_x, target_y, dest_geometry.width, dest_geometry.height,
                |cx, cy| dest_geometry.is_solid(cx, cy),
                |cx, cy| occupant_tiles.iter().any(|t| *t == (cx, cy)),
            )
        };

        let mut npc_obj = self.npcs.remove(i);
        npc_obj.entity.tile_x = dest_x;
        npc_obj.entity.tile_y = dest_y;
        npc_obj.entity.x = dest_x as f32 * TILE_SIZE;
        npc_obj.entity.y = dest_y as f32 * TILE_SIZE;
        npc_obj.entity.target_x = npc_obj.entity.x;
        npc_obj.entity.target_y = npc_obj.entity.y;
        npc_obj.entity.moving = false;
        // Re-anchor the wander tether to the new spot so the NPC stays around
        // the portal exit instead of trying to drift back toward its original
        // home tile (which is now on a different map entirely).
        npc_obj.home_tx = dest_x;
        npc_obj.home_ty = dest_y;

        if dest_map == self.map.id {
            self.npcs.push(npc_obj);
        } else {
            self.npcs_offstage
                .entry(dest_map.to_string())
                .or_insert_with(Vec::new)
                .push(npc_obj);
        }
    }

    fn handle_portal(&mut self) {
        let portal = match tilemap::check_portal(self.map.id, self.player.tile_x, self.player.tile_y) {
            Some(p) => p,
            None => return,
        };
        let secret = portal.secret;
        let mut dest_map = portal.to_map;
        let dest_x = portal.to_x;
        let dest_y = portal.to_y;
        let from_map = self.map.id.to_string();

        if dest_map == "dream" {
            self.dreaming = true;
        } else if portal.from_map == "dream" && dest_map == "overworld" {
            self.dreaming = false;
        } else if self.dreaming && dest_map == "overworld" {
            dest_map = "dream";
        }

        self.map = Map::by_id(dest_map);
        if self.dreaming && self.map.render_mode == tilemap::RenderMode::Normal {
            self.map.render_mode = tilemap::RenderMode::Dream;
        }
        // Stash the map we're leaving so wanderers there don't reset on
        // re-entry, then pop the destination's NPC roster (or fall back to the
        // map's default roster on first visit).
        let leaving = std::mem::take(&mut self.npcs);
        self.npcs_offstage.insert(from_map.clone(), leaving);
        self.npcs = self.npcs_offstage
            .remove(self.map.id)
            .unwrap_or_else(|| npc::npcs_for_map(self.map.id));

        self.player.tile_x = dest_x;
        self.player.tile_y = dest_y;
        self.player.x = dest_x as f32 * TILE_SIZE;
        self.player.y = dest_y as f32 * TILE_SIZE;
        self.player.target_x = self.player.x;
        self.player.target_y = self.player.y;
        self.player.moving = false;
        self.player.dir = portal.dir;

        // Make sure the player isn't crowded out by a wanderer that
        // accumulated on the entry tile while we were away. If anyone's there,
        // bounce them to the nearest free tile.
        self.displace_npcs_at(dest_x, dest_y);

        let sparky_pos = find_sparky_spot(dest_x, dest_y, &self.map, &self.npcs);
        self.sparky.entity.tile_x = sparky_pos.0;
        self.sparky.entity.tile_y = sparky_pos.1;
        self.sparky.entity.x = sparky_pos.0 as f32 * TILE_SIZE;
        self.sparky.entity.y = sparky_pos.1 as f32 * TILE_SIZE;
        self.sparky.entity.target_x = self.sparky.entity.x;
        self.sparky.entity.target_y = self.sparky.entity.y;
        self.sparky.entity.moving = false;
        self.sparky.follow_queue.clear();

        self.events.push(GameEvent::MapTransitioned {
            from: from_map,
            to: self.map.id.to_string(),
        });

        if secret {
            let lines = secret_entry_dialogue(self.map.id);
            if !lines.is_empty() {
                self.start_dialogue(lines);
                self.set_state(GameState::Dialogue);
            }
        }
    }

    // ─── Rendering ─────────────────────────────────────

    fn render_world(&mut self, screen: (f32, f32)) {
        let (sw, sh) = screen;
        if self.state == GameState::Intake {
            clear_background(Color::from_rgba(26, 26, 46, 255));
            set_default_camera();

            let sparky_x = sw / 2.0 - TILE_SIZE / 2.0;
            let sparky_y = 60.0;
            sprites::robot::draw_robot(sparky_x, sparky_y, Dir::Down, 0, self.game_time);

            if let Some(ref iq) = self.intake {
                if iq.phase == IntakePhase::Question || iq.phase == IntakePhase::Transition {
                    let progress_text = format!("Question {} of {}", iq.question_index + 1, INTAKE_QUESTION_COUNT);
                    let tw = measure_text(&progress_text, None, 26, 1.0).width;
                    draw_text(&progress_text, sw / 2.0 - tw / 2.0, 134.0,
                        26.0, Color::from_rgba(144, 202, 249, 200));
                }
            }

            if let Some(ref iq) = self.intake {
                if let Some(ref ac) = iq.challenge {
                    ui::challenge::draw_challenge(&ac.state, &ac.challenge, self.game_time);
                }
            }
        } else {
            set_camera(&Camera2D {
                zoom: vec2(2.0 / sw, 2.0 / sh),
                target: vec2(self.camera.x + GAME_W / 2.0, self.camera.y + GAME_H / 2.0),
                ..Default::default()
            });

            clear_background(Color::from_rgba(26, 26, 46, 255));
            tilemap::draw_map(&self.map, self.camera.x, self.camera.y, GAME_W, GAME_H, self.game_time);

            enum SpriteKind<'a> { Player, Sparky, Npc(&'a npc::Npc) }
            struct Renderable<'a> { y: f32, kind: SpriteKind<'a> }
            let mut renderables: Vec<Renderable> = vec![];

            renderables.push(Renderable { y: self.player.y, kind: SpriteKind::Player });
            renderables.push(Renderable { y: self.sparky.entity.y, kind: SpriteKind::Sparky });
            for n in &self.npcs {
                renderables.push(Renderable { y: n.entity.y, kind: SpriteKind::Npc(n) });
            }
            renderables.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());

            for r in &renderables {
                match &r.kind {
                    SpriteKind::Player => match self.player_gender {
                        Gender::Boy => sprites::player::draw_player_boy(self.player.x, self.player.y, self.player.dir, self.player.frame, self.game_time),
                        Gender::Girl => sprites::player::draw_player_girl(self.player.x, self.player.y, self.player.dir, self.player.frame, self.game_time),
                    },
                    SpriteKind::Sparky => sprites::robot::draw_robot(self.sparky.entity.x, self.sparky.entity.y, self.sparky.entity.dir, self.sparky.entity.frame, self.game_time),
                    SpriteKind::Npc(n) => n.draw(self.game_time),
                }
            }

            set_default_camera();
        }
    }

    fn render_hud(&mut self, screen: (f32, f32)) {
        ui::hud::draw_area_name(self.map.id, self.player.tile_x, self.player.tile_y);
        self.dum_dum_hud.draw(self.dum_dums, screen);
        self.debug_overlay.draw(
            self.map.id, self.player.tile_x, self.player.tile_y,
            self.dum_dums, self.play_time,
            &self.profile, self.session_log.challenge_count(), self.session_log.correct_count(),
            screen,
        );
    }

    /// Draw everything for the current frame. Only called in production — tests
    /// skip this so they don't need a macroquad context.
    pub fn render(&mut self, screen: (f32, f32), input: &FrameInput) {
        match self.state {
            GameState::Title => {
                let layout = ui::title_screen::layout_title(&self.save_slots, screen);
                ui::title_screen::draw_title(&layout, &self.save_slots, self.game_time, input.mouse_pos);
                return;
            }
            GameState::NewGame => {
                if let Some(ref form) = self.new_game_form {
                    let layout = ui::title_screen::layout_form(form, screen);
                    form.draw(&layout, input.mouse_pos);
                }
                return;
            }
            _ => {}
        }

        // World + HUD for all gameplay states (Intake handled inside render_world).
        self.render_world(screen);
        self.render_hud(screen);

        if self.state == GameState::InteractionMenu {
            let layout = ui::interaction_menu::layout(&self.menu_options, screen);
            ui::interaction_menu::draw(&layout, input.mouse_pos);
        }

        self.dialogue.draw();

        // Challenge overlay (separate from intake's in-render_world drawing).
        if let Some(ref ac) = self.active_challenge {
            ui::challenge::draw_challenge(&ac.state, &ac.challenge, self.game_time);
        }

        // KenKen overlay
        if let Some(ref ak) = self.active_kenken {
            let layout = ui::kenken::layout(&ak.session, screen);
            ui::kenken::draw_kenken(&ak.session, &layout, self.game_time, ak.selected, ak.intro_step);
        }

        if self.settings_open {
            ui::settings_overlay::draw(screen);
        }
    }

    // ─── Save helpers ──────────────────────────────────

    fn gather_save_data(&self) -> SaveData {
        SaveData {
            version: 2,
            name: self.player_name.clone(),
            gender: self.player_gender,
            map_id: self.map.id.to_string(),
            player_x: self.player.tile_x,
            player_y: self.player.tile_y,
            player_dir: self.player.dir,
            sparky_x: self.sparky.entity.tile_x,
            sparky_y: self.sparky.entity.tile_y,
            math_band: None,
            dum_dums: self.dum_dums,
            play_time: self.play_time,
            timestamp: 0,
            gifts_given: self.gifts_given.clone(),
            profile: self.profile.clone(),
        }
    }

    fn load_from_save(&mut self, save_data: &SaveData) {
        self.player_name = save_data.name.clone();
        self.player_gender = save_data.gender;
        self.profile = save_data.profile.clone();
        self.dum_dums = save_data.dum_dums;
        self.play_time = save_data.play_time;
        self.gifts_given = save_data.gifts_given.clone();

        self.map = Map::by_id(&save_data.map_id);
        self.npcs = npc::npcs_for_map(self.map.id);
        self.npcs_offstage.clear();

        self.player.tile_x = save_data.player_x;
        self.player.tile_y = save_data.player_y;
        self.player.x = save_data.player_x as f32 * TILE_SIZE;
        self.player.y = save_data.player_y as f32 * TILE_SIZE;
        self.player.target_x = self.player.x;
        self.player.target_y = self.player.y;
        self.player.moving = false;
        self.player.dir = save_data.player_dir;

        self.sparky.entity.tile_x = save_data.sparky_x;
        self.sparky.entity.tile_y = save_data.sparky_y;
        self.sparky.entity.x = save_data.sparky_x as f32 * TILE_SIZE;
        self.sparky.entity.y = save_data.sparky_y as f32 * TILE_SIZE;
        self.sparky.entity.target_x = self.sparky.entity.x;
        self.sparky.entity.target_y = self.sparky.entity.y;
        self.sparky.entity.moving = false;
        self.sparky.follow_queue.clear();
    }
}

// ─── Free helpers ──────────────────────────────────────

fn is_dev_zone_code(name: &str) -> bool {
    let normalized: String = name.chars().filter(|c| !c.is_whitespace()).collect();
    normalized.eq_ignore_ascii_case("justinbailey")
}

fn make_challenge_profile(profile: &LearnerProfile) -> ChallengeProfile {
    ChallengeProfile {
        math_band: profile.math_band.max(1).min(10),
        spread_width: profile.spread_width,
        operation_stats: profile.operation_stats.clone(),
    }
}

fn start_challenge(rng: &mut SmallRng, profile: &LearnerProfile, game_time: f32) -> ActiveChallenge {
    let cp = make_challenge_profile(profile);
    let challenge = generate_challenge(&cp, rng);

    let cra = profile.cra_stages
        .get(&challenge.operation)
        .copied()
        .unwrap_or(CraStage::Concrete);

    let cs = ChallengeState {
        phase: Phase::Presented,
        correct_answer: challenge.correct_answer,
        attempts: 0,
        max_attempts: profile.wrongs_before_teach.max(1) as u32,
        correct: None,
        question: DisplaySpeech {
            display: challenge.display_text.clone(),
            speech: challenge.speech_text.clone(),
        },
        feedback: None,
        reward: None,
        render_hint: RenderHint {
            cra_stage: cra,
            answer_mode: "choice".into(),
            interaction_type: "quiz".into(),
        },
        hint_used: false,
        hint_level: 0,
        told_me: false,
        voice: VoiceState::reset(),
    };

    ActiveChallenge {
        state: cs,
        challenge,
        choice_bounds: vec![],
        scaffold: ScaffoldBounds { show_me: None, tell_me: None },
        complete_timer: 0.0,
        start_time: game_time,
    }
}

fn start_kenken(rng: &mut SmallRng, profile: &LearnerProfile, game_time: f32, source: String) -> ActiveKenKen {
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

fn start_intake_challenge(challenge: Challenge, _band: u8, game_time: f32) -> ActiveChallenge {
    let cs = ChallengeState {
        phase: Phase::Presented,
        correct_answer: challenge.correct_answer,
        attempts: 0,
        max_attempts: 2,
        correct: None,
        question: DisplaySpeech {
            display: challenge.display_text.clone(),
            speech: challenge.speech_text.clone(),
        },
        feedback: None,
        reward: None,
        render_hint: RenderHint {
            cra_stage: CraStage::Abstract,
            answer_mode: "choice".into(),
            interaction_type: "quiz".into(),
        },
        hint_used: false,
        hint_level: 0,
        told_me: false,
        voice: VoiceState::reset(),
    };

    ActiveChallenge {
        state: cs,
        challenge,
        choice_bounds: vec![],
        scaffold: ScaffoldBounds { show_me: None, tell_me: None },
        complete_timer: 0.0,
        start_time: game_time,
    }
}

fn sparky_dialogue_lines(rng: &mut SmallRng) -> Vec<DialogueLine> {
    let lines = [
        "BEEP BOOP! Hi boss! I polished my antenna just for you!",
        "BZZZT! I think a butterfly landed on my head! Is it still there?",
        "Did you know robots dream about lollipops? I do! Every night!",
        "Whoa! My circuits are tingling! That means adventure is near!",
        "I tried to count all the flowers but I ran out of beeps!",
        "Hey boss! Watch this! *spins around* WHOAAAA I'm dizzy!",
        "Beep bop boop! That's robot for 'you're awesome!'",
        "ALERT ALERT! Fun detected in this area! Beep boop!",
    ];
    let idx = rng.gen_range(0..lines.len());
    vec![DialogueLine { speaker: "Sparky".into(), text: lines[idx].into() }]
}

/// Build an `EntityState` for the resolver. Inverts Entity's "tile_x = dest
/// while moving" convention: the resolver wants `tile_x/tile_y` to be the
/// SOURCE tile (the one the entity is visibly leaving) and `moving_to` to
/// hold the destination, so both are reserved against other intents.
fn entity_state(id: EntityId, e: &Entity, solidity: Solidity) -> EntityState {
    if !e.moving {
        return EntityState { id, tile_x: e.tile_x, tile_y: e.tile_y, moving_to: None, solidity };
    }
    // Pixel `(target_x - x)/TILE_SIZE` rounds to the signed tile-delta
    // remaining; subtracting from the (post-start_move) tile coords recovers
    // the source.
    let dx_rem = ((e.target_x - e.x) / TILE_SIZE).round() as i32;
    let dy_rem = ((e.target_y - e.y) / TILE_SIZE).round() as i32;
    let src_x = (e.tile_x as i32 - dx_rem).max(0) as usize;
    let src_y = (e.tile_y as i32 - dy_rem).max(0) as usize;
    EntityState { id, tile_x: src_x, tile_y: src_y, moving_to: Some((e.tile_x, e.tile_y)), solidity }
}

/// Translate held arrow/WASD keys into a `MoveIntent` and update `player.dir`
/// to match. Setting `dir` even when the move ends up blocked is intentional:
/// pressing into a wall should still turn the player so they're "facing" what
/// they want to interact with.
///
/// Returns `Stay` if the player is already mid-step (no new intent until they
/// settle on a tile) or no movement key is held.
fn read_player_intent(input: &FrameInput, player: &mut Entity) -> MoveIntent {
    if player.moving { return MoveIntent::Stay; }
    let dir = if input.down(KeyCode::Up) || input.down(KeyCode::W) {
        Some((Direction::Up, Dir::Up))
    } else if input.down(KeyCode::Down) || input.down(KeyCode::S) {
        Some((Direction::Down, Dir::Down))
    } else if input.down(KeyCode::Left) || input.down(KeyCode::A) {
        Some((Direction::Left, Dir::Left))
    } else if input.down(KeyCode::Right) || input.down(KeyCode::D) {
        Some((Direction::Right, Dir::Right))
    } else {
        None
    };
    match dir {
        Some((d, sprite_dir)) => { player.dir = sprite_dir; MoveIntent::Move(d) }
        None => MoveIntent::Stay,
    }
}

fn npc_dialogue_lines(npc: &npc::Npc, rng: &mut SmallRng) -> Vec<DialogueLine> {
    use npc::NpcKind::*;
    let lines: &[&str] = match npc.kind {
        Mommy => &[
            "Hi sweetie! I'm so proud of you for exploring!",
            "You and Sparky make the best team!",
            "I love you! Keep being amazing!",
        ],
        Sage | SageLab => &[
            "Ahhhh, young adventurer! The stars told me you'd come!",
            "Welcome! I am Professor Gizmo, master of numbers!",
            "The ancient scrolls speak of a hero... and I think it's YOU!",
        ],
        Kid1 => &[
            "Wanna see me do a cartwheel? Watch! ...okay I can't actually do one yet.",
            "Sparky is SO COOL! I wish I had a robot friend!",
            "Did you know frogs can jump SUPER far? Like, really far!",
        ],
        Kid2 => &[
            "Hi... um... do you like bugs? I found a really cool one.",
            "Sparky beeped at me and I think that means he likes me!",
            "Do you think clouds are soft? I think they're soft.",
        ],
        Shopkeeper => &[
            "Welcome to my shop! Everything costs Dum Dums!",
            "I've got the finest wares in all of Robot Village!",
        ],
        DreamSage => &[
            "You are dreaming... or are you? The numbers whisper here...",
            "In dreams, 2 + 2 can be anything... but it's still 4.",
        ],
        GlitchDog => &[
            "BORK BORK! sys.treat.exe... GOOD BOY overflow!",
            "Woof! *static* I am... a good boy? BORK.dll loaded!",
            "fetch(ball) returned: UNDEFINED... but I still love you!",
        ],
        GroveSpirit => &[
            "How... did you find this place? The trees have hidden it for ages...",
            "It's dangerous to go alone... take this!",
            "The leaves whisper your name... they say you are very clever.",
        ],
        // Dev-control NPCs go through apply_dev_control, never this path.
        CtrlBand | CtrlKenkenLevel | CtrlCraReset | CtrlIntroReset
        | CtrlTriggerKenken | CtrlTriggerChallenge => &["Hello there!"],
    };
    let idx = rng.gen_range(0..lines.len());
    vec![DialogueLine { speaker: npc.name().into(), text: lines[idx].into() }]
}

/// True iff any pixel of the NPC's tile rect overlaps the camera viewport.
/// Used to gate wander cooldown ticks — off-screen wanderers freeze in place
/// so unseen rooms don't burn RNG and don't have characters drifting around
/// out of sight.
fn npc_in_camera(cam: (f32, f32), n: &npc::Npc) -> bool {
    let x = n.entity.x;
    let y = n.entity.y;
    x + TILE_SIZE > cam.0
        && x < cam.0 + GAME_W
        && y + TILE_SIZE > cam.1
        && y < cam.1 + GAME_H
}

fn find_sparky_spot(player_x: usize, player_y: usize, map: &Map, npcs: &[npc::Npc]) -> (usize, usize) {
    let candidates = [
        (player_x, player_y + 1),
        (player_x, player_y.wrapping_sub(1)),
        (player_x + 1, player_y),
        (player_x.wrapping_sub(1), player_y),
    ];
    for (cx, cy) in candidates {
        if cx < map.width && cy < map.height
            && !map.is_solid(cx, cy)
            && !npcs.iter().any(|n| n.entity.tile_x == cx && n.entity.tile_y == cy)
        {
            return (cx, cy);
        }
    }
    (player_x, player_y)
}

fn facing_tile(tx: usize, ty: usize, dir: Dir) -> (usize, usize) {
    match dir {
        Dir::Up => (tx, ty.wrapping_sub(1)),
        Dir::Down => (tx, ty + 1),
        Dir::Left => (tx.wrapping_sub(1), ty),
        Dir::Right => (tx + 1, ty),
    }
}

fn give_reaction_dialogue(
    target_id: &str, target_name: &str,
    milestone: &Option<robot_buddy_domain::economy::give::Milestone>,
    rng: &mut SmallRng,
) -> Vec<DialogueLine> {
    let text = if let Some(ms) = milestone {
        match (target_id, ms.reaction.as_str()) {
            ("sparky", "first") => "My FIRST Dum Dum?! This is the BEST DAY of my robot LIFE!".into(),
            ("sparky", "spin") => "FIVE DUM DUMS! Watch me spin! *spins* WHOAAAA!".into(),
            ("sparky", "accessory") => "TEN?! I'm wearing a bow tie now! Do I look fancy?!".into(),
            ("sparky", "color_change") => "TWENTY! My chest light is changing color! BZZZT!".into(),
            ("sparky", "ultimate") => "FIFTY DUM DUMS. Boss. I... I don't have words. BEEP.".into(),
            (_, "first") => "My first Dum Dum! Thank you so much, you're the best!".into(),
            _ => format!("WOW! You've given me {} Dum Dums! You're amazing!", ms.total),
        }
    } else {
        match target_id {
            "sparky" => {
                let lines = ["MMMMM! *crunch* Circuits... BUZZING!", "Dum Dum Dum Dum! That's my favorite song!", "BZZZT! Sugar rush! BEEP BOOP BEEP!"];
                lines[rng.gen_range(0..lines.len())].into()
            }
            _ => "Thank you! You're so kind!".into(),
        }
    };
    vec![DialogueLine { speaker: target_name.into(), text }]
}

fn speak_challenge_feedback(cs: &ChallengeState) {
    if let Some(ref fb) = cs.feedback {
        audio::tts::speak("Sparky", &fb.speech);
    }
}

fn secret_entry_dialogue(map_id: &str) -> Vec<DialogueLine> {
    match map_id {
        "dream" => vec![
            DialogueLine { speaker: "Sparky".into(),
                text: "BZZZT! Boss! My circuits feel all tingly! Everything looks... purple?".into() },
            DialogueLine { speaker: "Sparky".into(),
                text: "Are we... dreaming? The flowers are floating! BEEP BOOP this is WEIRD!".into() },
        ],
        "doghouse" => vec![
            DialogueLine { speaker: "Sparky".into(),
                text: "ERROR ERROR! Visual systems reporting... BORK?! What IS this place?!".into() },
            DialogueLine { speaker: "Sparky".into(),
                text: "My display is all glitchy! I see scan lines and... is that a DOG made of CODE?!".into() },
        ],
        "grove" => vec![
            DialogueLine { speaker: "Sparky".into(),
                text: "Whoa boss! We just walked RIGHT THROUGH those trees! How did we do that?!".into() },
            DialogueLine { speaker: "Sparky".into(),
                text: "This place is SO pretty! And SO secret! The trees are whispering!".into() },
        ],
        _ => vec![],
    }
}
