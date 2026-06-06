//! The Game struct: all state, all logic, all rendering.
//!
//! Production: `main()` captures input from macroquad, calls `step()`, awaits next_frame.
//! Tests: build a `FrameInput` synthetically and call `step()` directly. (Tests still need
//! a macroquad window today because draw calls run unconditionally — Phase 4 will split.)

use macroquad::prelude::*;
use ::rand::{Rng, SeedableRng};
use ::rand::rngs::SmallRng;
use ::rand::seq::SliceRandom;
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
use robot_buddy_domain::logic::patterns::{
    self, PatternPhase, PatternSession, generate_for_level,
};
use robot_buddy_domain::logic::balance::{
    self, BalancePhase, BalanceSession, generate_for_band as generate_balance_for_band,
};
use robot_buddy_domain::logic::sudoku::{
    self, SudokuPhase, SudokuSession, generate_for_level as generate_sudoku_for_level,
};
use robot_buddy_domain::economy::shop::{self, ShopItem};
use robot_buddy_domain::world::encounters::{self, EncounterConfig, EncounterKind};
use robot_buddy_domain::logic::manipulate_concrete::{self, ConcreteKind, generate_concrete};
use robot_buddy_domain::logic::number_line::{self, generate_number_line};
use robot_buddy_domain::quest::{self, Quest, QuestAction, QuestSession, QuestStatus, QuestStep};
use robot_buddy_domain::types::{Phase, CraStage, FrustrationLevel, Operation};
use robot_buddy_domain::world::movement::{
    Direction, EntityId, EntityState, GridDims, MoveIntent, MoveResolution,
    Solidity, resolve_moves,
};

use crate::tilemap::{self, Map, TILE_SIZE};
use crate::sprites::{self, Dir};
use crate::follower::Follower;
use crate::npc::{self, NpcKind};
use crate::ui;
use crate::ui::dialogue::{DialogueBox, DialogueLine};
use crate::ui::challenge::{ChoiceBound, ScaffoldBounds};
use crate::ui::title_screen::{TitleAction, NewGameAction, NewGameForm};
use crate::ui::hud::{DumDumHud, DebugOverlay};
use crate::ui::interaction_menu::MenuOption;
use crate::save::{self, CompanionSave, SaveBackend, SaveData, SaveSlots, Gender};
use crate::audio;
use crate::session;
use crate::input::FrameInput;

pub const GAME_W: f32 = 960.0;
pub const GAME_H: f32 = 720.0;
const MOVE_SPEED: f32 = 200.0;
const INTAKE_QUESTION_COUNT: usize = 5;

/// Where Sparky waits when an NPC has taken over the buddy slot — next to
/// Professor Gizmo on the overworld so the kid always knows where to find him.
pub const SPARKY_HOME_MAP: &str = "overworld";
pub const SPARKY_HOME_TX: usize = 13;
pub const SPARKY_HOME_TY: usize = 12;

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
    Pattern,
    Balance,
    Sudoku,
    Shop,
    Manipulative,
    Quest,
}

/// Opt-in toggles for in-development paths that aren't ready for default play.
/// All default OFF so the test suite and normal play are unaffected; flip them
/// on in the dev control room (or in production wiring) to playtest. A drop of
/// tech debt traded for being able to try these before they're fully baked.
#[derive(Clone, Copy, Debug, Default)]
pub struct FeatureFlags {
    /// Random encounters fire as the kid explores.
    pub encounters: bool,
    /// Add/sub challenges route to a hands-on CRA manipulative instead of the
    /// multiple-choice quiz when the learner's CRA stage for the op warrants it.
    pub cra_manipulatives: bool,
    /// Quests are offered and executable.
    pub quest: bool,
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

pub struct ActivePattern {
    pub session: PatternSession,
    pub complete_timer: f32,
    pub start_time: f32,
    pub source_npc: String,
}

pub struct ActiveBalance {
    pub session: BalanceSession,
    pub complete_timer: f32,
    pub start_time: f32,
    pub source_npc: String,
}

pub struct ActiveSudoku {
    pub session: SudokuSession,
    pub selected: Option<(u8, u8)>,
    pub complete_timer: f32,
    pub start_time: f32,
    pub source_npc: String,
}

/// Inline multiple-choice for a quest's MathPuzzle step (kept self-contained so
/// quests never hand control to the challenge state and back).
#[derive(Clone)]
pub struct QuestPuzzle {
    pub choices: Vec<i32>,
    pub answer: i32,
}

pub struct ActiveQuest {
    pub session: QuestSession,
    /// Present while the current step is a MathPuzzle.
    pub puzzle: Option<QuestPuzzle>,
    pub message: Option<String>,
}

pub struct ActiveManipulative {
    pub manip: ui::manipulative::Manip,
    /// The challenge this manipulative stands in for — drives the learner event
    /// + prompt so the adaptive system gets the same signal as the quiz path.
    pub challenge: Challenge,
    pub complete_timer: f32,
    pub start_time: f32,
}

pub struct ActiveShop {
    pub catalog: Vec<ShopItem>,
    pub owned: std::collections::HashSet<String>,
    /// `Some(index)` while solving the purchase subtraction for that catalog
    /// item; `None` while browsing.
    pub selected: Option<usize>,
    pub choices: Vec<u32>,
    pub answer: u32,
    pub cost: u32,
    pub balance_before: u32,
    pub message: Option<String>,
    pub source_npc: String,
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
    /// Emitted when the player's follower NPC changes. `joined` is the NPC who
    /// just became the companion (`None` if the slot was cleared); `left` is
    /// the previous companion who returned home (`None` on first companion).
    CompanionChanged { joined: Option<String>, left: Option<String> },
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
    PatternStarted { level: u8, source: String },
    PatternResolved {
        correct: bool,
        level: u8,
        attempts: u8,
        response_ms: f64,
    },
    BalanceStarted { level: u8, source: String },
    BalanceResolved {
        correct: bool,
        level: u8,
        attempts: u8,
        response_ms: f64,
    },
    /// A shop purchase succeeded: the kid solved the cost subtraction.
    DumDumsSpent { amount: u32, item: String },
    /// A random encounter fired ("flavor" | "dum_dum" | "challenge" | "sighting").
    EncounterTriggered { kind: String },
    /// A quest run reached its final step.
    QuestCompleted,
    SudokuStarted { grid_size: u8, source: String },
    SudokuResolved {
        correct: bool,
        grid_size: u8,
        constraint_violations: u8,
        response_ms: f64,
    },
}

// ─── The Game ───────────────────────────────────────────

pub struct Game {
    // World
    pub map: Map,
    pub player: Entity,
    pub sparky: Follower,
    pub camera: GameCamera,    pub npcs: Vec<npc::Npc>,
    /// NPCs that belong to maps the player isn't on right now. Wandering NPCs
    /// who stepped through a portal accumulate here under the destination map
    /// id; on map change we swap the current `npcs` vec with whatever's stashed
    /// for the new map (or fall back to `npcs_for_map`'s default roster on
    /// first visit). Reset on new game / load — saves only persist the current
    /// map's NPC layout, off-map wanderers snap back to defaults.
    pub npcs_offstage: HashMap<String, Vec<npc::Npc>>,
    /// The NPC currently following the player. Detached from any map roster
    /// while in this slot; travels across maps with the player. Player rotates
    /// buddies by gifting a dum dum: a non-buddy gets recruited, the previous
    /// buddy returns to their static place. Invariant: `companion.is_some()`
    /// iff `sparky_parked` — at most one of Sparky/companion follows at a time.
    pub companion: Option<npc::Npc>,
    /// True when an NPC has taken Sparky's place. Parked Sparky sits at
    /// `(SPARKY_HOME_TX, SPARKY_HOME_TY)` on `SPARKY_HOME_MAP`; he's only
    /// rendered, soft-blocked, and interactable when the player is on his
    /// parked map. When the player gifts him a dum dum, he rejoins.
    pub sparky_parked: bool,
    /// Wander cooldown for parked Sparky. Ticks down only while parked AND
    /// the player is on his home map; reset to a small initial delay on
    /// park so he doesn't twitch on the same frame he's swapped out.
    sparky_wander_cooldown: f32,
    pub dreaming: bool,
    /// Remaining tiles of a click-to-walk path the player auto-follows. Empty
    /// when walking by keyboard. Cleared by keyboard input, interaction, and
    /// map changes.
    player_path: Vec<(usize, usize)>,
    /// When a tap targeted an interactable (NPC / Sparky / chest), the tile to
    /// auto-interact with on arrival. Taken (one-shot) when the player lands
    /// adjacent.
    pending_interact: Option<(usize, usize)>,
    /// Set for one frame when an arrival should fire the interaction, as if the
    /// kid pressed Space facing the target.
    auto_interact: bool,
    /// The tapped destination tile, shown as a walk marker until arrival.
    click_target: Option<(usize, usize)>,

    // Time
    pub game_time: f32,
    pub play_time: f32,

    // State machine
    pub state: GameState,
    intake: Option<IntakeState>,
    active_challenge: Option<ActiveChallenge>,
    active_kenken: Option<ActiveKenKen>,
    active_pattern: Option<ActivePattern>,
    active_balance: Option<ActiveBalance>,
    active_sudoku: Option<ActiveSudoku>,
    active_shop: Option<ActiveShop>,
    active_manipulative: Option<ActiveManipulative>,
    active_quest: Option<ActiveQuest>,
    /// Cosmetics bought from Bolt this session (in-memory; not yet persisted).
    shop_owned: std::collections::HashSet<String>,
    /// Opt-in in-development feature toggles (default all off).
    pub features: FeatureFlags,
    /// Tiles walked since the last random encounter (for encounter pacing).
    steps_since_encounter: u32,
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
    /// Whether the settings overlay's parent-only experimental section is shown.
    parent_panel_open: bool,

    // Soft-block pressure per entity (driver of `Solidity::SoftAfter`).
    // Sparky and the companion are soft-blockers — pressure accumulates while
    // the player walks into one and clears once the player either changes
    // direction or moves. Wandering NPCs are PushableAfter, also tracked here.
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
            sparky: Follower::new(14, 13),
            camera: GameCamera { x: 0.0, y: 0.0 },
            player_path: Vec::new(),
            pending_interact: None,
            auto_interact: false,
            click_target: None,
            npcs,
            npcs_offstage: HashMap::new(),
            companion: None,
            sparky_parked: false,
            sparky_wander_cooldown: 0.0,
            dreaming: false,
            game_time: 0.0,
            play_time: 0.0,
            state: GameState::Title,
            intake: None,
            active_challenge: None,
            active_kenken: None,
            active_pattern: None,
            active_balance: None,
            active_sudoku: None,
            active_shop: None,
            active_manipulative: None,
            active_quest: None,
            shop_owned: std::collections::HashSet::new(),
            features: FeatureFlags::default(),
            steps_since_encounter: 0,
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
            parent_panel_open: false,
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

    /// Test-facing snapshot of the current dialogue's lines as
    /// `(speaker, text)` pairs, in order. Lets story tests assert on who says
    /// what during a scene.
    pub fn dialogue_lines(&self) -> Vec<(String, String)> {
        self.dialogue.lines().iter()
            .map(|l| (l.speaker.clone(), l.text.clone()))
            .collect()
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

    /// Read-only view of the active pattern session (None if none is on screen).
    /// Tests use this with `ui::patterns::layout` to compute click targets.
    pub fn active_pattern(&self) -> Option<&ActivePattern> {
        self.active_pattern.as_ref()
    }

    /// Read-only view of the active balance session (None if none is on screen).
    /// Tests use this with `ui::balance::layout` to compute click targets.
    pub fn active_balance(&self) -> Option<&ActiveBalance> {
        self.active_balance.as_ref()
    }

    /// Read-only view of the active Sudoku session (None if none is on screen).
    /// Tests use this with `ui::sudoku::layout` to compute click targets.
    pub fn active_sudoku(&self) -> Option<&ActiveSudoku> {
        self.active_sudoku.as_ref()
    }

    /// Read-only view of the active shop session (None if the shop is closed).
    pub fn active_shop(&self) -> Option<&ActiveShop> {
        self.active_shop.as_ref()
    }

    /// Read-only view of the active CRA manipulative (None if not in one).
    pub fn active_manipulative(&self) -> Option<&ActiveManipulative> {
        self.active_manipulative.as_ref()
    }

    /// Read-only view of the active quest run (None if not on a quest).
    pub fn active_quest(&self) -> Option<&ActiveQuest> {
        self.active_quest.as_ref()
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
            // Leaving active play cancels any click-to-walk in progress, so the
            // player never auto-resumes a stale path — or auto-interacts with a
            // since-moved tile — after a dialogue / challenge / encounter.
            if self.state == GameState::Playing && new_state != GameState::Playing {
                self.clear_walk();
            }
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
            self.active_pattern = None;
            self.active_balance = None;
            self.active_sudoku = None;
            self.active_shop = None;
            self.active_manipulative = None;
            self.active_quest = None;
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
            if let Some(c) = self.companion.as_mut() { c.animate(dt); }
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
                // A queued walk path is tile-indexed for the old map — drop it.
                self.clear_walk();
            }
        }

        if self.state == GameState::Playing && !arrived_npcs.is_empty() {
            self.handle_npc_portals(&arrived_npcs);
        }

        // Random encounters (opt-in): after a completed tile step, the world
        // may spring something. Off by default — flip the dev flag to playtest.
        if self.features.encounters && arrived && self.state == GameState::Playing && !self.player.moving {
            self.steps_since_encounter = self.steps_since_encounter.saturating_add(1);
            let cfg = EncounterConfig {
                steps_since_last_encounter: self.steps_since_encounter,
                min_steps_between: 15,
                challenge_freq: self.profile.challenge_freq,
                area: self.map.id.to_string(),
            };
            if encounters::should_trigger_encounter(&cfg, &mut self.rng) {
                let kind = encounters::pick_encounter(&cfg, &mut self.rng);
                self.steps_since_encounter = 0;
                self.fire_encounter(kind);
            }
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
            GameState::Playing => { self.step_playing(input, dt, screen); false }
            GameState::InteractionMenu => false,
            GameState::Dialogue => { self.step_dialogue(input); false }
            GameState::Challenge => { self.step_challenge(input, dt, screen); false }
            GameState::KenKen => { self.step_kenken(input, dt, screen); false }
            GameState::Pattern => { self.step_pattern(input, dt, screen); false }
            GameState::Balance => { self.step_balance(input, dt, screen); false }
            GameState::Sudoku => { self.step_sudoku(input, dt, screen); false }
            GameState::Shop => { self.step_shop(input, screen); false }
            GameState::Manipulative => { self.step_manipulative(input, dt, screen); false }
            GameState::Quest => { self.step_quest(input, screen); false }
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
                        self.companion = None;
                        self.sparky_parked = false;
                        self.sparky_wander_cooldown = 0.0;
                        self.player = Entity::new(7, 10);
                        self.player.dir = Dir::Up;
                        self.sparky = Follower::new(8, 10);
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
                        self.sparky = Follower::new(14, 13);
                        self.npcs = npc::npcs_for_map(self.map.id);
                        self.npcs_offstage.clear();
                        self.companion = None;
                        self.sparky_parked = false;
                        self.sparky_wander_cooldown = 0.0;
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

    /// Translate a screen-space tap into a walk path. The renderer centres the
    /// world on `camera + GAME_W/2` and draws 1:1, so a world point maps to
    /// `screen = world - camera + (sw - GAME_W)/2`; inverting gives the formula
    /// below (exact at any window size, not just 960×720). Walks onto the tapped
    /// tile, or up to a tile adjacent to it when the target is solid (a wall, an
    /// NPC, the chest).
    fn set_path_from_click(&mut self, mx: f32, my: f32, screen: (f32, f32)) {
        let (sw, sh) = screen;
        let wx = mx + self.camera.x + (GAME_W - sw) / 2.0;
        let wy = my + self.camera.y + (GAME_H - sh) / 2.0;
        if wx < 0.0 || wy < 0.0 {
            return;
        }
        let goal = ((wx / TILE_SIZE) as usize, (wy / TILE_SIZE) as usize);
        let (w, h) = (self.map.width, self.map.height);
        if goal.0 >= w || goal.1 >= h {
            return;
        }
        let start = (self.player.tile_x, self.player.tile_y);
        let interactable = self.interactable_at(goal);
        // Tiles occupied by another entity are impassable to the router, so a
        // tap never wedges the player walking-in-place against a standing NPC.
        // The player's own tile (start) is never excluded, and the goal of an
        // interactable tap is reached via `find_path_adjacent` anyway.
        let occupied = self.occupied_tiles();
        let path_opt = {
            let map = &self.map;
            let walkable = |c: usize, r: usize| {
                !map.is_solid(c, r) && ((c, r) == start || !occupied.contains(&(c, r)))
            };
            // Walk onto a free tile; walk *up to* a solid or an interactable
            // (NPC / Sparky / chest) you can't stand on.
            if interactable || map.is_solid(goal.0, goal.1) {
                crate::pathfinding::find_path_adjacent(start, goal, w, h, walkable)
            } else {
                crate::pathfinding::find_path(start, goal, w, h, walkable)
            }
        };
        if let Some(path) = path_opt {
            self.player_path = path;
            self.click_target = Some(goal);
            // Remember to auto-interact on arrival when the tap was on something
            // you talk to / open.
            self.pending_interact = if interactable { Some(goal) } else { None };
        }
    }

    /// Cancel any in-progress click-to-walk: drop the path, the pending
    /// interaction, and the on-screen marker.
    fn clear_walk(&mut self) {
        self.player_path.clear();
        self.pending_interact = None;
        self.click_target = None;
    }

    /// Tiles currently occupied by a blocking entity (roster NPCs + on-map
    /// Sparky). The companion is excluded — it trails the player and steps out
    /// of the way rather than blocking a route.
    fn occupied_tiles(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> =
            self.npcs.iter().map(|n| (n.entity.tile_x, n.entity.tile_y)).collect();
        if self.sparky_is_here() {
            out.push((self.sparky.entity.tile_x, self.sparky.entity.tile_y));
        }
        out
    }

    /// Whether `tile` holds something the player interacts with (NPC, Sparky,
    /// or a treasure chest) on the current map.
    fn interactable_at(&self, tile: (usize, usize)) -> bool {
        let (c, r) = tile;
        if r < self.map.height && c < self.map.width && self.map.tiles[r][c] == tilemap::Tile::Chest {
            return true;
        }
        if self.sparky_is_here()
            && (self.sparky.entity.tile_x, self.sparky.entity.tile_y) == tile
        {
            return true;
        }
        self.npcs.iter().any(|n| (n.entity.tile_x, n.entity.tile_y) == tile)
    }

    /// Derive a one-tile move toward the next tile on the queued walk path.
    /// Consumed tiles are dropped as the player arrives; a stale (non-adjacent)
    /// path is abandoned. Returns `Stay` when there's nothing to follow.
    fn next_path_intent(&mut self) -> MoveIntent {
        if self.player.moving || self.player_path.is_empty() {
            return MoveIntent::Stay;
        }
        // Drop any leading tiles the player already stands on (e.g. just arrived).
        while self.player_path.first() == Some(&(self.player.tile_x, self.player.tile_y)) {
            self.player_path.remove(0);
        }
        let Some(&(nx, ny)) = self.player_path.first() else {
            return MoveIntent::Stay;
        };
        let dx = nx as i32 - self.player.tile_x as i32;
        let dy = ny as i32 - self.player.tile_y as i32;
        let dir = match (dx, dy) {
            (1, 0) => Direction::Right,
            (-1, 0) => Direction::Left,
            (0, 1) => Direction::Down,
            (0, -1) => Direction::Up,
            // Path desynced from the player's tile — abandon it.
            _ => {
                self.player_path.clear();
                return MoveIntent::Stay;
            }
        };
        self.player.dir = match dir {
            Direction::Up => Dir::Up,
            Direction::Down => Dir::Down,
            Direction::Left => Dir::Left,
            Direction::Right => Dir::Right,
        };
        MoveIntent::Move(dir)
    }

    fn step_playing(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        // ── Movement: collect intents, resolve, apply ───────────────────
        // A tap on the map sets a walk path (click-to-walk); keyboard input
        // overrides it. The debug overlay owns clicks when it's up.
        if input.mouse_clicked && !self.debug_overlay.visible {
            let (mx, my) = input.mouse_pos;
            self.set_path_from_click(mx, my, screen);
        }
        let player_intent = match read_player_intent(input, &mut self.player) {
            MoveIntent::Move(d) => {
                self.clear_walk(); // keyboard takes over from auto-walk
                MoveIntent::Move(d)
            }
            MoveIntent::Stay => self.next_path_intent(),
        };

        // Arrival: once the walk path is spent and the player is standing still,
        // either fire the queued auto-interact (face the NPC/chest and act as if
        // Space was pressed) or just drop the walk marker.
        if !self.player.moving && self.player_path.is_empty() {
            if let Some(tgt) = self.pending_interact.take() {
                let dx = tgt.0 as i32 - self.player.tile_x as i32;
                let dy = tgt.1 as i32 - self.player.tile_y as i32;
                if dx.unsigned_abs() + dy.unsigned_abs() == 1 {
                    self.player.dir = match (dx, dy) {
                        (1, 0) => Dir::Right,
                        (-1, 0) => Dir::Left,
                        (0, 1) => Dir::Down,
                        _ => Dir::Up,
                    };
                    self.auto_interact = true;
                }
            }
            self.click_target = None;
        }

        let player_at = (self.player.tile_x, self.player.tile_y);
        let sparky_here = self.sparky_is_here();
        // Active Sparky follows the player's path; parked Sparky idles near
        // Professor Gizmo with the same wander roll a kid uses. Off-map
        // parked Sparky doesn't appear in the resolver — see snapshot below.
        let sparky_intent = if self.sparky.entity.moving {
            MoveIntent::Stay
        } else if self.sparky_parked {
            if sparky_here {
                let (intent, face) = npc::next_wander_intent(
                    (self.sparky.entity.tile_x, self.sparky.entity.tile_y),
                    false,
                    (SPARKY_HOME_TX, SPARKY_HOME_TY),
                    npc::WANDER_RADIUS,
                    &mut self.sparky_wander_cooldown,
                    dt, &mut self.rng,
                );
                if let Some(f) = face { self.sparky.entity.dir = f; }
                intent
            } else {
                MoveIntent::Stay
            }
        } else {
            self.sparky.next_intent(player_at)
        };
        let companion_intent = self.companion.as_mut()
            .map(|c| c.next_follower_intent(player_at.0, player_at.1))
            .unwrap_or(MoveIntent::Stay);

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
                    if sparky_here
                        && self.sparky.entity.tile_x == nx
                        && self.sparky.entity.tile_y == ny
                    {
                        Some(EntityId::Sparky)
                    } else if self.companion.as_ref()
                        .map(|c| c.entity.tile_x == nx && c.entity.tile_y == ny)
                        .unwrap_or(false)
                    {
                        Some(EntityId::Companion)
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
        let mut intents: Vec<(EntityId, MoveIntent)> =
            Vec::with_capacity(3 + self.npcs.len());
        intents.push((EntityId::Player, player_intent));
        if sparky_here {
            intents.push((EntityId::Sparky, sparky_intent));
        }
        if self.companion.is_some() {
            intents.push((EntityId::Companion, companion_intent));
        }
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
                    if let Some(c) = self.companion.as_mut() {
                        if let Some(p) = c.pathing.as_mut() {
                            p.record_player_pos(self.player.tile_x, self.player.tile_y);
                        }
                    }
                    self.player.start_move(to.0, to.1);
                    self.pressure.clear();
                }
                MoveResolution::Granted { entity: EntityId::Sparky, to, .. } => {
                    self.sparky.on_move_granted();
                    self.sparky.entity.start_move(to.0, to.1);
                }
                MoveResolution::Granted { entity: EntityId::Companion, to, .. } => {
                    if let Some(c) = self.companion.as_mut() {
                        c.on_follower_move_granted();
                        c.entity.start_move(to.0, to.1);
                    }
                }
                MoveResolution::Granted { entity: EntityId::Npc(i), to, .. } => {
                    if let Some(n) = self.npcs.get_mut(*i as usize) {
                        n.entity.start_move(to.0, to.1);
                    }
                }
                _ => {}
            }
        }

        // Space (or an arrival auto-interact): interact with what's in front.
        if (input.pressed(KeyCode::Space) || std::mem::take(&mut self.auto_interact))
            && !self.player.moving
        {
            self.clear_walk(); // stop auto-walking when the kid interacts
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
            } else if let Some(target) = npc::get_interact_target_with_companion(
                self.player.tile_x, self.player.tile_y, self.player.dir,
                &self.npcs, self.companion.as_ref(),
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
                    has_shop: Some(target_kind == npc::NpcKind::Shopkeeper),
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
            } else if self.sparky_is_here() && npc::is_facing_sparky(
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

    /// Build the per-frame snapshot the resolver consumes. Player is always
    /// present. Sparky shows up only when he's on the current map (active
    /// buddy, or parked at his home tile and the player is visiting). The
    /// companion (if any) is included as soft-block so the player can squeeze
    /// past them. NPCs follow in `Vec` order so `EntityId::Npc(i)` matches
    /// the index in `self.npcs`.
    fn snapshot_entities(&self) -> Vec<EntityState> {
        let mut v = Vec::with_capacity(3 + self.npcs.len());
        v.push(entity_state(EntityId::Player, &self.player, Solidity::Solid, false));
        if self.sparky_is_here() {
            // Active Sparky soft-blocks (player squeezes past after pressure)
            // and phases through other entities while retracing. Parked Sparky
            // is just another loose creature near Gizmo — pushable like a kid,
            // and obeys collisions like one.
            let (sparky_solidity, sparky_phasing) = if self.sparky_parked {
                (Solidity::PushableAfter(0.18), false)
            } else {
                (Solidity::SoftAfter(0.12), true)
            };
            v.push(entity_state(EntityId::Sparky, &self.sparky.entity, sparky_solidity, sparky_phasing));
        }
        if let Some(c) = self.companion.as_ref() {
            // Companion is a follower — same rules as active Sparky.
            v.push(entity_state(EntityId::Companion, &c.entity, Solidity::SoftAfter(0.12), true));
        }
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
            v.push(entity_state(EntityId::Npc(i as u32), &n.entity, solidity, false));
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
                    self.begin_challenge(ac);
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
            CtrlTriggerPattern => {
                let source = kind.as_str().to_string();
                let ap = start_pattern(&mut self.rng, &self.profile, self.game_time, source.clone());
                self.events.push(GameEvent::PatternStarted {
                    level: self.profile.pattern_level,
                    source,
                });
                self.active_pattern = Some(ap);
                self.set_state(GameState::Pattern);
            }
            CtrlTriggerBalance => {
                let source = kind.as_str().to_string();
                let ab = start_balance(&mut self.rng, &self.profile, self.game_time, source.clone());
                self.events.push(GameEvent::BalanceStarted {
                    level: balance::balance_level_for_band(self.profile.math_band),
                    source,
                });
                self.active_balance = Some(ab);
                self.set_state(GameState::Balance);
            }
            CtrlTriggerSudoku => {
                let source = kind.as_str().to_string();
                let asd = start_sudoku(&mut self.rng, &self.profile, self.game_time, source.clone());
                self.events.push(GameEvent::SudokuStarted {
                    grid_size: asd.session.puzzle.grid_size,
                    source,
                });
                self.active_sudoku = Some(asd);
                self.set_state(GameState::Sudoku);
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
            CtrlToggleEncounters => {
                self.features.encounters = !self.features.encounters;
                let on = if self.features.encounters { "ON" } else { "OFF" };
                self.start_dialogue(vec![line(&format!("BEEP. Random encounters are now {on}."))]);
                self.set_state(GameState::Dialogue);
            }
            CtrlTriggerEncounter => {
                // Fire one encounter right now for testing (ignores the flag and
                // the step pacing). Routes through the same handler as live play.
                let cfg = EncounterConfig {
                    steps_since_last_encounter: 999,
                    min_steps_between: 0,
                    challenge_freq: self.profile.challenge_freq,
                    area: self.map.id.to_string(),
                };
                let kind = encounters::pick_encounter(&cfg, &mut self.rng);
                self.fire_encounter(kind);
            }
            CtrlToggleManipulatives => {
                self.features.cra_manipulatives = !self.features.cra_manipulatives;
                let on = if self.features.cra_manipulatives { "ON" } else { "OFF" };
                self.start_dialogue(vec![line(&format!("BEEP. CRA manipulatives are now {on}."))]);
                self.set_state(GameState::Dialogue);
            }
            CtrlTriggerManipulative => {
                // Roll challenges until one maps to a manipulative (add/sub at a
                // concrete/representational CRA stage), then enter it directly.
                // Use a low band so a small, manipulative-friendly add/sub turns
                // up reliably (the dev profile's band 5 is mostly multiplication).
                let mut low = self.profile.clone();
                low.math_band = 1;
                let mut entered = false;
                for _ in 0..64 {
                    let ac = start_challenge(&mut self.rng, &low, self.game_time);
                    if let Some(manip) = try_make_manipulative(&self.profile, &ac.challenge) {
                        self.events.push(GameEvent::ChallengeStarted {
                            question: ac.challenge.display_text.clone(),
                        });
                        self.active_manipulative = Some(ActiveManipulative {
                            manip,
                            challenge: ac.challenge,
                            complete_timer: 0.0,
                            start_time: self.game_time,
                        });
                        self.set_state(GameState::Manipulative);
                        entered = true;
                        break;
                    }
                }
                if !entered {
                    self.start_dialogue(vec![line("No manipulative-friendly challenge rolled. Try again.")]);
                    self.set_state(GameState::Dialogue);
                }
            }
            CtrlToggleQuest => {
                self.features.quest = !self.features.quest;
                let on = if self.features.quest { "ON" } else { "OFF" };
                self.start_dialogue(vec![line(&format!("BEEP. Quests are now {on}."))]);
                self.set_state(GameState::Dialogue);
            }
            CtrlStartQuest => {
                self.start_quest(quest::welcome_quest());
            }
            // Non-dev kinds shouldn't reach here -- caller gates on is_dev_control.
            other => {
                self.start_dialogue(vec![line(&format!("Unknown control: {}", other.as_str()))]);
                self.set_state(GameState::Dialogue);
            }
        }
    }

    /// Route a rolled encounter to the right presentation: flavor/sighting →
    /// dialogue, found Dum Dum → reward + dialogue, challenge → the normal
    /// challenge lifecycle. Caller has already confirmed we're in Playing.
    fn fire_encounter(&mut self, kind: EncounterKind) {
        let label = match &kind {
            EncounterKind::FlavorDialogue { .. } => "flavor",
            EncounterKind::FoundDumDum => "dum_dum",
            EncounterKind::Challenge => "challenge",
            EncounterKind::MathSighting { .. } => "sighting",
        };
        self.events.push(GameEvent::EncounterTriggered { kind: label.into() });
        match kind {
            EncounterKind::FlavorDialogue { speaker, text }
            | EncounterKind::MathSighting { speaker, text } => {
                self.start_dialogue(vec![DialogueLine { speaker, text }]);
                self.set_state(GameState::Dialogue);
            }
            EncounterKind::FoundDumDum => {
                self.dum_dums += 1;
                self.dum_dum_hud.flash();
                self.events.push(GameEvent::DumDumsAwarded { amount: 1 });
                self.start_dialogue(vec![DialogueLine {
                    speaker: "Sparky".into(),
                    text: "Ooh! A shiny Dum Dum, just sitting here!".into(),
                }]);
                self.set_state(GameState::Dialogue);
            }
            EncounterKind::Challenge => {
                let ac = start_challenge(&mut self.rng, &self.profile, self.game_time);
                self.events.push(GameEvent::ChallengeStarted {
                    question: ac.challenge.display_text.clone(),
                });
                audio::tts::speak("Sparky", &ac.challenge.speech_text);
                self.active_challenge = Some(ac);
                self.set_state(GameState::Challenge);
            }
        }
    }

    /// Enter a challenge — as a hands-on CRA manipulative when the feature flag
    /// is on and the learner's CRA stage for this operation warrants it,
    /// otherwise the standard multiple-choice challenge. Either way the same
    /// `ChallengeStarted` event fires and the learner gets the same signal.
    fn begin_challenge(&mut self, ac: ActiveChallenge) {
        self.events.push(GameEvent::ChallengeStarted {
            question: ac.challenge.display_text.clone(),
        });
        if self.features.cra_manipulatives {
            if let Some(manip) = try_make_manipulative(&self.profile, &ac.challenge) {
                // Speak the prompt too, so a TTS-on parent hears it whether the
                // kid gets the quiz or the hands-on version.
                audio::tts::speak("Sparky", &ac.challenge.speech_text);
                self.active_manipulative = Some(ActiveManipulative {
                    manip,
                    challenge: ac.challenge,
                    complete_timer: 0.0,
                    start_time: self.game_time,
                });
                self.set_state(GameState::Manipulative);
                return;
            }
        }
        audio::tts::speak("Sparky", &ac.challenge.speech_text);
        self.active_challenge = Some(ac);
        self.set_state(GameState::Challenge);
    }

    fn step_manipulative(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
        let mut resolve = false;
        if let Some(ref mut am) = self.active_manipulative {
            if am.manip.is_complete() {
                am.complete_timer += dt;
                if am.complete_timer >= 2.0
                    || input.pressed(KeyCode::Space)
                    || input.pressed(KeyCode::Enter)
                    || input.mouse_clicked
                {
                    resolve = true;
                }
            } else {
                let layout = ui::manipulative::layout(&am.manip, screen);
                let intent = if input.mouse_clicked {
                    let (mx, my) = input.mouse_pos;
                    ui::manipulative::handle_click(mx, my, &am.manip, &layout)
                } else {
                    ui::manipulative::handle_key(&am.manip, input, &layout)
                };
                if let Some(intent) = intent {
                    apply_manip_intent(&mut am.manip, intent);
                }
            }
        }

        if resolve {
            if let Some(am) = self.active_manipulative.take() {
                let response_ms = ((self.game_time - am.start_time) as f64 * 1000.0).min(120000.0);
                // Manipulatives complete only when correct — same learner signal
                // as solving the quiz, tagged with the CRA stage actually shown.
                let cra = self.profile.cra_stages.get(&am.challenge.operation).copied();
                let event = LearnerEvent::PuzzleAttempted {
                    correct: true,
                    operation: am.challenge.operation,
                    sub_skill: am.challenge.sub_skill,
                    band: am.challenge.sampled_band,
                    center_band: Some(am.challenge.center_band),
                    response_time_ms: Some(response_ms),
                    hint_used: false,
                    told_me: false,
                    cra_level_shown: cra,
                    timestamp: Some(self.game_time as f64 * 1000.0),
                };
                self.profile = learner_reducer(self.profile.clone(), event);

                let award = 1u32;
                self.dum_dums += award;
                self.dum_dum_hud.flash();
                self.events.push(GameEvent::DumDumsAwarded { amount: award });
                self.events.push(GameEvent::ChallengeResolved { correct: true, response_ms });
            }
            self.set_state(GameState::Playing);
        }
    }

    fn start_quest(&mut self, quest: Quest) {
        let session = quest::quest_reducer(QuestSession::new(quest), QuestAction::Start);
        let puzzle = build_quest_puzzle(&session, &mut self.rng);
        self.active_quest = Some(ActiveQuest { session, puzzle, message: None });
        self.set_state(GameState::Quest);
    }

    fn step_quest(&mut self, input: &FrameInput, screen: (f32, f32)) {
        // Pull the current step (clone) so we can mutate the session afterward.
        let step = match self.active_quest.as_ref().and_then(|aq| aq.session.current_step().cloned()) {
            Some(s) => s,
            None => {
                self.active_quest = None;
                self.set_state(GameState::Playing);
                return;
            }
        };

        let intent = {
            let aq = self.active_quest.as_ref().unwrap();
            let Some(view) = quest_view(aq) else { return };
            let layout = ui::quest::layout(&view, screen);
            if input.mouse_clicked {
                let (mx, my) = input.mouse_pos;
                ui::quest::handle_click(mx, my, &layout)
            } else {
                ui::quest::handle_key(input, &layout)
            }
        };
        let Some(intent) = intent else { return };

        use ui::quest::QuestClick;
        let mut act: Option<QuestAction> = None;
        match intent {
            QuestClick::Continue => match &step {
                QuestStep::Dialogue { .. } => act = Some(QuestAction::AdvanceStep),
                QuestStep::Travel { map, x, y } => {
                    act = Some(QuestAction::ArriveAt { map: map.clone(), x: *x, y: *y })
                }
                QuestStep::Reward { dum_dums } => {
                    self.dum_dums += *dum_dums;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: *dum_dums });
                    act = Some(QuestAction::AdvanceStep);
                }
                QuestStep::Choice { .. } => act = Some(QuestAction::ChooseOption { index: 0 }),
                QuestStep::MathPuzzle { .. } => {}
            },
            QuestClick::Answer(v) => {
                if let QuestStep::MathPuzzle { .. } = &step {
                    let answer = self.active_quest.as_ref().unwrap().puzzle.as_ref().map(|p| p.answer);
                    if Some(v) == answer {
                        act = Some(QuestAction::CompletePuzzle { correct: true });
                    } else {
                        self.active_quest.as_mut().unwrap().message =
                            Some("Hmm, not quite — try again!".into());
                    }
                }
            }
        }

        if let Some(action) = act {
            // Apply on a detached session so self.rng is free for the next
            // puzzle without overlapping the active_quest borrow.
            let mut session = self.active_quest.as_ref().unwrap().session.clone();
            session = quest::quest_reducer(session, action);
            let new_puzzle = build_quest_puzzle(&session, &mut self.rng);
            let complete = session.status == QuestStatus::Complete;
            {
                let aq = self.active_quest.as_mut().unwrap();
                aq.session = session;
                aq.puzzle = new_puzzle;
                aq.message = None;
            }
            if complete {
                self.events.push(GameEvent::QuestCompleted);
                self.active_quest = None;
                self.set_state(GameState::Playing);
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

    fn step_pattern(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
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
                let response_ms = ((self.game_time - ap.start_time) as f64 * 1000.0).min(120000.0);
                let level = self.profile.pattern_level;
                let attempts = ap.session.attempts;

                self.profile = learner_reducer(self.profile.clone(), LearnerEvent::PatternAttempted {
                    correct: was_correct,
                    level,
                    attempts,
                    response_time_ms: Some(response_ms),
                });

                if was_correct {
                    // Same reward shape as a correct challenge or kenken: 1 Dum Dum.
                    let award = 1u32;
                    self.dum_dums += award;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: award });
                }

                self.events.push(GameEvent::PatternResolved {
                    correct: was_correct,
                    level,
                    attempts,
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

    fn step_balance(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
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
                let response_ms = ((self.game_time - ab.start_time) as f64 * 1000.0).min(120000.0);
                let level = balance::balance_level_for_band(self.profile.math_band);
                let attempts = ab.session.attempts;

                if was_correct {
                    let award = 1u32;
                    self.dum_dums += award;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: award });
                }

                self.events.push(GameEvent::BalanceResolved {
                    correct: was_correct,
                    level,
                    attempts,
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

    fn step_sudoku(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
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
                let response_ms = ((self.game_time - asd.start_time) as f64 * 1000.0).min(120000.0);
                let grid_size = asd.session.puzzle.grid_size;
                let violations = asd.session.constraint_violations;

                if was_correct {
                    let award = 1u32;
                    self.dum_dums += award;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: award });
                }

                self.events.push(GameEvent::SudokuResolved {
                    correct: was_correct,
                    grid_size,
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

    fn step_shop(&mut self, input: &FrameInput, screen: (f32, f32)) {
        let Some(ash) = self.active_shop.as_ref() else { return };
        let view = shop_view(ash);
        let layout = ui::shop::layout(&ash.catalog, &view, screen);

        let intent = if input.mouse_clicked {
            let (mx, my) = input.mouse_pos;
            ui::shop::handle_click(mx, my, &layout)
        } else {
            ui::shop::handle_key(input, &layout)
        };
        let Some(intent) = intent else { return };

        match intent {
            ui::shop::ShopInput::Close => {
                if let Some(ash) = self.active_shop.take() {
                    self.shop_owned = ash.owned;
                }
                self.set_state(GameState::Playing);
            }
            ui::shop::ShopInput::SelectItem(i) => {
                let ash = self.active_shop.as_mut().unwrap();
                if ash.selected.is_some() {
                    return; // already solving a purchase
                }
                let item = ash.catalog[i].clone();
                match shop::process_purchase(self.dum_dums, &item.id, &ash.owned) {
                    shop::PurchaseOutcome::Bought { result } => {
                        ash.selected = Some(i);
                        ash.cost = result.spent;
                        ash.answer = result.new_balance;
                        ash.balance_before = self.dum_dums;
                        ash.message = None;
                        let choices = subtraction_choices(self.dum_dums, result.spent, &mut self.rng);
                        ash.choices = choices;
                    }
                    shop::PurchaseOutcome::CantAfford { shortfall } => {
                        ash.message = Some(format!("You need {shortfall} more Dum Dums!"));
                    }
                    shop::PurchaseOutcome::AlreadyOwned => {
                        ash.message = Some("Sparky already has that one!".into());
                    }
                    shop::PurchaseOutcome::UnknownItem => {}
                }
            }
            ui::shop::ShopInput::Answer(v) => {
                let ash = self.active_shop.as_mut().unwrap();
                let Some(i) = ash.selected else { return };
                if v == ash.answer {
                    let item = ash.catalog[i].clone();
                    self.dum_dums = ash.answer;
                    ash.owned.insert(item.id.clone());
                    ash.selected = None;
                    ash.choices.clear();
                    ash.message = Some(format!("Sparky LOVES the {}!", item.name));
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsSpent { amount: ash.cost, item: item.id });
                } else {
                    // Natural consequence, not punishment — recount and retry.
                    ash.message = Some("Hmm, let me count again...".into());
                }
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
                "pattern" => {
                    let source = self.menu_target_id.clone();
                    let ap = start_pattern(&mut self.rng, &self.profile, self.game_time, source);
                    self.events.push(GameEvent::PatternStarted {
                        level: self.profile.pattern_level,
                        source: ap.source_npc.clone(),
                    });
                    self.active_pattern = Some(ap);
                    self.set_state(GameState::Pattern);
                }
                "balance" => {
                    let source = self.menu_target_id.clone();
                    let ab = start_balance(&mut self.rng, &self.profile, self.game_time, source);
                    self.events.push(GameEvent::BalanceStarted {
                        level: balance::balance_level_for_band(self.profile.math_band),
                        source: ab.source_npc.clone(),
                    });
                    self.active_balance = Some(ab);
                    self.set_state(GameState::Balance);
                }
                "sudoku" => {
                    let source = self.menu_target_id.clone();
                    let asd = start_sudoku(&mut self.rng, &self.profile, self.game_time, source);
                    self.events.push(GameEvent::SudokuStarted {
                        grid_size: asd.session.puzzle.grid_size,
                        source: asd.source_npc.clone(),
                    });
                    self.active_sudoku = Some(asd);
                    self.set_state(GameState::Sudoku);
                }
                "shop" => {
                    let source = self.menu_target_id.clone();
                    self.active_shop = Some(ActiveShop {
                        catalog: shop::shop_catalog(),
                        owned: self.shop_owned.clone(),
                        selected: None,
                        choices: Vec::new(),
                        answer: 0,
                        cost: 0,
                        balance_before: 0,
                        message: None,
                        source_npc: source,
                    });
                    self.set_state(GameState::Shop);
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

                        // If the recipient is an NPC who isn't already the
                        // companion, the dum dum recruits them — and any
                        // previous companion returns home.
                        let swap = self.maybe_swap_companion_from_gift();

                        let save_data = self.gather_save_data();
                        self.save_backend.save_to(self.active_slot, &save_data);
                        self.auto_save_timer = 0.0;

                        let reaction = give_reaction_dialogue(
                            &self.menu_target_id, &self.menu_target_name,
                            &result.milestone, &mut self.rng,
                        );
                        let lines = match swap {
                            Some((joined, left)) => buddy_swap_dialogue(&joined, left.as_deref(), reaction),
                            None => reaction,
                        };
                        self.start_dialogue(lines);
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
            use ui::settings_overlay::{Feature, SettingsResult};
            if let Some(result) = ui::settings_overlay::handle_input(input, screen, self.parent_panel_open) {
                match result {
                    // These stay in the overlay — just mutate state, don't close.
                    SettingsResult::ToggleParentPanel => {
                        self.parent_panel_open = !self.parent_panel_open;
                    }
                    SettingsResult::ToggleFeature(f) => match f {
                        Feature::Encounters => self.features.encounters = !self.features.encounters,
                        Feature::Manipulatives => {
                            self.features.cra_manipulatives = !self.features.cra_manipulatives
                        }
                        Feature::Quest => self.features.quest = !self.features.quest,
                    },
                    SettingsResult::Close => {
                        self.settings_open = false;
                        self.parent_panel_open = false;
                    }
                    SettingsResult::BackToTitle => {
                        self.settings_open = false;
                        self.parent_panel_open = false;
                        audio::tts::cancel();
                        self.dialogue.active = false;
                        self.active_challenge = None;
                        self.active_kenken = None;
                        self.active_pattern = None;
                        self.active_balance = None;
                        self.active_sudoku = None;
                        self.active_shop = None;
                        self.active_manipulative = None;
                        self.active_quest = None;
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

    /// True when Sparky is currently visible on the player's map. Sparky is
    /// either actively following (and thus always on the player's map) or
    /// parked at his home tile on `SPARKY_HOME_MAP`. While parked elsewhere,
    /// he should not render, soft-block, or be interactable.
    pub fn sparky_is_here(&self) -> bool {
        !self.sparky_parked || self.map.id == SPARKY_HOME_MAP
    }

    /// Stable id string for the entity currently following the player. Used
    /// downstream by random-event / dialogue systems that want to vary based
    /// on who's tagging along.
    pub fn current_buddy_id(&self) -> &str {
        match self.companion.as_ref() {
            Some(c) => c.kind.as_str(),
            None => "sparky",
        }
    }

    /// Tile + direction for parked Sparky's resting spot. Sparky faces the
    /// player's typical entry direction (Down — toward the path) so the kid
    /// runs into him head-on when arriving at the overworld.
    fn park_sparky(&mut self) {
        self.sparky_parked = true;
        self.sparky.entity.tile_x = SPARKY_HOME_TX;
        self.sparky.entity.tile_y = SPARKY_HOME_TY;
        self.sparky.entity.x = SPARKY_HOME_TX as f32 * TILE_SIZE;
        self.sparky.entity.y = SPARKY_HOME_TY as f32 * TILE_SIZE;
        self.sparky.entity.target_x = self.sparky.entity.x;
        self.sparky.entity.target_y = self.sparky.entity.y;
        self.sparky.entity.moving = false;
        self.sparky.entity.dir = Dir::Down;
        self.sparky.pathing.clear();
        // Initial wander delay so he settles for a beat before twitching.
        self.sparky_wander_cooldown = npc::WANDER_COOLDOWN_MIN;
    }

    /// Sparky rejoins the player as buddy. Teleports him to a spot adjacent
    /// to the player, lets the path queue rebuild naturally as the player
    /// moves. Returns true if Sparky was previously parked (so the caller
    /// knows a swap actually happened).
    fn unpark_sparky(&mut self) -> bool {
        if !self.sparky_parked { return false; }
        self.sparky_parked = false;
        let (px, py) = (self.player.tile_x, self.player.tile_y);
        let pos = find_sparky_spot(px, py, &self.map, &self.npcs);
        self.sparky.entity.tile_x = pos.0;
        self.sparky.entity.tile_y = pos.1;
        self.sparky.entity.x = pos.0 as f32 * TILE_SIZE;
        self.sparky.entity.y = pos.1 as f32 * TILE_SIZE;
        self.sparky.entity.target_x = self.sparky.entity.x;
        self.sparky.entity.target_y = self.sparky.entity.y;
        self.sparky.entity.moving = false;
        self.sparky.pathing.clear();
        true
    }

    /// Build the NPC roster for `map_id`, preferring whatever's currently
    /// stashed offstage and falling back to the static template. Filters out
    /// the current companion's kind so the same NPC can't appear in two
    /// places (next to the player AND back home in the roster).
    fn load_map_roster(&mut self, map_id: &'static str) -> Vec<npc::Npc> {
        let mut roster = self.npcs_offstage
            .remove(map_id)
            .unwrap_or_else(|| npc::npcs_for_map(map_id));
        if let Some(c) = self.companion.as_ref() {
            let companion_kind = c.kind;
            roster.retain(|n| n.kind != companion_kind);
        }
        roster
    }

    /// Resolve the gift recipient (held in `self.menu_target_id`) into a
    /// buddy swap, if applicable. Returns `Some((joined, left))` when a
    /// swap happened (kinds as stable id strings: NPC kinds use
    /// `NpcKind::as_str()`; Sparky is the literal "sparky"). Returns `None`
    /// when the gift doesn't change the buddy — gifting the active buddy
    /// (no-op), or gifting a chest.
    fn maybe_swap_companion_from_gift(&mut self) -> Option<(String, Option<String>)> {
        let target_id = self.menu_target_id.clone();

        // Gifting parked Sparky brings him back as buddy; gifting active Sparky
        // is just a regular gift (no swap).
        if target_id == "sparky" {
            if !self.sparky_parked { return None; }
            let left = self.swap_sparky_in();
            self.events.push(GameEvent::CompanionChanged {
                joined: Some("sparky".to_string()),
                left: Some(left.as_str().to_string()),
            });
            return Some(("sparky".to_string(), Some(left.as_str().to_string())));
        }

        // Gifting an NPC who's in the current roster recruits them. The
        // displaced buddy is whichever was already in the slot — an NPC
        // companion goes home; Sparky parks (handled inside swap_companion_to).
        let idx = self.npcs.iter().position(|n| n.kind.as_str() == target_id)?;
        let (joined, left) = self.swap_companion_to(idx);
        let left_id = left
            .map(|k| k.as_str().to_string())
            .unwrap_or_else(|| "sparky".to_string());
        self.events.push(GameEvent::CompanionChanged {
            joined: Some(joined.as_str().to_string()),
            left: Some(left_id.clone()),
        });
        Some((joined.as_str().to_string(), Some(left_id)))
    }

    /// Make the NPC at `recipient_idx` in `self.npcs` the player's new
    /// companion. If Sparky was the buddy, he's parked at his home tile. If
    /// another NPC was the buddy, they return to their own home (same logic
    /// as a regular NPC-to-NPC swap). Returns the new buddy's kind and, when
    /// the displaced buddy was an NPC, that NPC's kind.
    fn swap_companion_to(&mut self, recipient_idx: usize) -> (NpcKind, Option<NpcKind>) {
        let mut new_companion = self.npcs.remove(recipient_idx);
        let joined = new_companion.kind;
        new_companion.start_following();

        let left = self.companion.replace(new_companion).map(|mut old| {
            let kind = old.kind;
            old.reset_to_home();
            self.send_npc_home(old);
            kind
        });

        // First-time recruitment: Sparky was the active buddy, now he parks.
        if left.is_none() {
            self.park_sparky();
        }

        (joined, left)
    }

    /// Sparky rejoins as buddy. The current NPC companion (must be Some)
    /// returns home. Returns the kind that left so the caller can build a
    /// CompanionChanged event.
    fn swap_sparky_in(&mut self) -> NpcKind {
        let mut leaving = self.companion.take()
            .expect("swap_sparky_in called with no companion to displace");
        let kind = leaving.kind;
        leaving.reset_to_home();
        self.send_npc_home(leaving);
        self.unpark_sparky();
        kind
    }

    /// Place a swapped-out companion back into its home map. Uses
    /// `find_npc_spawn_spot` so a wanderer that drifted onto the home tile
    /// in the meantime doesn't get stomped on.
    fn send_npc_home(&mut self, mut npc: npc::Npc) {
        let home_map = npc.home_map;
        let (hx, hy) = (npc.home_tx, npc.home_ty);

        if home_map == self.map.id {
            let map = &self.map;
            let player = (self.player.tile_x, self.player.tile_y);
            let sparky = (self.sparky.entity.tile_x, self.sparky.entity.tile_y);
            let companion_pos = self.companion.as_ref()
                .map(|c| (c.entity.tile_x, c.entity.tile_y));
            let others: Vec<(usize, usize)> = self.npcs.iter()
                .map(|n| (n.entity.tile_x, n.entity.tile_y))
                .collect();
            let (nx, ny) = npc::find_npc_spawn_spot(
                hx, hy, map.width, map.height,
                |cx, cy| map.is_solid(cx, cy),
                |cx, cy| (cx, cy) == player || (cx, cy) == sparky
                    || companion_pos == Some((cx, cy))
                    || others.iter().any(|t| *t == (cx, cy)),
            );
            npc.entity.tile_x = nx;
            npc.entity.tile_y = ny;
            npc.entity.x = nx as f32 * TILE_SIZE;
            npc.entity.y = ny as f32 * TILE_SIZE;
            npc.entity.target_x = npc.entity.x;
            npc.entity.target_y = npc.entity.y;
            self.npcs.push(npc);
        } else {
            // Home is a different map: stash them in the offstage roster so
            // they pop back when the player next visits that map.
            self.npcs_offstage
                .entry(home_map.to_string())
                .or_insert_with(Vec::new)
                .push(npc);
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
            // Secret portals (dream, doghouse, grove) carry NPCs too — they're
            // ordinary doors as far as a wanderer is concerned. The dream world
            // is just a copy of the overworld and every secret world has a
            // non-secret exit, so a kid that drifts in can drift back out.
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

    /// Snap the active follower (Sparky or the NPC companion) to a free tile
    /// next to the player and clear its path queue. Called after any warp so a
    /// buddy never ends up stranded on the previous map's coordinates with a
    /// stale trail — which would leave them adrift in some random spot, locked
    /// until the player walked over and triggered the adjacency reset. Single
    /// entry point for both follower kinds so that "post-warp reposition" can't
    /// be applied to one and forgotten for the other.
    fn snap_follower_to_player(&mut self) {
        // Per the companion/Sparky invariant, exactly one of these follows at a
        // time: a companion implies parked Sparky, and an unparked Sparky
        // implies no companion. If nothing is following, there's nothing to do.
        let (px, py) = (self.player.tile_x, self.player.tile_y);
        let spot = find_sparky_spot(px, py, &self.map, &self.npcs);
        if let Some(c) = self.companion.as_mut() {
            let e = &mut c.entity;
            e.tile_x = spot.0;
            e.tile_y = spot.1;
            e.x = spot.0 as f32 * TILE_SIZE;
            e.y = spot.1 as f32 * TILE_SIZE;
            e.target_x = e.x;
            e.target_y = e.y;
            e.moving = false;
            if let Some(p) = c.pathing.as_mut() { p.clear(); }
        } else if !self.sparky_parked {
            let e = &mut self.sparky.entity;
            e.tile_x = spot.0;
            e.tile_y = spot.1;
            e.x = spot.0 as f32 * TILE_SIZE;
            e.y = spot.1 as f32 * TILE_SIZE;
            e.target_x = e.x;
            e.target_y = e.y;
            e.moving = false;
            self.sparky.pathing.clear();
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
        // re-entry, then pop the destination's NPC roster (filtered so the
        // current companion doesn't double-render on their home map).
        let leaving = std::mem::take(&mut self.npcs);
        self.npcs_offstage.insert(from_map.clone(), leaving);
        let dest_id = self.map.id;
        self.npcs = self.load_map_roster(dest_id);

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

        // Whoever's tagging along — Sparky or an NPC companion — snaps to the
        // player's side on the destination map with a cleared path queue. One
        // code path for both so a follower can never be left stranded on the
        // old map's coordinates with a stale trail.
        self.snap_follower_to_player();

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

            // Click-to-walk destination marker: a pulsing ring on the tapped
            // tile, drawn on the ground (under the sprites) until arrival.
            if let Some((tc, tr)) = self.click_target {
                let cx = (tc as f32 + 0.5) * TILE_SIZE;
                let cy = (tr as f32 + 0.5) * TILE_SIZE;
                let pulse = (self.game_time * 6.0).sin() * 0.5 + 0.5; // 0..1
                let r = TILE_SIZE * 0.28 + pulse * TILE_SIZE * 0.10;
                let gold = Color::new(1.0, 0.84, 0.30, 0.85);
                draw_circle_lines(cx, cy, r, 3.0, gold);
                draw_circle(cx, cy, 4.0, gold);
            }

            enum SpriteKind<'a> { Player, Sparky, Npc(&'a npc::Npc) }
            struct Renderable<'a> { y: f32, kind: SpriteKind<'a> }
            let mut renderables: Vec<Renderable> = vec![];

            renderables.push(Renderable { y: self.player.y, kind: SpriteKind::Player });
            if self.sparky_is_here() {
                renderables.push(Renderable { y: self.sparky.entity.y, kind: SpriteKind::Sparky });
            }
            if let Some(c) = self.companion.as_ref() {
                renderables.push(Renderable { y: c.entity.y, kind: SpriteKind::Npc(c) });
            }
            // Cull roster NPCs outside the viewport. The map only draws the
            // tiles under the camera (everything else is the void-blue clear
            // color), so an unculled wanderer off to the side would float in
            // that void instead of staying hidden until the camera reaches it.
            let cam = (self.camera.x, self.camera.y);
            for n in &self.npcs {
                if npc_in_camera(cam, n) {
                    renderables.push(Renderable { y: n.entity.y, kind: SpriteKind::Npc(n) });
                }
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

        // Pattern overlay
        if let Some(ref ap) = self.active_pattern {
            let layout = ui::patterns::layout(&ap.session, screen);
            ui::patterns::draw_pattern(&ap.session, &layout, self.game_time);
        }

        // Balance overlay
        if let Some(ref ab) = self.active_balance {
            let layout = ui::balance::layout(&ab.session, screen);
            ui::balance::draw_balance(&ab.session, &layout, self.game_time);
        }

        // Sudoku overlay
        if let Some(ref asd) = self.active_sudoku {
            let layout = ui::sudoku::layout(&asd.session, screen);
            ui::sudoku::draw_sudoku(&asd.session, &layout, asd.selected);
        }

        // Shop overlay
        if let Some(ref ash) = self.active_shop {
            let view = shop_view(ash);
            let layout = ui::shop::layout(&ash.catalog, &view, screen);
            ui::shop::draw_shop(&ash.catalog, &ash.owned, self.dum_dums, &view, &layout, ash.message.as_deref());
        }

        // CRA manipulative overlay
        if let Some(ref am) = self.active_manipulative {
            let layout = ui::manipulative::layout(&am.manip, screen);
            ui::manipulative::draw(&am.manip, &am.challenge.display_text, &layout);
        }

        // Quest overlay
        if let Some(ref aq) = self.active_quest {
            if let Some(view) = quest_view(aq) {
                let layout = ui::quest::layout(&view, screen);
                let title = aq.session.quest.title.clone();
                ui::quest::draw(&view, &title, aq.message.as_deref(), &layout);
            }
        }

        if self.settings_open {
            ui::settings_overlay::draw(screen, self.features, self.parent_panel_open);
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
            sparky_parked: self.sparky_parked,
            math_band: None,
            dum_dums: self.dum_dums,
            play_time: self.play_time,
            timestamp: 0,
            gifts_given: self.gifts_given.clone(),
            profile: self.profile.clone(),
            companion: self.companion.as_ref().map(|c| CompanionSave {
                kind: c.kind.as_str().to_string(),
                home_map: c.home_map.to_string(),
                tile_x: c.entity.tile_x,
                tile_y: c.entity.tile_y,
            }),
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
        self.npcs_offstage.clear();
        // Rehydrate companion first so the roster filter sees it. The kind is
        // looked up via its home map's template — that's where home_tx/home_ty
        // and sprite all live. If the saved kind no longer matches anything in
        // its home roster (shouldn't happen, but might after refactors) we
        // silently drop the companion rather than panic on load.
        self.companion = save_data.companion.as_ref().and_then(|cs| {
            let home_map = Map::by_id(&cs.home_map);
            let mut template = npc::npcs_for_map(home_map.id)
                .into_iter()
                .find(|n| n.id_str() == cs.kind)?;
            template.entity.tile_x = cs.tile_x;
            template.entity.tile_y = cs.tile_y;
            template.entity.x = cs.tile_x as f32 * TILE_SIZE;
            template.entity.y = cs.tile_y as f32 * TILE_SIZE;
            template.entity.target_x = template.entity.x;
            template.entity.target_y = template.entity.y;
            template.entity.moving = false;
            template.start_following();
            Some(template)
        });
        // load_map_roster filters out the companion's kind so they don't
        // appear in two places when the player visits their home map.
        let map_id = self.map.id;
        self.npcs = self.load_map_roster(map_id);

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
        self.sparky.pathing.clear();
        self.sparky_parked = save_data.sparky_parked;
        // Fresh cooldown after load — wander roll won't fire on the first
        // frame, and only ticks if Sparky's parked AND on his home map.
        self.sparky_wander_cooldown = if save_data.sparky_parked {
            npc::WANDER_COOLDOWN_MIN
        } else {
            0.0
        };
    }
}

// ─── Free helpers ──────────────────────────────────────

/// Build the player-facing view of the quest's current step (borrows the step
/// + generated choices). `None` once the quest is complete/inactive.
fn quest_view(aq: &ActiveQuest) -> Option<ui::quest::QuestView<'_>> {
    use ui::quest::QuestView;
    let step = aq.session.current_step()?;
    Some(match step {
        QuestStep::Dialogue { speaker, lines } => QuestView::Narrative { speaker, lines },
        QuestStep::Travel { map, x, y } => {
            QuestView::Travel { label: format!("Head to {map} at ({x}, {y})...") }
        }
        QuestStep::MathPuzzle { context, .. } => {
            let choices = aq.puzzle.as_ref().map(|p| p.choices.as_slice()).unwrap_or(&[]);
            QuestView::Puzzle { prompt: context, choices }
        }
        QuestStep::Choice { prompt, .. } => {
            QuestView::Narrative { speaker: "Choose", lines: std::slice::from_ref(prompt) }
        }
        QuestStep::Reward { dum_dums } => QuestView::Reward { dum_dums: *dum_dums },
    })
}

/// Generate the inline multiple-choice for the current MathPuzzle step (if any)
/// from its operands + operation. The answer is computed; the distractors are
/// nearby values.
fn build_quest_puzzle(session: &QuestSession, rng: &mut SmallRng) -> Option<QuestPuzzle> {
    match session.current_step()? {
        QuestStep::MathPuzzle { operation, operands, .. } => {
            let (a, b) = (*operands)?;
            let (a, b) = (a as i32, b as i32);
            let answer = match operation {
                // Operands may be authored in either order; a quest answer is a
                // count, so keep it non-negative (the magnitude of the difference).
                Operation::Sub => (a - b).abs(),
                Operation::Multiply => a * b,
                Operation::Divide => if b != 0 { a / b } else { 0 },
                _ => a + b, // Add / NumberBond
            };
            Some(QuestPuzzle { choices: quest_answer_choices(answer, rng), answer })
        }
        _ => None,
    }
}

/// Answer tiles for a quest puzzle: the correct value plus nearby positive
/// distractors, shuffled. Always includes the answer.
fn quest_answer_choices(answer: i32, rng: &mut SmallRng) -> Vec<i32> {
    let mut out = vec![answer];
    for d in [answer + 1, answer - 1, answer + 2, answer - 2, answer + 3] {
        if out.len() >= 4 {
            break;
        }
        if d >= 0 && !out.contains(&d) {
            out.push(d);
        }
    }
    out.shuffle(rng);
    out
}

/// Build the shop's current view (Browsing vs. solving a purchase subtraction)
/// from the active session. Borrows the session so the layout/draw can read it.
fn shop_view(ash: &ActiveShop) -> ui::shop::ShopView<'_> {
    match ash.selected {
        Some(i) => ui::shop::ShopView::Buying {
            item: &ash.catalog[i],
            balance: ash.balance_before,
            cost: ash.cost,
            choices: &ash.choices,
        },
        None => ui::shop::ShopView::Browsing,
    }
}

/// Answer tiles for "balance − cost = ?": the correct remainder plus plausible
/// near-miss distractors (forgot to subtract, off-by-one), shuffled, all > 0
/// where possible. Always includes the right answer.
fn subtraction_choices(balance: u32, cost: u32, rng: &mut SmallRng) -> Vec<u32> {
    let answer = balance.saturating_sub(cost);
    let mut out = vec![answer];
    // Common slip-ups make the best distractors.
    for cand in [balance, answer + 1, answer.saturating_sub(1), answer + 2] {
        if out.len() >= 3 {
            break;
        }
        if !out.contains(&cand) {
            out.push(cand);
        }
    }
    out.shuffle(rng);
    out
}

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

fn start_pattern(rng: &mut SmallRng, profile: &LearnerProfile, game_time: f32, source: String) -> ActivePattern {
    let level = profile.pattern_level.max(1);
    let puzzle = generate_for_level(level, rng);
    ActivePattern {
        session: PatternSession::new(puzzle),
        complete_timer: 0.0,
        start_time: game_time,
        source_npc: source,
    }
}

fn start_balance(rng: &mut SmallRng, profile: &LearnerProfile, game_time: f32, source: String) -> ActiveBalance {
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

fn start_sudoku(rng: &mut SmallRng, profile: &LearnerProfile, game_time: f32, source: String) -> ActiveSudoku {
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

/// Choose a CRA manipulative for an add/sub challenge based on the learner's
/// CRA stage for that operation. Concrete → hands-on objects; Representational →
/// number line. Returns `None` for Abstract, other operations, or operands too
/// large for a tidy manipulative (those keep the standard challenge).
fn try_make_manipulative(profile: &LearnerProfile, challenge: &Challenge) -> Option<ui::manipulative::Manip> {
    use ui::manipulative::Manip;
    let op = challenge.operation;
    if !matches!(op, Operation::Add | Operation::Sub) {
        return None;
    }
    let (a, b) = (challenge.numbers.a, challenge.numbers.b);
    if a < 0 || b < 0 || a > 20 || b > 20 {
        return None;
    }
    let (a, b) = (a as u8, b as u8);
    match profile.cra_stages.get(&op).copied().unwrap_or(CraStage::Concrete) {
        CraStage::Concrete => {
            let kind = if op == Operation::Sub { ConcreteKind::TakeAway } else { ConcreteKind::AddGroups };
            // Keep concrete object counts manageable.
            if a.saturating_add(b) > 12 {
                return None;
            }
            let puzzle = generate_concrete(kind, a, b, &mut SmallRng::seed_from_u64(0));
            Some(Manip::Concrete(manipulate_concrete::ConcreteSession::new(puzzle)))
        }
        CraStage::Representational => {
            let puzzle = generate_number_line(a, b, op, &mut SmallRng::seed_from_u64(0));
            Some(Manip::NumberLine(number_line::NumberLineSession::new(puzzle)))
        }
        CraStage::Abstract => None,
    }
}

fn apply_manip_intent(manip: &mut ui::manipulative::Manip, intent: ui::manipulative::ManipInput) {
    use ui::manipulative::{Manip, ManipInput};
    match (manip, intent) {
        (Manip::Concrete(s), ManipInput::Concrete(a)) => {
            *s = manipulate_concrete::concrete_reducer(s.clone(), a);
        }
        (Manip::NumberLine(s), ManipInput::NumberLine(a)) => {
            *s = number_line::number_line_reducer(s.clone(), a);
        }
        // Mismatched pairings can't occur (layout builds inputs from the same
        // session), so ignore them rather than panic.
        _ => {}
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
fn entity_state(id: EntityId, e: &Entity, solidity: Solidity, phasing: bool) -> EntityState {
    if !e.moving {
        return EntityState {
            id, tile_x: e.tile_x, tile_y: e.tile_y,
            moving_to: None, solidity,
            phase_through_entities: phasing,
        };
    }
    // Pixel `(target_x - x)/TILE_SIZE` rounds to the signed tile-delta
    // remaining; subtracting from the (post-start_move) tile coords recovers
    // the source.
    let dx_rem = ((e.target_x - e.x) / TILE_SIZE).round() as i32;
    let dy_rem = ((e.target_y - e.y) / TILE_SIZE).round() as i32;
    let src_x = (e.tile_x as i32 - dx_rem).max(0) as usize;
    let src_y = (e.tile_y as i32 - dy_rem).max(0) as usize;
    EntityState {
        id, tile_x: src_x, tile_y: src_y,
        moving_to: Some((e.tile_x, e.tile_y)), solidity,
        phase_through_entities: phasing,
    }
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
        Pip => &[
            "Squeak! You found my little clearing!",
            "I like to wander in circles. It's very fun!",
            "Got any snacks? I'm always a bit hungry, hehe!",
        ],
        // Dev-control NPCs go through apply_dev_control, never this path.
        CtrlBand | CtrlKenkenLevel | CtrlCraReset | CtrlIntroReset
        | CtrlTriggerKenken | CtrlTriggerPattern | CtrlTriggerBalance
        | CtrlTriggerSudoku | CtrlTriggerChallenge
        | CtrlToggleEncounters | CtrlTriggerEncounter
        | CtrlToggleManipulatives | CtrlTriggerManipulative
        | CtrlToggleQuest | CtrlStartQuest => &["Hello there!"],
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

/// Build the "X joined you!" / "Y heads home" lines that follow the gift
/// reaction whenever a dum dum swaps the buddy. The base `reaction` plays
/// first so the kid sees the giftee react before the role change. `joined`
/// and `left` are stable id strings: NPC kinds use `NpcKind::as_str()`;
/// Sparky uses the literal "sparky".
fn buddy_swap_dialogue(
    joined: &str,
    left: Option<&str>,
    mut reaction: Vec<DialogueLine>,
) -> Vec<DialogueLine> {
    // Each line is spoken by the character it's about — the newcomer says they
    // want to come along, the departing buddy says their goodbye. Sparky only
    // talks when Sparky is the one joining or leaving; he isn't even on the map
    // when an NPC-to-NPC swap happens (e.g. Mommy hands off to Bolt), so making
    // him narrate those would be nonsense.
    let (join_speaker, join_text) = if joined == "sparky" {
        ("Sparky".to_string(), "BEEP BOOP! I'm BACK, boss! Let's go adventuring!".to_string())
    } else {
        let name = display_name_for_buddy_id(joined);
        (name, "Ooh, an adventure? I'd love to come along with you!".to_string())
    };
    reaction.push(DialogueLine { speaker: join_speaker, text: join_text });

    if let Some(prev) = left {
        let (leave_speaker, leave_text) = if prev == "sparky" {
            ("Sparky".to_string(), "Heading back to Professor Gizmo to charge up! Catch ya later, boss!".to_string())
        } else {
            let name = display_name_for_buddy_id(prev);
            (name, "I'll head on home now — have so much fun out there!".to_string())
        };
        reaction.push(DialogueLine { speaker: leave_speaker, text: leave_text });
    }
    reaction
}

fn display_name_for_buddy_id(id: &str) -> String {
    if id == "sparky" { return "Sparky".into(); }
    // Resolve the id straight to its kind — no map-roster walking, so a buddy
    // from any map (even one added later) gets its real name. Raw id is the
    // graceful fallback if the token is unknown.
    npc::NpcKind::from_id(id)
        .map(|k| k.display_name().to_string())
        .unwrap_or_else(|| id.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::InMemoryBackend;
    use ::rand::SeedableRng;
    use ::rand::rngs::SmallRng;
    use robot_buddy_domain::quest::{Quest, QuestAction, QuestSession, QuestStep};

    fn game() -> Game {
        Game::with_backend(7, Box::new(InMemoryBackend::default()))
    }

    // ── Fix #1: leaving Playing cancels an in-progress click-to-walk ──
    #[test]
    fn leaving_playing_clears_walk_state() {
        let mut g = game();
        g.set_state(GameState::Playing);
        g.player_path = vec![(1, 1), (2, 1)];
        g.pending_interact = Some((3, 1));
        g.click_target = Some((3, 1));

        g.set_state(GameState::Dialogue); // e.g. a random encounter interrupts

        assert!(g.player_path.is_empty(), "walk path must not survive into another state");
        assert_eq!(g.pending_interact, None, "pending auto-interact must be cancelled");
        assert_eq!(g.click_target, None, "walk marker must be cleared");
    }

    // ── Fix #2: tap→tile mapping holds when the window isn't 960×720 ──
    #[test]
    fn click_maps_to_tile_at_any_window_size() {
        const TILE: f32 = TILE_SIZE;
        for (sw, sh) in [(GAME_W, GAME_H), (480.0, 360.0), (1280.0, 800.0)] {
            let mut g = game();
            g.camera = GameCamera { x: 0.0, y: 0.0 };
            let goal = (g.player.tile_x, g.player.tile_y); // own tile: always reachable
            // Screen position the renderer would put this tile's centre at.
            let sx = (goal.0 as f32 + 0.5) * TILE - g.camera.x + (sw - GAME_W) / 2.0;
            let sy = (goal.1 as f32 + 0.5) * TILE - g.camera.y + (sh - GAME_H) / 2.0;
            g.set_path_from_click(sx, sy, (sw, sh));
            assert_eq!(
                g.click_target,
                Some(goal),
                "tap should resolve to the intended tile at window {sw}x{sh}",
            );
        }
    }

    // ── Fix #3: a quest subtraction step never yields a negative answer ──
    #[test]
    fn quest_subtraction_answer_is_non_negative_either_operand_order() {
        for (a, b) in [(2u16, 5u16), (5, 2), (3, 3)] {
            let quest = Quest {
                id: "t".into(),
                title: "t".into(),
                description: "t".into(),
                steps: vec![QuestStep::MathPuzzle {
                    operation: Operation::Sub,
                    band: 3,
                    context: "take away".into(),
                    operands: Some((a, b)),
                }],
                math_domain: vec![Operation::Sub],
                min_band: 1,
                max_band: 5,
            };
            let session = quest::quest_reducer(QuestSession::new(quest), QuestAction::Start);
            let mut rng = SmallRng::seed_from_u64(1);
            let p = build_quest_puzzle(&session, &mut rng).expect("math step yields a puzzle");
            assert_eq!(p.answer, (a as i32 - b as i32).abs());
            assert!(p.answer >= 0);
            assert!(p.choices.contains(&p.answer));
            assert!(p.choices.len() >= 2, "need at least two tiles, got {:?}", p.choices);
            assert!(p.choices.iter().all(|&c| c >= 0), "no negative tiles: {:?}", p.choices);
        }
    }

    #[test]
    fn quest_answer_choices_are_non_negative_and_distinct() {
        let mut rng = SmallRng::seed_from_u64(2);
        for answer in 0..6 {
            let ch = quest_answer_choices(answer, &mut rng);
            assert!(ch.contains(&answer));
            assert!(ch.iter().all(|&c| c >= 0));
            let mut sorted = ch.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), ch.len(), "choices distinct: {ch:?}");
        }
    }
}
