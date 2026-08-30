//! The Game struct: all state, all logic, all rendering.
//!
//! Production: `main()` captures input from macroquad, calls `step()`, awaits next_frame.
//! Tests: build a `FrameInput` synthetically and call `step()` directly. (Tests still need
//! a macroquad window today because draw calls run unconditionally — Phase 4 will split.)

use crate::prelude::*;
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
    IntakeAnswer, generate_intake_question, process_intake_results, next_intake_band, intake_complete,
};
use robot_buddy_domain::economy::give;
use robot_buddy_domain::economy::rewards;
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
use robot_buddy_domain::logic::descent::{
    DiveAction, DiveNudge, DivePhase, DiveSession, dive_reducer, generate_dive,
};
use robot_buddy_domain::logic::leap::{
    Clue, LeapAction, LeapPhase, LeapPuzzle, LeapSession, generate_leap, leap_reducer,
};
use robot_buddy_domain::logic::shooter::{
    ShooterSession, ShooterAction, ShooterPhase, shooter_reducer,
};
use robot_buddy_domain::logic::sudoku::{
    self, SudokuPhase, SudokuSession, generate_for_level as generate_sudoku_for_level,
};
use robot_buddy_domain::economy::shop::{self, Currency, ItemKind, ShopItem, ShopKind};
use robot_buddy_domain::economy::wardrobe::{self, HandOver, Wardrobe};
use robot_buddy_domain::world::encounters::{self, EncounterConfig, EncounterKind};
use robot_buddy_domain::quest::{self, Quest, QuestAction, QuestSession, QuestStatus, QuestStep};
use robot_buddy_domain::types::{Phase, CraStage, FrustrationLevel, GamePace, Operation};
use robot_buddy_domain::world::movement::{
    Direction, EntityId, EntityState, GridDims, MoveIntent, MoveResolution,
    Solidity, resolve_moves,
};

use crate::tilemap::{self, Map, TILE_SIZE};
use crate::sprites::{self, Dir};
use crate::follower::Follower;
use crate::npc::{self, NpcKind};
use crate::number_track;
use crate::ui;
use crate::ui::dialogue::{DialogueBox, DialogueLine};
use crate::ui::challenge::{ChoiceBound, ScaffoldBounds};
use crate::ui::title_screen::{TitleAction, NewGameAction, NewGameForm};
use crate::ui::hud::{DumDumHud, PearlHud, DebugOverlay};
use crate::ui::interaction_menu::MenuOption;
use crate::save::{self, CompanionSave, SaveBackend, SaveData, SaveSlots, Gender};
use crate::audio;
use crate::session;
use crate::input::FrameInput;

pub const GAME_W: f32 = 960.0;
pub const GAME_H: f32 = 720.0;
const MOVE_SPEED: f32 = 200.0;
/// How long one of Shelly's leaps takes, whatever its size — so a leap reads
/// as a jump rather than a long swim, and a big leap still feels like one hop.
const LEAP_SECONDS: f32 = 0.4;
/// A full fuel tank. Refilling at a depot tops the rocket back up to this.
const FUEL_MAX: u32 = 10;

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
    /// Handing a piece of shop swag to a buddy.
    Swag,
    /// Diving the shaft to the trench — the descent minigame.
    Descent,
    Quest,
    /// The Goyish Map's number-bond space shooter (real-time minigame).
    Shooter,
}

/// Opt-in toggles for in-development paths that aren't ready for default play.
/// All default OFF so the test suite and normal play are unaffected; flip them
/// on in the dev control room (or in production wiring) to playtest. A drop of
/// tech debt traded for being able to try these before they're fully baked.
#[derive(Clone, Copy, Debug, Default)]
pub struct FeatureFlags {
    /// Random encounters fire as the kid explores.
    pub encounters: bool,
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

/// The number-bond space shooter, live. The domain `ShooterSession` holds all
/// the game state (ship, aliens, waves, shield); the rest is UI-only bookkeeping
/// mirroring the other `Active*` structs.
pub struct ActiveShooter {
    pub session: ShooterSession,
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

pub struct ActiveShop {
    /// Which counter this is — decides the currency, the title, and whether
    /// `owned` means "worn" (Bolt's swag) or "bought" (Hermie's upgrades).
    pub shop: ShopKind,
    pub catalog: Vec<ShopItem>,
    pub owned: std::collections::BTreeSet<String>,
    /// `Some(index)` while solving the purchase subtraction for that catalog
    /// item; `None` while browsing.
    pub selected: Option<usize>,
    pub choices: Vec<u32>,
    pub answer: u32,
    pub cost: u32,
    pub balance_before: u32,
    pub message: Option<String>,
    pub source_npc: String,
    /// True while the outfit-color swatches are up (after buying Color
    /// Change, or re-opened from its catalog row).
    pub picking_color: bool,
    /// The quote on the counter while the kid works out a pearl trade.
    pub trading: Option<shop::TradeQuote>,
}

/// A live "Give Swag" session: the kid is picking which of the pieces they're
/// wearing to hand to `recipient_id`. Rebuilt from the wardrobe after every
/// hand-over, so the list always shows what's still on the kid.
pub struct ActiveSwag {
    pub recipient_id: String,
    pub recipient_name: String,
    /// Sprite to preview the recipient with. `None` is Sparky, who's a robot
    /// rather than a roster NPC.
    pub recipient_sprite: Option<npc::SpriteType>,
    /// Catalog entries for the swag the kid is wearing, cheapest first.
    pub items: Vec<ShopItem>,
    pub message: Option<String>,
}

/// A live descent: the kid is kicking down the shaft looking for the trench
/// door. Lives only while `GameState::Descent` is up; bailing drops it.
pub struct ActiveDescent {
    pub session: DiveSession,
    /// Beat held after landing so the kid sees the door open before the map
    /// swaps out from under them.
    pub landed_timer: f32,
    pub message: Option<String>,
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
    /// Pixels per second for the move in flight. Walking pace by default; a
    /// leap raises it for one hop and it resets on arrival.
    pub speed: f32,
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
            speed: MOVE_SPEED,
        }
    }

    pub fn move_toward_target(&mut self, dt: f32) -> bool {
        if !self.moving { return false; }
        let dx = self.target_x - self.x;
        let dy = self.target_y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = self.speed * dt;
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
            self.speed = MOVE_SPEED; // a one-off leap speed never sticks
            self.frame += 1;
            return true;
        }
        self.x += dx / dist * step;
        self.y += dy / dist * step;
        false
    }

    /// Send this entity to a tile at a one-off speed, so a multi-tile leap
    /// takes about as long as a single step instead of trudging across the
    /// gap. Speed resets to walking pace on arrival.
    pub fn start_leap(&mut self, nx: usize, ny: usize, seconds: f32) {
        let dx = nx as f32 * TILE_SIZE - self.x;
        let dy = ny as f32 * TILE_SIZE - self.y;
        let dist = (dx * dx + dy * dy).sqrt();
        self.start_move(nx, ny);
        self.speed = (dist / seconds.max(0.05)).max(MOVE_SPEED);
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
    /// A gate guardian's puzzle was solved; the passage is now open.
    GateOpened { gate_id: String },
    /// The rocket spent fuel taking a space jump.
    FuelSpent { amount: u32, remaining: u32 },
    /// A fuel depot refilled the rocket after its puzzle was solved.
    Refueled { to: u32 },
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
    /// Pearls spent at Hermie's deep stall.
    PearlsSpent { amount: u32, item: String },
    /// A trip to Hermie's trade desk: pearls in, Dum Dums out, remainder kept.
    PearlsTraded { pearls: u32, dum_dums: u32, left_over: u32 },
    /// A piece of shop swag changed hands: the kid took it off, `recipient`
    /// (an NPC id or "sparky") put it on and keeps it from here on.
    SwagGiven { item: String, recipient: String },
    /// A descent started: the shaft's door depth and the fewest kicks that
    /// reach it.
    DescentStarted { door: u8, optimal: u8 },
    /// The diver rested on the trench door. `kicks` vs `optimal` is the silent
    /// read on how efficiently they decomposed the depth.
    DescentLanded { door: u8, kicks: u8, optimal: u8 },
    /// A random encounter fired ("flavor" | "dum_dum" | "challenge" | "sighting").
    EncounterTriggered { kind: String },
    /// A quest run reached its final step.
    QuestCompleted,
    /// The kid found Shelly's pearl: landed on the called-out stone of a
    /// number path. `mark` is the stone's number; `jumps` is how many stone-
    /// to-stone hops the kid took vs the `optimal` straight count-on from
    /// where they stepped onto the path — silent efficiency signal for the
    /// adaptive system (never shown to the kid).
    NumberLineReached { mark: u8, jumps: u8, optimal: u8 },
    /// Shelly set up a pearl trip: the pearl's stone and the leap size/count
    /// that reaches it.
    LeapTripOffered { pearl: u8, size: u8, count: u8 },
    /// The kid landed on Shelly's pearl. `resets` is how many wrong leap sizes
    /// they tried first — the silent read on whether the size was reasoned out
    /// or found by trial (never shown to the kid).
    PearlFound { stone: u8, size: u8, leaps: u8, resets: u8, pearls: u32 },
    SudokuStarted { grid_size: u8, source: String },
    SudokuResolved {
        correct: bool,
        grid_size: u8,
        constraint_violations: u8,
        response_ms: f64,
    },
    /// The number-bond space shooter launched from the Goyish Map.
    ShooterStarted { band: u8, source: String },
    /// A shooter wave was fully cleared; `wave` is the just-cleared wave index.
    ShooterWaveCleared { wave: u8 },
    /// The shooter run ended. `waves` is how many were cleared; `hits`/`misses`
    /// are correct/incorrect number-bond pairings (stealth-assessment signal).
    ShooterResolved { waves: u8, hits: u32, misses: u32, response_ms: f64 },
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
    /// Which map parked Sparky is currently loitering on. Starts at
    /// `SPARKY_HOME_MAP` when he parks, but — like any wanderer — if he drifts
    /// or gets pushed onto a portal tile he travels through it, so this tracks
    /// where he ended up. Only meaningful while `sparky_parked`.
    sparky_map: &'static str,
    /// The pearl trip in progress on this map's stone path — Shelly's chosen
    /// leap size and where the kid has leapt to. Only lives while they're
    /// standing on the stone it thinks they're on, so walking around the path
    /// can never pass for leaping it. See `check_number_track_landing`.
    leap_session: Option<LeapSession>,
    /// Brief floating cheer text + remaining seconds, shown after a collection.
    track_toast: Option<(String, f32)>,
    /// Reef-local currency, earned hopping the number path and (later) from the
    /// deeper zones; spent at the reef trader on diving gear.
    pub pearls: u32,
    pearl_hud: PearlHud,
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
    active_shooter: Option<ActiveShooter>,
    active_shop: Option<ActiveShop>,
    active_swag: Option<ActiveSwag>,
    active_descent: Option<ActiveDescent>,
    active_quest: Option<ActiveQuest>,
    /// Cosmetics bought from Bolt (persisted in the save).
    /// Who's wearing which shop swag — the kid included, under
    /// `wardrobe::PLAYER`. Swag handed to a buddy leaves the kid's outfit
    /// (which is what frees Bolt to sell them another one) and stays on that
    /// buddy whether or not they're the one currently tagging along.
    wardrobe: Wardrobe,
    /// Outfit color id for the Color Change cosmetic (persisted in the save).
    color_choice: String,
    /// Opt-in in-development feature toggles (default all off).
    pub features: FeatureFlags,
    /// Tiles walked since the last random encounter (for encounter pacing).
    steps_since_encounter: u32,
    pending_challenge: bool,
    /// Gate id whose challenge is currently on screen (set when the kid takes
    /// on a gate guardian; cleared when that challenge resolves).
    opening_gate: Option<String>,
    /// Gate ids the kid has already solved. Persisted so a guardian stays
    /// stepped-aside across sessions. Reusable for any map's gates.
    satisfied_gates: std::collections::HashSet<String>,
    /// Destination map ids whose one-time entry toll has been paid. After the
    /// first paid trip, that portal is free forever. Persisted. Reusable.
    paid_tolls: std::collections::HashSet<String>,
    /// Secret map ids whose arrival cutscene has already played. Persisted, so
    /// the long "we're UNDERWATER!" speech is a first-time thrill instead of a
    /// toll paid on every dive.
    seen_intros: std::collections::HashSet<String>,
    /// How fast the arcade cabinet runs. A parent dial, set in the parent
    /// section of settings and persisted per save slot — the kid never sees a
    /// label for it (Invariant 6). It changes the clock, never the numbers.
    pub game_pace: GamePace,
    /// Permanent perks bought at a counter (currently Hermie's Diving Net).
    /// Not wearable and never given away — once bought, always on. Persisted.
    upgrades: std::collections::BTreeSet<String>,
    /// Rocket fuel for space jumps. Spent per fuel-costed portal, refilled by
    /// solving Tank the fuel droid's puzzle. Persisted.
    fuel: u32,
    /// True while a fuel-depot's refill puzzle is on screen (set on interact,
    /// consumed when the challenge resolves).
    pending_refuel: bool,
    /// Counts down after fuel is spent or refilled so the gauge pulses — makes
    /// the otherwise-silent fuel change register.
    fuel_flash: f32,
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
            sparky_map: SPARKY_HOME_MAP,
            leap_session: None,
            track_toast: None,
            pearls: 0,
            pearl_hud: PearlHud::new(),
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
            active_shooter: None,
            active_shop: None,
            active_swag: None,
            active_descent: None,
            active_quest: None,
            wardrobe: Wardrobe::new(),
            color_choice: sprites::player::OUTFIT_COLORS[0].0.to_string(),
            features: FeatureFlags::default(),
            steps_since_encounter: 0,
            pending_challenge: false,
            opening_gate: None,
            satisfied_gates: std::collections::HashSet::new(),
            paid_tolls: std::collections::HashSet::new(),
            seen_intros: std::collections::HashSet::new(),
            game_pace: GamePace::default(),
            upgrades: std::collections::BTreeSet::new(),
            fuel: FUEL_MAX,
            pending_refuel: false,
            fuel_flash: 0.0,
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

    /// True once the kid has solved the gate guardian with this id. Lets tests
    /// (and any future UI) check a gate's open state without exposing the set.
    pub fn gate_is_solved(&self, gate_id: &str) -> bool {
        self.satisfied_gates.contains(gate_id)
    }

    /// Current rocket fuel. Exposed for tests / future HUD reads.
    pub fn fuel(&self) -> u32 { self.fuel }

    /// Set rocket fuel (tests use this to set up jump scenarios).
    pub fn set_fuel(&mut self, fuel: u32) { self.fuel = fuel; }

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

    pub fn active_shooter(&self) -> Option<&ActiveShooter> {
        self.active_shooter.as_ref()
    }

    /// Read-only view of the active shop session (None if the shop is closed).
    pub fn active_shop(&self) -> Option<&ActiveShop> {
        self.active_shop.as_ref()
    }

    /// Mutable wardrobe access, for tests and dev tooling that need to dress
    /// somebody without walking them through the shop.
    pub fn wardrobe_mut(&mut self) -> &mut Wardrobe {
        &mut self.wardrobe
    }

    pub fn active_swag(&self) -> Option<&ActiveSwag> {
        self.active_swag.as_ref()
    }

    /// The pearl trip in progress, if the kid is standing on Shelly's stones.
    pub fn leap_session(&self) -> Option<&LeapSession> {
        self.leap_session.as_ref()
    }

    pub fn active_descent(&self) -> Option<&ActiveDescent> {
        self.active_descent.as_ref()
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

        // Tap the on-screen gear to open settings (so it works without a
        // keyboard). Handle it before dispatch so the same tap can't also start
        // a click-to-walk; consume the frame.
        if !self.settings_open && self.state == GameState::Playing && input.mouse_clicked {
            let (gx, gy, gw, gh) = settings_gear_rect(screen);
            let (mx, my) = input.mouse_pos;
            if mx >= gx && mx <= gx + gw && my >= gy && my <= gy + gh {
                self.settings_open = true;
                return;
            }
        }

        let early_exit = if self.settings_open {
            false
        } else {
            self.dispatch_state(input, dt, screen)
        };
        if early_exit { return; }

        // P opens the parent overlay (settings, with the parent section already
        // expanded so the feature flags are right there).
        if !self.settings_open && input.pressed(KeyCode::P)
            && self.state != GameState::Title && self.state != GameState::NewGame
        {
            self.settings_open = true;
            self.parent_panel_open = true;
        }

        // Backtick toggles the dev debug overlay. Accept both the keycode and
        // the typed '`' char — on web these arrive via independent browser
        // events (keydown vs. keypress), so honoring either is robust to
        // keyboard-mapping quirks. (Session export is also on the parent panel,
        // reachable by mouse, if the key still won't cooperate.)
        let backtick = input.pressed(KeyCode::GraveAccent) || input.chars_typed.contains(&'`');
        if !self.settings_open && backtick
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
            self.active_shooter = None;
            self.active_shop = None;
            self.active_swag = None;
            self.active_descent = None;
            self.active_quest = None;
            self.pending_challenge = false;
        }
        self.dum_dum_hud.update(dt);
        self.pearl_hud.update(dt);
        if let Some((_, ref mut t)) = self.track_toast {
            *t -= dt;
            if *t <= 0.0 { self.track_toast = None; }
        }
        if self.fuel_flash > 0.0 { self.fuel_flash -= dt; }

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
        let mut sparky_arrived = false;
        let arrived = if self.settings_open {
            false
        } else {
            let a = self.player.move_toward_target(dt);
            sparky_arrived = self.sparky.animate(dt);
            if let Some(c) = self.companion.as_mut() { c.animate(dt); }
            for (i, n) in self.npcs.iter_mut().enumerate() {
                if n.animate(dt) { arrived_npcs.push(i); }
            }
            self.dialogue.update(dt);
            a
        };

        // A rideable buddy (Chompy the shark) is a mount: lock it to the
        // player's exact position and facing every frame so the kid rides it
        // rather than the shark trailing behind. Done after the player has
        // animated this frame so the mount tracks pixel-for-pixel.
        if let Some(c) = self.companion.as_mut() {
            if c.is_rideable() {
                c.entity.tile_x = self.player.tile_x;
                c.entity.tile_y = self.player.tile_y;
                c.entity.x = self.player.x;
                c.entity.y = self.player.y;
                c.entity.target_x = self.player.target_x;
                c.entity.target_y = self.player.target_y;
                c.entity.moving = self.player.moving;
                c.entity.dir = self.player.dir;
            }
        }

        // Parked Sparky is a loose creature near Gizmo — if he's nudged or
        // pushed onto a portal tile, he travels through it like any wanderer
        // would, instead of just standing on the doorway. Only fires while he's
        // parked and on the player's current map (the only time he can move).
        if sparky_arrived && self.state == GameState::Playing
            && self.sparky_parked && self.sparky_is_here()
        {
            self.handle_parked_sparky_portal();
        }

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

        // Buddies heading off-map blink home once they've walked out of view.
        if self.state == GameState::Playing {
            self.evict_offscreen_leavers(screen);
            self.check_number_track_landing(dt);
        }

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
            GameState::Shooter => { self.step_shooter(input, dt, screen); false }
            GameState::Shop => { self.step_shop(input, screen); false }
            GameState::Swag => { self.step_swag(input, screen); false }
            GameState::Descent => { self.step_descent(input, dt, screen); false }
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
                        // Intake runs before any buddy can be recruited — Sparky.
                        speak_challenge_feedback(&ac.state, "Sparky");
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
                            speak_challenge_feedback(&ac.state, "Sparky");
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

                    // Adaptive length: stop as soon as placement has converged
                    // (bracketed / floored / ceilinged) so low-level intake
                    // isn't a string of identical band-1 questions.
                    let ceiling = (iq.configured_band as u16 + 2).min(10) as u8;
                    if intake_complete(&iq.answers, ceiling) {
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
        // On Shelly's stones the kid leaps rather than walks — the current in
        // the gaps makes ordinary steps impossible anyway. Handled first so a
        // tap on her panel never doubles as a click-to-walk.
        if self.handle_leap_input(input, screen) {
            return;
        }
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
        // A rideable buddy doesn't path-follow — it's pinned to the player —
        // so it neither generates a follow intent nor enters the resolver.
        let companion_rideable = self.companion.as_ref().map(|c| c.is_rideable()).unwrap_or(false);
        let companion_intent = if companion_rideable {
            MoveIntent::Stay
        } else {
            self.companion.as_mut()
                .map(|c| c.next_follower_intent(player_at.0, player_at.1))
                .unwrap_or(MoveIntent::Stay)
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
        if self.companion.is_some() && !companion_rideable {
            intents.push((EntityId::Companion, companion_intent));
        }
        // Snapshot the camera rect once so the wander gate doesn't re-borrow
        // self mid-iteration. Off-screen wanderers freeze: no cooldown tick,
        // no random direction roll. The kid you can't see isn't burning RNG.
        let view = visible_world_rect((self.camera.x, self.camera.y), screen);
        for (i, n) in self.npcs.iter_mut().enumerate() {
            let intent = if n.homing {
                // A buddy walking back to its spot finishes the trip even if it
                // strolls off-screen — it's a short, finite route.
                n.next_homing_intent()
            } else if npc_in_camera(view, n) {
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
                        // A rideable mount isn't retracing a path — skip its queue.
                        if !c.is_rideable() {
                            if let Some(p) = c.pathing.as_mut() {
                                p.record_player_pos(self.player.tile_x, self.player.tile_y);
                            }
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
                        // A homing buddy advances its route queue on each grant.
                        if n.homing { n.on_follower_move_granted(); }
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
                let buddy = self.current_buddy_name();
                self.menu_target_id = "chest".into();
                self.menu_target_name = buddy.clone();
                self.start_dialogue(vec![DialogueLine {
                    speaker: buddy,
                    text: "OOOOH a treasure chest! But it has a LOCK! We need to solve the puzzle to open it!".into(),
                }]);
                self.pending_challenge = true;
                self.set_state(GameState::Dialogue);
            } else if let Some(target) = npc::get_interact_target_with_companion(
                self.player.tile_x, self.player.tile_y, self.player.dir,
                &self.npcs, self.companion.as_ref(),
            ).map(|n| (n.kind, n.can_receive_gifts, n.never_challenge, n.is_puzzler, n.gate, n.gate_id, n.refuel, n.launch_shooter, n.dive, n)) {
                let (target_kind, can_receive_gifts, never_challenge, is_puzzler, is_gate, gate_id, is_refuel, is_launch_shooter, is_dive, target_ref) = target;
                let target_id = target_kind.as_str().to_string();
                let target_name = target_kind.display_name().to_string();

                // Dev knob bay NPCs short-circuit the normal interaction flow.
                // Each ctrl_* kind maps to one effect -- cycle a profile field,
                // reset a flag, or fire a fresh puzzle.
                if target_kind.is_dev_control() {
                    self.apply_dev_control(target_kind);
                    return;
                }

                // A closed gate guardian short-circuits too: it always poses a
                // puzzle (no menu, no random roll). Solve it and it steps aside.
                // Reuses the chest's pending_challenge path; `opening_gate`
                // remembers which gate to open when the puzzle resolves.
                if is_gate {
                    self.menu_target_id = target_id;
                    self.menu_target_name = target_name.clone();
                    self.opening_gate = gate_id.map(|s| s.to_string());
                    self.start_dialogue(vec![DialogueLine {
                        speaker: target_name,
                        text: "*yaaawn* Oh, hello! I'm napping right across the path. Solve a little number puzzle for me and I'll scooch aside, deal?".into(),
                    }]);
                    self.pending_challenge = true;
                    self.set_state(GameState::Dialogue);
                    return;
                }

                // A fuel depot always poses a puzzle; solving it tops up the
                // tank. Like the gate, it short-circuits the normal menu.
                if is_refuel {
                    self.menu_target_id = target_id;
                    self.menu_target_name = target_name.clone();
                    self.pending_refuel = true;
                    self.pending_challenge = true;
                    self.start_dialogue(vec![DialogueLine {
                        speaker: target_name,
                        text: "BEEP BOOP! Solve a number puzzle and I'll fill the rocket right up to the top!".into(),
                    }]);
                    self.set_state(GameState::Dialogue);
                    return;
                }

                // The arcade operator launches the number-bond shooter straight
                // away — a self-contained minigame that never routes through the
                // challenge/dialogue states and back.
                if is_launch_shooter {
                    self.start_shooter(target_name);
                    return;
                }

                let npc_info = NpcInfo {
                    id: target_id.clone(),
                    can_receive_gifts: Some(can_receive_gifts),
                    has_shop: Some(matches!(target_kind,
                        npc::NpcKind::Shopkeeper | npc::NpcKind::HermitCrab)),
                    is_puzzler: Some(is_puzzler),
                    runs_dive: Some(is_dive),
                };
                let player_st = PlayerState { dum_dums: self.dum_dums, swag_worn: self.player_swag().len() as u32 };
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
                    runs_dive: None,
                };
                let player_st = PlayerState { dum_dums: self.dum_dums, swag_worn: self.player_swag().len() as u32 };
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
            } else if self.companion.as_ref().is_some_and(|c| c.is_rideable()) {
                // You're sitting ON this buddy, so there's no tile to face them
                // from — reaching out over their nose is the only way to talk to
                // (or dress up) your own mount.
                let (kind, can_gift, never_challenge) = {
                    let c = self.companion.as_ref().unwrap();
                    (c.kind, c.can_receive_gifts, c.never_challenge)
                };
                let npc_info = NpcInfo {
                    id: kind.as_str().to_string(),
                    can_receive_gifts: Some(can_gift),
                    has_shop: None,
                    is_puzzler: Some(false),
                    runs_dive: None,
                };
                let player_st = PlayerState { dum_dums: self.dum_dums, swag_worn: self.player_swag().len() as u32 };
                let opts = interaction_options::get_interaction_options(&npc_info, &player_st);
                self.menu_target_id = kind.as_str().to_string();
                self.menu_target_name = kind.display_name().to_string();
                self.menu_can_challenge = !never_challenge;

                if opts.len() == 1 {
                    let lines = self.companion.as_ref()
                        .map(|c| npc_dialogue_lines(c, &mut self.rng))
                        .unwrap_or_default();
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
            // A rideable mount shares the player's tile — it never collides as a
            // separate entity, so keep it out of the resolver snapshot.
            if !c.is_rideable() {
                // Companion is a follower — same rules as active Sparky.
                v.push(entity_state(EntityId::Companion, &c.entity, Solidity::SoftAfter(0.12), true));
            }
        }
        for (i, n) in self.npcs.iter().enumerate() {
            // Wanderers are loose creatures who shuffle around — leaning into
            // them shoves them aside. Stationary "rooted" NPCs (Mommy, Sage,
            // shopkeeper, dev knobs) stay solid; pushing them around would feel
            // off-character.
            let solidity = if n.gate {
                // A closed gate guardian fully blocks the chokepoint.
                Solidity::Solid
            } else if n.wanders || n.gate_id.is_some() {
                // Loose wanderers — and guardians who've already stepped aside —
                // yield when leaned on.
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
        // Whoever's tagging along narrates the challenge feedback. Bound up
        // front so it doesn't clash with the mutable borrow of active_challenge.
        let buddy = self.current_buddy_name();
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
                speak_challenge_feedback(&ac.state, &buddy);
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
                    speak_challenge_feedback(&ac.state, &buddy);
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

                // Was this a gate guardian's puzzle? On success the gate opens
                // for good (persisted); on a miss it stays closed and the kid
                // can simply try again — no progress lost.
                if let Some(gid) = self.opening_gate.take() {
                    if was_correct {
                        self.satisfied_gates.insert(gid.clone());
                        if let Some(n) = self.npcs.iter_mut()
                            .find(|n| n.gate_id.map_or(false, |s| s == gid))
                        {
                            n.gate = false;
                        }
                        self.events.push(GameEvent::GateOpened { gate_id: gid });
                    }
                }

                // Was this a fuel-depot refill? On success, top off the tank.
                // A miss just means try again — never a setback.
                if std::mem::take(&mut self.pending_refuel) && was_correct {
                    self.fuel = FUEL_MAX;
                    self.fuel_flash = 0.5;
                    self.events.push(GameEvent::Refueled { to: self.fuel });
                }
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
                audio::tts::speak(&self.current_buddy_name(), &ac.challenge.speech_text);
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
        };
        self.events.push(GameEvent::EncounterTriggered { kind: label.into() });
        match kind {
            EncounterKind::FlavorDialogue { text } => {
                // Whoever's tagging along does the chattering — Sparky or the
                // current NPC buddy.
                self.start_dialogue(vec![DialogueLine {
                    speaker: self.current_buddy_name(),
                    text,
                }]);
                self.set_state(GameState::Dialogue);
            }
            EncounterKind::FoundDumDum => {
                self.dum_dums += 1;
                self.dum_dum_hud.flash();
                self.events.push(GameEvent::DumDumsAwarded { amount: 1 });
                self.start_dialogue(vec![DialogueLine {
                    speaker: self.current_buddy_name(),
                    text: "Ooh! A shiny Dum Dum, just sitting here!".into(),
                }]);
                self.set_state(GameState::Dialogue);
            }
            EncounterKind::Challenge => {
                // A real adaptive challenge, but dressed in scene words so the
                // math reads as part of the world rather than a pop quiz.
                let mut ac = start_challenge(&mut self.rng, &self.profile, self.game_time);
                if let Some(frame) = encounters::frame_sighting(
                    self.map.id,
                    ac.challenge.operation,
                    ac.challenge.numbers.a,
                    ac.challenge.numbers.b,
                    &mut self.rng,
                ) {
                    ac.challenge.display_text = frame.display_text.clone();
                    ac.challenge.speech_text = frame.speech_text.clone();
                    ac.state.question.display = frame.display_text;
                    ac.state.question.speech = frame.speech_text;
                }
                self.events.push(GameEvent::ChallengeStarted {
                    question: ac.challenge.display_text.clone(),
                });
                audio::tts::speak(&self.current_buddy_name(), &ac.challenge.speech_text);
                self.active_challenge = Some(ac);
                self.set_state(GameState::Challenge);
            }
        }
    }

    /// Enter a challenge: the standard multiple-choice quiz. Fires
    /// `ChallengeStarted` so the adaptive system gets its signal.
    fn begin_challenge(&mut self, ac: ActiveChallenge) {
        self.events.push(GameEvent::ChallengeStarted {
            question: ac.challenge.display_text.clone(),
        });
        audio::tts::speak(&self.current_buddy_name(), &ac.challenge.speech_text);
        self.active_challenge = Some(ac);
        self.set_state(GameState::Challenge);
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
                // A normal Choice is made via Choose; an empty (degenerate)
                // Choice falls back to Continue so it can't soft-lock.
                QuestStep::Choice { options, .. } if options.is_empty() => {
                    act = Some(QuestAction::AdvanceStep)
                }
                QuestStep::Choice { .. } => {} // chosen via Choose, not Continue
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
            QuestClick::Choose(index) => {
                if let QuestStep::Choice { .. } = &step {
                    act = Some(QuestAction::ChooseOption { index });
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

                // Same payout rule as every activity: earned only by a clean
                // solve. Violations = mistakes; hints stay reward-neutral
                // (asking for help is a behavior we want, not a grind).
                if let Some(reward) = rewards::determine_reward(was_correct, violations as u32) {
                    self.dum_dums += reward.amount;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: reward.amount });
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

                // `attempts` counts every guess including the right one, so
                // mistakes = attempts - 1. Guess-grinding pays nothing.
                let mistakes = attempts.saturating_sub(1) as u32;
                if let Some(reward) = rewards::determine_reward(was_correct, mistakes) {
                    self.dum_dums += reward.amount;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: reward.amount });
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

                // The balance scale is the grindiest of all — tap every number
                // until it levels. Only a first-guess balance pays.
                let mistakes = attempts.saturating_sub(1) as u32;
                if let Some(reward) = rewards::determine_reward(was_correct, mistakes) {
                    self.dum_dums += reward.amount;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: reward.amount });
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

                if let Some(reward) = rewards::determine_reward(was_correct, violations as u32) {
                    self.dum_dums += reward.amount;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: reward.amount });
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

    /// Launch the number-bond space shooter. Difficulty rides the math band and
    /// the numbers are drawn per the learner's NumberBond CRA stage — both picked
    /// silently, the kid never sees them (Invariant 6).
    fn start_shooter(&mut self, source: String) {
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

    fn step_shooter(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
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
                let response_ms = ((self.game_time - a.start_time) as f64 * 1000.0).min(600000.0);

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
                if let Some(reward) = rewards::determine_reward(true, 0) {
                    self.dum_dums += reward.amount;
                    self.dum_dum_hud.flash();
                    self.events.push(GameEvent::DumDumsAwarded { amount: reward.amount });
                }

                self.events.push(GameEvent::ShooterResolved { waves, hits, misses, response_ms });
            }
            self.set_state(GameState::Playing);

            if self.map.id != "dev" {
                let save_data = self.gather_save_data();
                self.save_backend.save_to(self.active_slot, &save_data);
                self.auto_save_timer = 0.0;
            }
        }
    }

    /// The dive shaft leading down from this map, if it has one. There's no
    /// tile to step on any more — Inkwell is the way down.
    fn dive_portal(&self) -> Option<tilemap::Portal> {
        tilemap::all_portals().iter().copied()
            .find(|p| p.dive && p.from_map == self.map.id)
    }

    /// Open the descent: generate a shaft for the kid's band and hand them the
    /// kicks. Nothing is spent and nothing is lost if they swim back up.
    fn start_descent(&mut self) {
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
    fn step_descent(&mut self, input: &FrameInput, dt: f32, screen: (f32, f32)) {
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
    fn resolve_descent(&mut self) {
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

    /// The "Give Swag" picker. Handing a piece over moves it off the kid, so
    /// the list shrinks as they dress their buddy up — and Bolt is free to
    /// sell them another one of whatever they gave away.
    fn step_swag(&mut self, input: &FrameInput, screen: (f32, f32)) {
        let Some(asw) = self.active_swag.as_ref() else { return };
        let layout = ui::swag::layout(&asw.items, screen);

        let intent = if input.mouse_clicked {
            let (mx, my) = input.mouse_pos;
            ui::swag::handle_click(mx, my, &layout)
        } else {
            ui::swag::handle_key(input, &layout)
        };
        let Some(intent) = intent else { return };

        match intent {
            ui::swag::SwagInput::Close => {
                self.active_swag = None;
                self.set_state(GameState::Playing);
            }
            ui::swag::SwagInput::Give(i) => {
                let Some(item) = asw.items.get(i).cloned() else { return };
                let to = asw.recipient_id.clone();
                let name = asw.recipient_name.clone();
                let outcome = self.wardrobe.hand_over(wardrobe::PLAYER, &to, &item.id);
                let message = match outcome {
                    HandOver::Given => {
                        self.events.push(GameEvent::SwagGiven {
                            item: item.id.clone(),
                            recipient: to.clone(),
                        });
                        audio::tts::speak(&name, &format!("Ooh! A {}! Thank you!", item.name));
                        Some(format!("{name} puts on the {}!", item.name))
                    }
                    // Never a scolding — just a fact about their buddy.
                    HandOver::AlreadyWearing => Some(format!("{name} already has a {}!", item.name)),
                    HandOver::NotWorn => None,
                };
                let remaining = self.swag_catalog_for(wardrobe::PLAYER);
                if let Some(asw) = self.active_swag.as_mut() {
                    asw.items = remaining;
                    asw.message = message;
                }
                if outcome == HandOver::Given && self.map.id != "dev" {
                    let save_data = self.gather_save_data();
                    self.save_backend.save_to(self.active_slot, &save_data);
                    self.auto_save_timer = 0.0;
                }
            }
        }
    }

    /// Catalog entries for everything `who` is wearing, in catalog order so the
    /// picker rows are stable between openings.
    fn swag_catalog_for(&self, who: &str) -> Vec<ShopItem> {
        let worn = self.wardrobe.worn_by(who);
        shop::shop_catalog().into_iter().filter(|i| worn.contains(&i.id)).collect()
    }

    fn step_shop(&mut self, input: &FrameInput, screen: (f32, f32)) {
        let Some(ash) = self.active_shop.as_ref() else { return };
        let view = shop_view(ash, &self.color_choice);
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
                // "Done" dismisses the nearest thing: the color picker if it's
                // up, otherwise the whole shop.
                let ash = self.active_shop.as_mut().unwrap();
                if ash.picking_color {
                    ash.picking_color = false;
                    ash.message = None;
                    return;
                }
                if let Some(ash) = self.active_shop.take() {
                    // Only the wearable half of `owned` belongs in the
                    // wardrobe; upgrades were banked when they were bought.
                    let swag: Vec<String> = ash.owned.into_iter()
                        .filter(|id| !self.upgrades.contains(id))
                        .collect();
                    self.wardrobe.set_worn(wardrobe::PLAYER, swag);
                }
                self.set_state(GameState::Playing);
            }
            ui::shop::ShopInput::SelectItem(i) => {
                // Balances read before the session borrow so the purchase
                // branch below can use them without fighting the borrowck.
                let purse = {
                    let shop = self.active_shop.as_ref().unwrap().shop;
                    self.balance_for(shop.currency())
                };
                let pearls = self.pearls;
                let ash = self.active_shop.as_mut().unwrap();
                if ash.selected.is_some() || ash.picking_color {
                    return; // already solving a purchase or picking a color
                }
                let item = ash.catalog[i].clone();
                // An owned Color Change re-opens the picker — buying it once
                // means you get to change colors whenever you like.
                if item.id == "color_change" && ash.owned.contains(&item.id) {
                    ash.picking_color = true;
                    ash.message = None;
                    return;
                }
                // The trade desk isn't a purchase, it's a conversion: hand
                // over the pile and work out what it's worth.
                if let ItemKind::Trade { rate } = item.kind {
                    let quote = shop::quote_trade(pearls, rate);
                    if quote.gain == 0 {
                        let need = rate - pearls;
                        ash.message = Some(format!(
                            "Not enough for a Dum Dum yet — you need {need} more pearls!",
                        ));
                        return;
                    }
                    ash.selected = Some(i);
                    ash.answer = quote.gain;
                    ash.message = None;
                    ash.choices = division_choices(quote.gain, quote.offered, &mut self.rng);
                    ash.trading = Some(quote);
                    return;
                }
                let balance = purse;
                match shop::process_purchase(balance, &item.id, &ash.owned) {
                    shop::PurchaseOutcome::Bought { result } => {
                        ash.selected = Some(i);
                        ash.cost = result.spent;
                        ash.answer = result.new_balance;
                        ash.balance_before = balance;
                        ash.message = None;
                        let choices = subtraction_choices(balance, result.spent, &mut self.rng);
                        ash.choices = choices;
                    }
                    shop::PurchaseOutcome::CantAfford { shortfall } => {
                        ash.message = Some(format!(
                            "You need {shortfall} more {}!", item.currency.label(),
                        ));
                    }
                    shop::PurchaseOutcome::AlreadyOwned => {
                        // You can only wear one of each — but give it to a
                        // buddy and Bolt will happily sell you another.
                        ash.message = Some("You're already wearing that one!".into());
                    }
                    shop::PurchaseOutcome::UnknownItem => {}
                }
            }
            ui::shop::ShopInput::Answer(v) => {
                // Resolve the guess on the shop session, then drop that borrow
                // before touching `self` (balances, events, save).
                enum Settled {
                    Bought { item: ShopItem, spent: u32, left: u32 },
                    Traded(shop::TradeQuote),
                }
                let settled = {
                    let ash = self.active_shop.as_mut().unwrap();
                    let Some(i) = ash.selected else { return };
                    if v != ash.answer {
                        // Natural consequence, not punishment — recount and retry.
                        ash.message = Some("Hmm, let me count again...".into());
                        None
                    } else if let Some(quote) = ash.trading.take() {
                        ash.selected = None;
                        ash.choices.clear();
                        ash.message = Some(if quote.left_over > 0 {
                            format!(
                                "{} Dum Dums, and {} pearls back in your pocket!",
                                quote.gain, quote.left_over,
                            )
                        } else {
                            format!("{} Dum Dums, spot on!", quote.gain)
                        });
                        Some(Settled::Traded(quote))
                    } else {
                        let item = ash.catalog[i].clone();
                        ash.owned.insert(item.id.clone());
                        ash.selected = None;
                        ash.choices.clear();
                        if item.id == "color_change" {
                            // The fun part of Color Change is choosing — go
                            // straight to the swatches.
                            ash.picking_color = true;
                            ash.message = Some("You got it! Pick your color!".into());
                        } else if matches!(item.kind, ItemKind::Upgrade) {
                            // Say what it DOES, not just that it's bought — a
                            // perk with no visible effect is a mystery.
                            ash.message = Some(if item.blurb.is_empty() {
                                format!("The {} is yours for keeps!", item.name)
                            } else {
                                format!("{} — {}!", item.name, item.blurb)
                            });
                        } else {
                            ash.message = Some(format!("You look GREAT in the {}!", item.name));
                        }
                        let spent = ash.cost;
                        let left = ash.answer;
                        Some(Settled::Bought { item, spent, left })
                    }
                };

                let Some(settled) = settled else { return };
                match settled {
                    Settled::Bought { item, spent, left } => {
                        match item.currency {
                            Currency::DumDums => {
                                self.dum_dums = left;
                                self.dum_dum_hud.flash();
                                self.events.push(GameEvent::DumDumsSpent {
                                    amount: spent, item: item.id.clone(),
                                });
                            }
                            Currency::Pearls => {
                                self.pearls = left;
                                self.pearl_hud.flash();
                                self.events.push(GameEvent::PearlsSpent {
                                    amount: spent, item: item.id.clone(),
                                });
                            }
                        }
                        // Upgrades are banked here rather than worn — they're
                        // perks, not outfits, and can't be handed to a buddy.
                        if matches!(item.kind, ItemKind::Upgrade) {
                            self.upgrades.insert(item.id.clone());
                            if let Some(ash) = self.active_shop.as_mut() {
                                ash.owned.insert(item.id.clone());
                            }
                        }
                    }
                    Settled::Traded(quote) => {
                        self.pearls = self.pearls.saturating_sub(quote.spent);
                        self.dum_dums = self.dum_dums.saturating_add(quote.gain);
                        self.pearl_hud.flash();
                        self.dum_dum_hud.flash();
                        self.events.push(GameEvent::PearlsTraded {
                            pearls: quote.spent,
                            dum_dums: quote.gain,
                            left_over: quote.left_over,
                        });
                    }
                }

                // Persist immediately so the purchase (and the spent currency)
                // survive a reload even if the kid quits right now.
                if let Some(ash) = self.active_shop.as_ref() {
                    let swag: Vec<String> = ash.owned.iter()
                        .filter(|id| !self.upgrades.contains(*id))
                        .cloned()
                        .collect();
                    self.wardrobe.set_worn(wardrobe::PLAYER, swag);
                }
                if self.map.id != "dev" {
                    let save_data = self.gather_save_data();
                    self.save_backend.save_to(self.active_slot, &save_data);
                }
            }
            ui::shop::ShopInput::PickColor(i) => {
                let Some((id, _)) = sprites::player::OUTFIT_COLORS.get(i) else { return };
                self.color_choice = id.to_string();
                let ash = self.active_shop.as_mut().unwrap();
                ash.message = Some("Looking good!".into());
                // Persist right away, same as a purchase — the new outfit
                // should survive a reload even if the kid quits now.
                if self.map.id != "dev" {
                    let save_data = self.gather_save_data();
                    self.save_backend.save_to(self.active_slot, &save_data);
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
                        // The companion is checked too: a mount you're riding
                        // isn't in the roster, but it's still who you're talking to.
                        let lines = self.npcs.iter().chain(self.companion.iter())
                            .find(|n| n.id_str() == self.menu_target_id)
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
                "dive" => {
                    self.start_descent();
                }
                "shop" => {
                    let source = self.menu_target_id.clone();
                    let shop = if self.menu_target_id == "hermit_crab" {
                        ShopKind::Hermie
                    } else {
                        ShopKind::Bolt
                    };
                    self.active_shop = Some(ActiveShop {
                        shop,
                        catalog: shop.catalog(),
                        owned: self.shop_owned_for(shop),
                        selected: None,
                        choices: Vec::new(),
                        answer: 0,
                        cost: 0,
                        balance_before: 0,
                        message: None,
                        source_npc: source,
                        picking_color: false,
                        trading: None,
                    });
                    self.set_state(GameState::Shop);
                }
                "swag" => {
                    // Whoever's in front of the kid gets dressed up. Sparky is
                    // a robot rather than a roster NPC, hence the sprite-less
                    // preview; everyone else previews as themselves.
                    let sprite = self.npcs.iter()
                        .chain(self.companion.iter())
                        .find(|n| n.id_str() == self.menu_target_id)
                        .map(|n| n.sprite);
                    self.active_swag = Some(ActiveSwag {
                        recipient_id: self.menu_target_id.clone(),
                        recipient_name: self.menu_target_name.clone(),
                        recipient_sprite: sprite,
                        items: self.swag_catalog_for(wardrobe::PLAYER),
                        message: None,
                    });
                    self.set_state(GameState::Swag);
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
                        Feature::Quest => self.features.quest = !self.features.quest,
                    },
                    // Mouse-reachable session export (parent dashboard). Same
                    // payload as the debug overlay's Export button.
                    // Parent dial: slow the arcade down (or speed it up) and
                    // persist it, without touching which numbers get asked.
                    SettingsResult::SetPace(pace) => {
                        self.game_pace = pace;
                        if self.map.id != "dev" {
                            let save_data = self.gather_save_data();
                            self.save_backend.save_to(self.active_slot, &save_data);
                        }
                    }
                    SettingsResult::ExportSession => {
                        let json = session::build_export(
                            &self.player_name, &self.session_log, &self.gifts_given,
                            self.dum_dums, self.play_time, &self.profile, self.map.id,
                        );
                        let filename = format!("robot-buddy-session-{}.json", self.play_time as u64);
                        session::download_json(&json, &filename);
                    }
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
                        self.active_swag = None;
                        self.active_descent = None;
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
        !self.sparky_parked || self.map.id == self.sparky_map
    }

    /// What a counter treats as already-bought: swag is "what the kid is
    /// wearing" (hand it to a buddy and it's for sale again), upgrades are
    /// "what they've bought" (permanent). Hermie sells both, so his shelf
    /// checks the union.
    fn shop_owned_for(&self, shop: ShopKind) -> std::collections::BTreeSet<String> {
        let mut owned = self.player_swag().clone();
        if shop == ShopKind::Hermie {
            owned.extend(self.upgrades.iter().cloned());
        }
        owned
    }

    /// The purse a counter spends from.
    fn balance_for(&self, currency: Currency) -> u32 {
        match currency {
            Currency::DumDums => self.dum_dums,
            Currency::Pearls => self.pearls,
        }
    }

    /// The permanent perks the kid is carrying — drawn on them, but never in
    /// the wardrobe, so they can't be handed to a buddy.
    pub fn gear_worn(&self) -> &std::collections::BTreeSet<String> {
        &self.upgrades
    }

    /// The floating cheer currently on screen, if any. Lets tests read the
    /// feedback a kid would actually see.
    pub fn track_toast_text(&self) -> Option<&str> {
        self.track_toast.as_ref().map(|(msg, _)| msg.as_str())
    }

    /// True once the kid owns Hermie's Diving Net, which pays a bonus pearl on
    /// every find from then on — the grind rewarding the grind.
    pub fn has_diving_net(&self) -> bool {
        self.upgrades.contains(shop::DIVING_NET)
    }

    /// Everything the kid is wearing right now. Swag they've handed to a
    /// buddy isn't in here any more — that's the whole point.
    pub fn player_swag(&self) -> &std::collections::BTreeSet<String> {
        self.wardrobe.worn_by(wardrobe::PLAYER)
    }

    /// What `who` (an NPC id, `"sparky"`, or `wardrobe::PLAYER`) is wearing.
    pub fn swag_worn_by(&self, who: &str) -> &std::collections::BTreeSet<String> {
        self.wardrobe.worn_by(who)
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

    /// Display name of whoever's currently tagging along — the active NPC
    /// companion if there is one, else Sparky. This is the voice that narrates
    /// flavor text (TTS) and side-chatter as the player explores, so it must
    /// track the real buddy rather than assuming Sparky.
    pub fn current_buddy_name(&self) -> String {
        match self.companion.as_ref() {
            Some(c) => c.name().to_string(),
            None => "Sparky".to_string(),
        }
    }

    /// Tile + direction for parked Sparky's resting spot. Sparky faces the
    /// player's typical entry direction (Down — toward the path) so the kid
    /// runs into him head-on when arriving at the overworld.
    fn park_sparky(&mut self) {
        self.sparky_parked = true;
        self.sparky_map = SPARKY_HOME_MAP;
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
            // Drop only the companion's OWN home-roster entry so they don't also
            // appear back home. A same-kind NPC that lives on a *different* map
            // (e.g. the reef's Shelly vs. the trench's Shelly, both `Clam`) is a
            // different creature and must stay — matching on kind alone made
            // recruiting one erase the other.
            let (kind, home) = (c.kind, c.home_map);
            roster.retain(|n| !(n.kind == kind && n.home_map == home));
        }
        // A gate the kid already solved stays open: clear the guardian's `gate`
        // flag so it's pushable and won't re-pose its puzzle.
        for n in roster.iter_mut() {
            if let Some(id) = n.gate_id {
                if self.satisfied_gates.contains(id) {
                    n.gate = false;
                }
            }
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
            old.stop_following();
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
        leaving.stop_following();
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
            // Same map: the buddy strolls back to its spot from wherever it was
            // tagging along, rather than blinking out of existence. Route is a
            // static-terrain BFS; the resolver handles any live entities in the
            // way (it just waits). If home is somehow unreachable, fall back to
            // the old snap so the NPC can never get lost.
            let map = &self.map;
            let cur = (npc.entity.tile_x, npc.entity.tile_y);
            let route = crate::pathfinding::find_path(
                cur, (hx, hy), map.width, map.height,
                |cx, cy| !map.is_solid(cx, cy),
            );
            match route {
                Some(path) => {
                    // start_homing tolerates an empty path (already home).
                    npc.entity.moving = false;
                    npc.start_homing(path);
                    self.npcs.push(npc);
                }
                None => {
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
                    npc.reset_to_home();
                    npc.entity.tile_x = nx;
                    npc.entity.tile_y = ny;
                    npc.entity.x = nx as f32 * TILE_SIZE;
                    npc.entity.y = ny as f32 * TILE_SIZE;
                    npc.entity.target_x = npc.entity.x;
                    npc.entity.target_y = npc.entity.y;
                    self.npcs.push(npc);
                }
            }
        } else {
            // Home is on another map: rather than blinking away on the spot, the
            // buddy heads for the nearest exit and only teleports home once it's
            // walked off-screen (or reached the doorway). Pick a door that leads
            // toward home — its own map first, then the overworld hub, then any
            // reachable exit — and route there.
            let cur = (npc.entity.tile_x, npc.entity.tile_y);
            let route = {
                let map = &self.map;
                let mut exits: Vec<&tilemap::Portal> = tilemap::all_portals().iter()
                    .filter(|p| p.from_map == map.id)
                    .collect();
                exits.sort_by_key(|p| {
                    let rank = if p.to_map == home_map { 0 }
                        else if p.to_map == "overworld" { 1 }
                        else { 2 };
                    let dist = (p.from_x as i32 - cur.0 as i32).abs()
                        + (p.from_y as i32 - cur.1 as i32).abs();
                    (rank, dist)
                });
                exits.iter().find_map(|p| {
                    crate::pathfinding::find_path(
                        cur, (p.from_x, p.from_y), map.width, map.height,
                        |cx, cy| !map.is_solid(cx, cy),
                    ).filter(|path| !path.is_empty())
                })
            };
            match route {
                Some(path) => {
                    npc.entity.moving = false;
                    npc.start_homing(path);
                    npc.leaving_map = true;
                    self.npcs.push(npc);
                }
                None => {
                    // Already at the door, or no reachable exit — snap straight
                    // to the offstage roster so the buddy can never get lost.
                    npc.reset_to_home();
                    self.npcs_offstage
                        .entry(home_map.to_string())
                        .or_insert_with(Vec::new)
                        .push(npc);
                }
            }
        }
    }

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
    fn check_number_track_landing(&mut self, _dt: f32) {
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
    fn handle_leap_input(&mut self, input: &FrameInput, screen: (f32, f32)) -> bool {
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
                let payout = track.payout
                    + if after.was_clean() { 1 } else { 0 }
                    + if self.has_diving_net() { shop::DIVING_NET_BONUS } else { 0 };
                self.pearls = self.pearls.saturating_add(payout);
                self.pearl_hud.flash();
                self.events.push(GameEvent::PearlFound {
                    stone: after.puzzle.pearl,
                    size: after.puzzle.size,
                    leaps: after.leaps,
                    resets: after.resets,
                    pearls: payout,
                });
                let mut cheer = if after.was_clean() {
                    format!("Right on it! The pearl was under stone {}!  +{payout} pearls", after.puzzle.pearl)
                } else {
                    format!("You found it! Stone {}.  +{payout} pearl", after.puzzle.pearl)
                };
                // Name the net every time it pays, so the kid can see the
                // twenty pearls still working for them.
                if self.has_diving_net() {
                    cheer.push_str("  (your net caught one!)");
                }
                audio::tts::speak("Shelly", "You found my pearl!");
                self.track_toast = Some((cheer, 2.4));
                // Shelly hides it again — a fresh trip next time they launch.
                self.leap_session = None;
                if self.map.id != "dev" {
                    let save_data = self.gather_save_data();
                    self.save_backend.save_to(self.active_slot, &save_data);
                }
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

    /// Whisk any swapped-out buddy that's `leaving_map` off to its real home the
    /// moment it walks off-screen or reaches the exit doorway. Until then it's a
    /// normal roster NPC strolling toward the door. Keeps the "walk out, then
    /// teleport" illusion without ever stranding a buddy on the wrong map.
    fn evict_offscreen_leavers(&mut self, screen: (f32, f32)) {
        let view = visible_world_rect((self.camera.x, self.camera.y), screen);
        let gone: Vec<usize> = self.npcs.iter().enumerate()
            .filter(|(_, n)| n.leaving_map && (!npc_in_camera(view, n) || !n.homing))
            .map(|(i, _)| i)
            .collect();
        for i in gone.into_iter().rev() {
            let mut n = self.npcs.remove(i);
            n.leaving_map = false;
            let home_map = n.home_map;
            n.reset_to_home();
            self.npcs_offstage
                .entry(home_map.to_string())
                .or_insert_with(Vec::new)
                .push(n);
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
            // A buddy walking out to leave the map is owned by the leaver-evict
            // path (it teleports to its real home, not through this door).
            if self.npcs[i].leaving_map { continue; }
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

    /// Parked Sparky stepped (or got pushed) onto a portal tile — carry him
    /// through it just like a roster wanderer. Updates `sparky_map` to wherever
    /// he lands; he won't move again until the player is on that map (only then
    /// does his wander roll run). Re-anchoring his wander tether isn't needed:
    /// off his home map he just waits to be found or re-recruited.
    fn handle_parked_sparky_portal(&mut self) {
        let (tx, ty) = (self.sparky.entity.tile_x, self.sparky.entity.tile_y);
        let portal = match tilemap::check_portal(self.sparky_map, tx, ty) {
            Some(p) => p,
            None => return,
        };
        let dest_map = portal.to_map;
        let dest_geometry = Map::by_id(dest_map);
        let player_on_dest = dest_map == self.map.id;
        let player = (self.player.tile_x, self.player.tile_y);
        let empty: Vec<npc::Npc> = Vec::new();
        let occupants = self.npcs_offstage.get(dest_map).unwrap_or(&empty);
        let occ_tiles: Vec<(usize, usize)> = occupants.iter()
            .map(|n| (n.entity.tile_x, n.entity.tile_y))
            .collect();
        let roster_tiles: Vec<(usize, usize)> = if player_on_dest {
            self.npcs.iter().map(|n| (n.entity.tile_x, n.entity.tile_y)).collect()
        } else {
            Vec::new()
        };
        let (dx, dy) = npc::find_npc_spawn_spot(
            portal.to_x, portal.to_y, dest_geometry.width, dest_geometry.height,
            |cx, cy| dest_geometry.is_solid(cx, cy),
            |cx, cy| (player_on_dest && (cx, cy) == player)
                || occ_tiles.iter().any(|t| *t == (cx, cy))
                || roster_tiles.iter().any(|t| *t == (cx, cy)),
        );
        self.sparky_map = dest_map;
        let e = &mut self.sparky.entity;
        e.tile_x = dx;
        e.tile_y = dy;
        e.x = dx as f32 * TILE_SIZE;
        e.y = dy as f32 * TILE_SIZE;
        e.target_x = e.x;
        e.target_y = e.y;
        e.moving = false;
        self.sparky_wander_cooldown = npc::WANDER_COOLDOWN_MIN;
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
            // Seed a never-visited destination's stash with its REAL roster
            // before adding the intruder. Otherwise the stash would hold only
            // the pushed NPC, and `load_map_roster` (which prefers an existing
            // stash over `npcs_for_map`) would spawn the map with its regular
            // residents missing.
            self.npcs_offstage
                .entry(dest_map.to_string())
                .or_insert_with(|| npc::npcs_for_map(dest_map))
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
        self.take_portal(*portal);
    }

    /// Travel through `portal`: tolls, fuel, the transfer itself, and the
    /// arrival beat. Split out from `handle_portal` because a dive ends
    /// somewhere the kid isn't standing — Inkwell sends them down the shaft
    /// from her ledge, so there's no tile underfoot to look the portal up from.
    fn take_portal(&mut self, portal: tilemap::Portal) {
        let secret = portal.secret;
        let mut dest_map = portal.to_map;
        let dest_x = portal.to_x;
        let dest_y = portal.to_y;
        let cost = portal.cost;
        let fuel_cost = portal.fuel_cost;
        let from_map = self.map.id.to_string();

        // Rocket fuel: a fuel-costed jump won't fire on an empty tank. Never a
        // punishment — Sparky points at the fuel droid; no transfer happens.
        if fuel_cost > 0 && self.fuel < fuel_cost {
            self.start_dialogue(vec![DialogueLine {
                speaker: self.current_buddy_name(),
                text: format!(
                    "Not enough fuel for that jump, boss! It needs {fuel_cost} and we've got {}. Let's find Tank the fuel droid and do some math to top up!",
                    self.fuel
                ),
            }]);
            self.set_state(GameState::Dialogue);
            return;
        }
        if fuel_cost > 0 {
            self.fuel -= fuel_cost;
            self.fuel_flash = 0.5;
            self.events.push(GameEvent::FuelSpent { amount: fuel_cost, remaining: self.fuel });
        }

        // One-time toll gate (reusable): a priced portal charges once per
        // destination, then it's unlocked for good. Falling short is never a
        // punishment — Sparky just cheers them on to go collect more, and no
        // transfer happens (they stay put). Keyed by destination map id.
        let toll_id = dest_map.to_string();
        let toll_due = cost > 0 && !self.paid_tolls.contains(&toll_id);
        if toll_due && self.dum_dums < cost {
            let need = cost - self.dum_dums;
            self.start_dialogue(vec![DialogueLine {
                speaker: self.current_buddy_name(),
                text: format!(
                    "Ooh, a dive spot! The first splash in costs {cost} Dum Dums. We need {need} more — let's go find some, boss!"
                ),
            }]);
            self.set_state(GameState::Dialogue);
            return;
        }
        if toll_due {
            self.dum_dums -= cost;
            self.paid_tolls.insert(toll_id);
            self.dum_dum_hud.flash();
            self.events.push(GameEvent::DumDumsSpent {
                amount: cost,
                item: format!("dive:{dest_map}"),
            });
        }

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

        // Reset the ambient pearl to the new map's path start (if any).
        // A pearl trip belongs to the map it started on.
        self.leap_session = None;

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

        // The arrival cutscene is a first-time-only thrill. Once a map's intro
        // has played it's remembered (and persisted), so a kid who dives the
        // reef every session doesn't sit through the same speech every time.
        if secret && self.seen_intros.insert(self.map.id.to_string()) {
            let lines = secret_entry_dialogue(self.map.id, &self.current_buddy_name());
            if !lines.is_empty() {
                self.start_dialogue(lines);
                self.set_state(GameState::Dialogue);
            }
        }
    }

    // ─── Rendering ─────────────────────────────────────

    /// Paint whatever `who` is wearing over the sprite just drawn for them.
    /// No-op for anyone who's been given nothing, which is almost everyone.
    fn draw_swag_on(&self, who: &str, x: f32, y: f32, dir: Dir, fit: sprites::swag::SwagFit) {
        let worn = self.wardrobe.worn_by(who);
        if worn.is_empty() { return; }
        sprites::swag::draw_swag(x, y, dir, 0.0, worn, &self.color_choice, fit);
    }

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
                    // Length is adaptive, so don't promise a fixed total.
                    let progress_text = format!("Question {}", iq.question_index + 1);
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
            // Draw the tiles the camera actually shows — the whole window, not
            // just the logical 960×720 frame. This is the structural guarantee
            // that nothing world-space (stones, sprites, markers) can ever be
            // visible over undrawn void: wherever the camera looks, tiles are.
            let view = visible_world_rect((self.camera.x, self.camera.y), (sw, sh));
            tilemap::draw_map(&self.map, view.x, view.y, view.w, view.h, self.game_time);

            // Embodied number line: stepping-stones drawn on the ground (under
            // the sprites) so the kid hops across the numbers.
            if let Some(track) = number_track::track_for_map(self.map.id) {
                let here = track.index_of((self.player.tile_x, self.player.tile_y));
                draw_number_track(&track, here, self.leap_session.as_ref(), self.game_time);
            }

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

            // `Mount` is the rideable *companion* only — never a roster NPC, so a
            // wild/gate shark that happens to be rideable still draws normally at
            // its own tile instead of teleporting under the player.
            enum SpriteKind<'a> { Player, Sparky, Npc(&'a npc::Npc), Mount(&'a npc::Npc) }
            struct Renderable<'a> { y: f32, kind: SpriteKind<'a> }
            let mut renderables: Vec<Renderable> = vec![];

            renderables.push(Renderable { y: self.player.y, kind: SpriteKind::Player });
            if self.sparky_is_here() {
                renderables.push(Renderable { y: self.sparky.entity.y, kind: SpriteKind::Sparky });
            }
            if let Some(c) = self.companion.as_ref() {
                if c.is_rideable() {
                    // A rideable buddy (Chompy) sits on the player's tile as a
                    // mount — nudge its sort key just behind the player so the kid
                    // always draws on top, looking like they're riding it.
                    renderables.push(Renderable { y: self.player.y - 1.0, kind: SpriteKind::Mount(c) });
                } else {
                    renderables.push(Renderable { y: c.entity.y, kind: SpriteKind::Npc(c) });
                }
            }
            // Skip roster NPCs outside the visible rect — pure draw-call
            // thrift; anywhere visible has tiles under it now.
            for n in &self.npcs {
                if npc_in_camera(view, n) {
                    renderables.push(Renderable { y: n.entity.y, kind: SpriteKind::Npc(n) });
                }
            }
            renderables.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());

            for r in &renderables {
                match &r.kind {
                    SpriteKind::Player => {
                        if self.map.id == "space_hub" {
                            // On the hub the kid pilots the rocket — that's the avatar.
                            sprites::player::draw_rocket(self.player.x, self.player.y, self.player.dir, self.player.frame, self.game_time);
                        } else {
                            // When riding a mount (Chompy, Echo), lift the kid onto
                            // its back and ride its own swim-bob so the two move
                            // as one body.
                            let py = match self.companion.as_ref() {
                                Some(c) if c.is_rideable() =>
                                    self.player.y + c.rider_offset(self.game_time),
                                _ => self.player.y,
                            };
                            match self.player_gender {
                                Gender::Boy => sprites::player::draw_player_boy(self.player.x, py, self.player.dir, self.player.frame, self.game_time),
                                Gender::Girl => sprites::player::draw_player_girl(self.player.x, py, self.player.dir, self.player.frame, self.game_time),
                            }
                            // Cosmetics bought from Bolt's shop ride on the kid.
                            sprites::player::draw_player_cosmetics(self.player.x, py, self.player.dir, self.player.frame, self.player_swag(), &self.color_choice);
                            // Perks from Hermie's stall ride along too — they're
                            // not wearable swag, but the kid should be able to
                            // SEE what twenty pearls bought them.
                            let bob = if self.player.frame % 2 == 1 { -2.0 } else { 0.0 };
                            sprites::swag::draw_gear(self.player.x, py, self.player.dir, bob,
                                &self.upgrades, sprites::swag::SwagFit::KID);
                            // On planet surfaces the kid wears a space helmet.
                            if self.map.render_mode == tilemap::RenderMode::Cosmic {
                                sprites::player::draw_spacesuit_overlay(self.player.x, py, self.player.frame);
                            }
                        }
                    }
                    SpriteKind::Sparky => {
                        let e = &self.sparky.entity;
                        sprites::robot::draw_robot(e.x, e.y, e.dir, e.frame, self.game_time);
                        self.draw_swag_on("sparky", e.x, e.y, e.dir,
                            sprites::swag::SwagFit::ROBOT);
                    }
                    SpriteKind::Npc(n) => {
                        n.draw(self.game_time);
                        self.draw_swag_on(n.id_str(), n.entity.x, n.entity.y, n.entity.dir,
                            n.sprite.swag_fit());
                    }
                    SpriteKind::Mount(n) => {
                        // The mount is pinned under its rider: draw its own
                        // sprite at the player's tile, facing the player's way,
                        // so the kid sits astride its back rather than
                        // alongside a blob.
                        n.draw_at(self.player.x, self.player.y, self.player.dir, self.game_time);
                        self.draw_swag_on(n.id_str(), self.player.x, self.player.y,
                            self.player.dir, n.sprite.swag_fit());
                    }
                }
            }

            // Price tags over the hub's planet pads so the jump cost is visible
            // up front (not a surprise only when a jump is refused).
            if self.map.id == "space_hub" {
                self.draw_planet_pad_labels();
            }

            set_default_camera();
        }
    }

    /// Draw a floating price tag above each planet pad on the hub: the planet's
    /// name and what a jump there costs — green "FREE", amber when affordable,
    /// red when the tank's too low. Reads the live fuel + portal table so it
    /// always matches what `handle_portal` will actually charge.
    fn draw_planet_pad_labels(&self) {
        for p in tilemap::all_portals() {
            if p.from_map != self.map.id { continue; }
            let tile = self.map.tiles[p.from_y][p.from_x];
            if !matches!(tile, tilemap::Tile::MoonPad | tilemap::Tile::MarsPad | tilemap::Tile::AsteroidPad) {
                continue;
            }
            let name = ui::hud::get_area_name(p.to_map, 0, 0);
            let cx = p.from_x as f32 * TILE_SIZE + TILE_SIZE / 2.0;

            let nw = measure_text(name, None, 18, 1.0).width;
            let free = p.fuel_cost == 0;
            let cost_str = if free { "FREE".to_string() } else { format!("{}", p.fuel_cost) };
            let cw = measure_text(&cost_str, None, 18, 1.0).width;
            let cost_line_w = if free { cw } else { cw + 16.0 }; // droplet + number
            let pill_w = nw.max(cost_line_w) + 18.0;
            let pill_h = 40.0;
            let px = cx - pill_w / 2.0;
            let py = p.from_y as f32 * TILE_SIZE - pill_h - 2.0;

            draw_rectangle(px, py, pill_w, pill_h, Color::new(0.04, 0.05, 0.12, 0.88));
            draw_rectangle_lines(px, py, pill_w, pill_h, 1.5, Color::new(0.45, 0.55, 0.85, 0.7));
            draw_text(name, cx - nw / 2.0, py + 17.0, 18.0, WHITE);

            let color = if free {
                Color::from_rgba(102, 220, 120, 255)   // green: free hop
            } else if self.fuel >= p.fuel_cost {
                Color::from_rgba(255, 193, 7, 255)      // amber: affordable
            } else {
                Color::from_rgba(255, 99, 99, 255)      // red: not enough fuel
            };
            if free {
                draw_text(&cost_str, cx - cw / 2.0, py + 34.0, 18.0, color);
            } else {
                // A little fuel droplet, then the number — readable for pre-readers.
                let group_x = cx - cost_line_w / 2.0;
                let dx = group_x + 6.0;
                let dy = py + 28.0;
                draw_circle(dx, dy + 2.0, 4.0, color);
                draw_triangle(
                    vec2(dx - 3.5, dy + 1.0),
                    vec2(dx + 3.5, dy + 1.0),
                    vec2(dx, dy - 5.0),
                    color,
                );
                draw_text(&cost_str, group_x + 14.0, py + 34.0, 18.0, color);
            }
        }
    }

    fn render_hud(&mut self, screen: (f32, f32)) {
        ui::hud::draw_area_name(self.map.id, self.player.tile_x, self.player.tile_y);
        self.dum_dum_hud.draw(self.dum_dums, screen);
        // Pearl counter — only once the kid has earned any (reef-local currency).
        if self.pearls > 0 {
            self.pearl_hud.draw(self.pearls, screen);
        }
        // A brief floating cheer after hopping to the pearl.
        if let Some((ref msg, _)) = self.track_toast {
            let (sw, _) = screen;
            let tw = measure_text(msg, None, 26, 1.0).width;
            draw_text(msg, sw / 2.0 - tw / 2.0, 92.0, 26.0, Color::from_rgba(178, 235, 242, 255));
        }
        // Rocket fuel gauge — only relevant (and only shown) in space.
        if self.map.render_mode == tilemap::RenderMode::Cosmic {
            ui::hud::draw_fuel_gauge(self.fuel, FUEL_MAX, self.fuel_flash, screen);
        }
        // On-screen settings gear (tap to open settings → parent options),
        // shown during free play so it's reachable without a keyboard.
        if self.state == GameState::Playing && !self.settings_open {
            let (gx, gy, gw, gh) = settings_gear_rect(screen);
            let (cx, cy) = (gx + gw / 2.0, gy + gh / 2.0);
            draw_rectangle(gx, gy, gw, gh, Color::new(0.078, 0.078, 0.157, 0.8));
            draw_rectangle_lines(gx, gy, gw, gh, 2.0, Color::new(1.0, 0.835, 0.310, 0.9));
            // A simple cog: ring of teeth + body + hub hole.
            let gold = Color::new(1.0, 0.835, 0.310, 1.0);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::PI / 4.0;
                draw_circle(cx + a.cos() * 13.0, cy + a.sin() * 13.0, 3.0, gold);
            }
            draw_circle(cx, cy, 11.0, gold);
            draw_circle(cx, cy, 4.5, Color::new(0.078, 0.078, 0.157, 1.0));
        }
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

        // Goyish Map shooter — a full-screen minigame.
        if let Some(ref a) = self.active_shooter {
            ui::shooter::draw(&a.session, screen, self.game_time);
        }

        // Shop overlay
        if let Some(ref ash) = self.active_shop {
            let view = shop_view(ash, &self.color_choice);
            let layout = ui::shop::layout(&ash.catalog, &view, screen);
            let balance = self.balance_for(ash.shop.currency());
            ui::shop::draw_shop(&ash.catalog, &ash.owned, balance, &view, &layout,
                ash.message.as_deref(), ash.shop);

            // While picking an outfit color, show a live preview of the kid in
            // the panel's top-right so tapping swatches visibly recolors them.
            if ash.picking_color {
                let px = layout.panel.x + layout.panel.w - 64.0;
                let py = layout.panel.y + 14.0;
                match self.player_gender {
                    Gender::Boy => sprites::player::draw_player_boy(px, py, Dir::Down, 0, self.game_time),
                    Gender::Girl => sprites::player::draw_player_girl(px, py, Dir::Down, 0, self.game_time),
                }
                sprites::player::draw_player_cosmetics(px, py, Dir::Down, 0, &ash.owned, &self.color_choice);
            }
        }

        // Shelly's leap panel — up whenever a pearl trip is going, so the call
        // and the sizes on offer are always on screen rather than in a toast.
        if let Some(ref s) = self.leap_session {
            let layout = ui::leap::layout(s, screen);
            ui::leap::draw(s, &layout, &leap_call(&s.puzzle), input.mouse_pos);
        }

        // Descent overlay
        if let Some(ref ad) = self.active_descent {
            let layout = ui::descent::layout(&ad.session, screen);
            ui::descent::draw(&ad.session, &layout, ad.message.as_deref(), self.game_time);
        }

        // Give-Swag overlay
        if let Some(ref asw) = self.active_swag {
            let layout = ui::swag::layout(&asw.items, screen);
            let taken = self.wardrobe.worn_by(&asw.recipient_id);
            ui::swag::draw(&asw.recipient_name, &asw.items, taken, &layout, asw.message.as_deref());
            // Live preview of the buddy in their current outfit, so handing
            // something over visibly lands on them.
            let (px, py) = layout.preview;
            match asw.recipient_sprite {
                Some(sprite) => {
                    sprite.draw_sprite(px, py, Dir::Down, self.game_time, false);
                    sprites::swag::draw_swag(px, py, Dir::Down, 0.0, taken,
                        &self.color_choice, sprite.swag_fit());
                }
                None => {
                    sprites::robot::draw_robot(px, py, Dir::Down, 0, self.game_time);
                    sprites::swag::draw_swag(px, py, Dir::Down, 0.0, taken,
                        &self.color_choice, sprites::swag::SwagFit::ROBOT);
                }
            }
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
            ui::settings_overlay::draw(screen, self.features, self.parent_panel_open, self.game_pace);
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
            pearls: self.pearls,
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
            shop_owned: Vec::new(), // legacy mirror; the wardrobe is the truth now
            wardrobe: self.wardrobe.clone(),
            color_choice: self.color_choice.clone(),
            satisfied_gates: self.satisfied_gates.iter().cloned().collect(),
            paid_tolls: self.paid_tolls.iter().cloned().collect(),
            seen_intros: self.seen_intros.iter().cloned().collect(),
            fuel: self.fuel,
            upgrades: self.upgrades.iter().cloned().collect(),
            game_pace: self.game_pace,
        }
    }

    fn load_from_save(&mut self, save_data: &SaveData) {
        self.player_name = save_data.name.clone();
        self.player_gender = save_data.gender;
        self.profile = save_data.profile.clone();
        self.dum_dums = save_data.dum_dums;
        self.pearls = save_data.pearls;
        self.play_time = save_data.play_time;
        self.gifts_given = save_data.gifts_given.clone();
        self.wardrobe = save_data.wardrobe.clone();
        self.color_choice = save_data.color_choice.clone();
        self.satisfied_gates = save_data.satisfied_gates.iter().cloned().collect();
        self.paid_tolls = save_data.paid_tolls.iter().cloned().collect();
        self.seen_intros = save_data.seen_intros.iter().cloned().collect();
        self.fuel = save_data.fuel;
        self.upgrades = save_data.upgrades.iter().cloned().collect();
        self.game_pace = save_data.game_pace;

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
        // Parked Sparky comes home on reload — we don't persist whichever map
        // he may have wandered off to, and home is always a safe, valid spot.
        self.sparky_map = SPARKY_HOME_MAP;
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
        QuestStep::Choice { prompt, options } => QuestView::Choice { prompt, options },
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

/// Build the shop's current view (browsing, solving a purchase subtraction,
/// or picking an outfit color) from the active session. Borrows the session
/// so the layout/draw can read it.
fn shop_view<'a>(ash: &'a ActiveShop, color_choice: &str) -> ui::shop::ShopView<'a> {
    if ash.picking_color {
        let current = sprites::player::OUTFIT_COLORS
            .iter()
            .position(|(id, _)| *id == color_choice)
            .unwrap_or(0);
        return ui::shop::ShopView::PickingColor { colors: sprites::player::OUTFIT_COLORS, current };
    }
    if let Some(ref quote) = ash.trading {
        return ui::shop::ShopView::Trading { quote, choices: &ash.choices };
    }
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

/// Answer tiles for "how many groups of `rate` are in this pile?" — the right
/// quotient plus the near-misses a kid actually makes (one group out, or the
/// whole pile counted as singles).
fn division_choices(answer: u32, offered: u32, rng: &mut SmallRng) -> Vec<u32> {
    let mut out = vec![answer];
    for cand in [answer + 1, answer.saturating_sub(1), offered, answer + 2] {
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

/// Screen-space rect of the on-screen settings gear (bottom-right, clear of the
/// top HUD/area-name), so parents can open settings — and the feature flags
/// inside — without a keyboard.
fn settings_gear_rect(screen: (f32, f32)) -> (f32, f32, f32, f32) {
    let (sw, sh) = screen;
    let size = 44.0;
    (sw - size - 12.0, sh - size - 12.0, size, size)
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
        Signpost => &[
            "Howdy! I've pointed the way for YEARS. Bit lonely, though.",
            "Psst... give a fella a Dum Dum and I'll come adventuring with you!",
            "I know ALL the directions. Left, right, up... and the other one!",
        ],
        // ReefShark normally reaches the player through the gate-challenge path,
        // not here — but if you chat after he's stepped aside, he's a sweetie.
        ReefShark => &[
            "Thanks for the puzzle, pal! Naps are better after a good brain stretch.",
            "Toothy grin, gentle heart. That's me!",
            "Swim on through, the cove's all yours!",
        ],
        SeaTurtle => &[
            "Greetings, little diver. I've ridden these currents a hundred years.",
            "Slow and steady finds the most pearls, you know.",
            "The coral grows a tiny bit every day. Just like you!",
        ],
        Dolphin => &[
            "Eee-eee! Give me a Dum Dum and you can ride on my back! Zoooom!",
            "Wanna race? I'll give you a head start! ...okay maybe two!",
            "Did you see my flip? I've been practicing!",
            "Bubbles are the BEST. Watch — bloop bloop bloop!",
        ],
        Crab => &[
            "Snip snap! Mind the claws, I'm just saying hi!",
            "Sideways is the only way to walk, obviously.",
            "I keep the sand tidy around here. Very important job.",
        ],
        Jelly => &[
            "...blub... (the jellyfish wobbles a friendly hello)",
            "Drifting is a perfectly good plan, thank you very much.",
            "Don't worry, I'm the no-sting kind!",
        ],
        Octopus => &[
            "Want to see the trench? Take the shaft — but you have to land RIGHT on the door!",
            "Kick down in big kicks or little ones. Five and five and two, that sort of thing!",
            "Mind the rock ledges — you can't rest on those. Eight arms and I still bonk them.",
        ],
        Clam => &[
            "Brrbl! My pearl hides under a stone — but the current's too strong to walk. You LEAP!",
            "Pick how big your leaps are BEFORE you jump. Every leap the same size, that's the trick!",
            "Too big and you'll sail right over it. Swim back and try a different size — I don't mind!",
            "Every time you find my pearl, I hide it again. It's my favorite game!",
        ],
        Anglerfish => &[
            "Like my light? I grew it myself! It's for finding pearls... and friends!",
            "Down here the dark is friendly — especially with a lamp on your head!",
            "If you get lost in the trench, just follow the glow. That's me!",
        ],
        Eel => &[
            "Wiggle wiggle! I know every crack and cranny in this trench!",
            "Did somebody say TREASURE? There's a chest past the vents, you know.",
            "I'm not slimy, I'm streamlined!",
        ],
        HermitCrab => &[
            "Pssst! Down here! I carry my whole shop on my back, see?",
            "Pearls only, friend. Dum Dums are for surface folk!",
            "Got pearls? I've got kelp crowns, and a net that finds you MORE pearls.",
            "Three pearls make a Dum Dum. That's the going rate and I'll not budge.",
        ],
        TurtleElder => &[
            "Come in, come in, little swimmer! Mind the kettle vent, it bubbles.",
            "I've lived on this reef two hundred years. The numbered stones? I helped lay them!",
            "Rest your fins a moment, dear. Adventuring is hungry work.",
        ],
        MoonAlien => &[
            "Zorp! You bounced all the way to the Moon! Boing boing!",
            "Low gravity is the BEST. Watch me jump super high! Wheee!",
            "I collect moon rocks. Wanna see? I have... a LOT.",
        ],
        // FuelBot reaches the player through the refuel-challenge path, not here.
        FuelBot => &[
            "BEEP. Tank online. Solve my puzzle and I'll top off your rocket!",
            "Fuel is friendship. ...no wait, that's not right. BEEP.",
        ],
        // MarsGuardian normally reaches the player via the gate path; this is
        // for after he's waved them through.
        MarsGuardian => &[
            "Course plotted! Safe travels, little astronaut. Rok approves.",
            "The cove's all yours now. Mind the red dust!",
            "Numbers are the best maps. You read them like a pro!",
        ],
        StarKeeper => &[
            "Welcome to the star chart, navigator! Spot the pattern in the stars?",
            "Every constellation hides a sequence. Can you finish it?",
            "Cassi has mapped a thousand skies. Today we map one together!",
        ],
        StationAlien => &[
            "Bleep bloop! A visitor! It's been AGES since anyone docked here!",
            "I keep the station tidy. Floating crumbs are a real problem.",
            "Did you know space has no up or down? My feet sure don't.",
        ],
        // Blaster Bubbe normally launches the shooter on interact; this line is
        // only a fallback so the match stays exhaustive.
        ArcadeAlien => &[
            "Bubeleh! Step right up to the cabinet and blast some number bonds!",
        ],
        // Dev-control NPCs go through apply_dev_control, never this path.
        CtrlBand | CtrlKenkenLevel | CtrlCraReset | CtrlIntroReset
        | CtrlTriggerKenken | CtrlTriggerPattern | CtrlTriggerBalance
        | CtrlTriggerSudoku | CtrlTriggerChallenge
        | CtrlToggleEncounters | CtrlTriggerEncounter
        | CtrlToggleQuest | CtrlStartQuest => &["Hello there!"],
    };
    let idx = rng.gen_range(0..lines.len());
    vec![DialogueLine { speaker: npc.name().into(), text: lines[idx].into() }]
}

/// The world-space rect the camera actually shows. `render_world`'s Camera2D
/// maps one world pixel to one screen pixel, centered on the logical
/// GAME_W×GAME_H frame — so a window larger than 960×720 sees MORE world than
/// the frame. Every "is it on screen" decision (tile drawing, atmosphere
/// overlays, sprite culling, leaver eviction) must go through this one rect;
/// anything measured against GAME_W×GAME_H instead ends up drawn over — or
/// hidden inside — the undrawn void at the window's fringe.
pub fn visible_world_rect(cam: (f32, f32), screen: (f32, f32)) -> Rect {
    let (sw, sh) = screen;
    Rect::new(
        cam.0 + (GAME_W - sw) / 2.0,
        cam.1 + (GAME_H - sh) / 2.0,
        sw,
        sh,
    )
}

/// True iff any pixel of the NPC's tile rect overlaps the visible world rect.
/// Used to gate wander cooldown ticks — off-screen wanderers freeze in place
/// so unseen rooms don't burn RNG and don't have characters drifting around
/// out of sight.
fn npc_in_camera(view: Rect, n: &npc::Npc) -> bool {
    view.overlaps(&Rect::new(n.entity.x, n.entity.y, TILE_SIZE, TILE_SIZE))
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

fn speak_challenge_feedback(cs: &ChallengeState, speaker: &str) {
    if let Some(ref fb) = cs.feedback {
        audio::tts::speak(speaker, &fb.speech);
    }
}

/// Flavor lines for stepping into a secret/special map, voiced by whoever's
/// currently tagging along (`speaker`) — not always Sparky, since the player
/// may be travelling with a recruited buddy.
fn secret_entry_dialogue(map_id: &str, speaker: &str) -> Vec<DialogueLine> {
    let line = |text: &str| DialogueLine { speaker: speaker.to_string(), text: text.to_string() };
    match map_id {
        "dream" => vec![
            line("BZZZT! Boss! My circuits feel all tingly! Everything looks... purple?"),
            line("Are we... dreaming? The flowers are floating! BEEP BOOP this is WEIRD!"),
        ],
        "doghouse" => vec![
            line("ERROR ERROR! Visual systems reporting... BORK?! What IS this place?!"),
            line("My display is all glitchy! I see scan lines and... is that a DOG made of CODE?!"),
        ],
        "grove" => vec![
            line("Whoa boss! We just walked RIGHT THROUGH those trees! How did we do that?!"),
            line("This place is SO pretty! And SO secret! The trees are whispering!"),
        ],
        "reef" => vec![
            line("BLUB BLUB! We're UNDERWATER, boss! And I didn't even rust! Best upgrade EVER!"),
            line("Look — coral, kelp, and is that a SHARK napping on the path? Let's go say hi!"),
            line("See Shelly the clam by the number-stones? Her bubble says which stone hides her PEARL!"),
            line("And little houses to the east! An underwater VILLAGE! Can we knock? Please please please?"),
        ],
        "trench" => vec![
            line("WHOA, the deep trench! It's darker down here, boss... and look at all the glowing vents!"),
            line("There's another Shelly with number-stones — find her pearl! The bright bubble column takes us back up."),
        ],
        "space_hub" => vec![
            line("3... 2... 1... BLAST OFF! WHEEEE! Boss, we're in SPACE! Actual outer SPACE!"),
            line("Fly the rocket to a glowing pad to visit a planet! Tank the fuel droid is over there if we run low."),
        ],
        _ => vec![],
    }
}

/// Shelly's call: the stone her pearl is under, plus the one clue she gives.
/// Younger kids get the leap size and skip-count it out; older ones get the
/// number of leaps and have to work the size out.
fn leap_call(puzzle: &LeapPuzzle) -> String {
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
fn draw_number_track(
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

    // ── Recruiting a same-kind NPC must not erase its twin on another map ──
    #[test]
    fn recruiting_the_reef_shelly_keeps_the_trench_shelly() {
        let mut g = game();
        let reef_shelly = npc::npcs_for_map("reef").into_iter()
            .find(|n| n.kind == NpcKind::Clam)
            .expect("reef roster has a Clam");
        assert_eq!(reef_shelly.home_map, "reef");
        g.companion = Some(reef_shelly);

        // The trench's own Shelly (a different Clam, home_map "trench") stays.
        let trench = g.load_map_roster("trench");
        assert!(
            trench.iter().any(|n| n.kind == NpcKind::Clam && n.home_map == "trench"),
            "the trench's Shelly must survive recruiting the reef's Shelly",
        );

        // But the companion's own home roster (reef) still drops the duplicate.
        let reef = g.load_map_roster("reef");
        assert!(
            !reef.iter().any(|n| n.kind == NpcKind::Clam),
            "the reef's Shelly is the companion, so she isn't also in the reef roster",
        );
    }

    // ── The visible-rect seam: what the camera shows is what gets drawn ──
    #[test]
    fn visible_world_rect_matches_the_window_not_the_logical_frame() {
        // At the logical size the rect IS the camera frame.
        let r = visible_world_rect((96.0, 48.0), (GAME_W, GAME_H));
        assert_eq!((r.x, r.y, r.w, r.h), (96.0, 48.0, GAME_W, GAME_H));

        // A larger window sees MORE world, centered on the same frame — the
        // extra margin splits evenly on both sides. draw_map and the sprite
        // culling both consume this rect, so nothing can be visible over
        // undrawn tiles at the window fringe.
        let r = visible_world_rect((96.0, 48.0), (GAME_W + 200.0, GAME_H + 100.0));
        assert_eq!((r.x, r.y), (96.0 - 100.0, 48.0 - 50.0));
        assert_eq!((r.w, r.h), (GAME_W + 200.0, GAME_H + 100.0));
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

    // ── On-screen settings gear opens the overlay (keyboard-free access) ──
    #[test]
    fn tapping_the_gear_opens_settings() {
        let mut g = game();
        g.set_state(GameState::Playing);
        assert!(!g.settings_open);
        let (gx, gy, gw, gh) = settings_gear_rect((960.0, 720.0));
        let click = crate::input::FrameInput::empty().with_mouse_click(gx + gw / 2.0, gy + gh / 2.0);
        g.step(&click, 1.0 / 60.0, (960.0, 720.0));
        assert!(g.settings_open, "tapping the gear should open settings");
    }

    #[test]
    fn tapping_elsewhere_does_not_open_settings() {
        let mut g = game();
        g.set_state(GameState::Playing);
        // A tap in the middle of the screen is gameplay (click-to-walk), not the gear.
        let click = crate::input::FrameInput::empty().with_mouse_click(480.0, 360.0);
        g.step(&click, 1.0 / 60.0, (960.0, 720.0));
        assert!(!g.settings_open, "a non-gear tap must not open settings");
    }

    // ── Shop cosmetics survive save → load ──
    #[test]
    fn shop_cosmetics_persist_through_save_load() {
        let mut g = game();
        g.wardrobe.put_on(wardrobe::PLAYER, "hat");
        g.wardrobe.put_on(wardrobe::PLAYER, "bow_tie");
        let data = g.gather_save_data();

        let mut g2 = game();
        assert!(g2.player_swag().is_empty());
        g2.load_from_save(&data);
        assert!(g2.player_swag().contains("hat"), "hat should persist");
        assert!(g2.player_swag().contains("bow_tie"), "bow tie should persist");
    }

    // ── Swag given to a buddy stays theirs across a save → load ──
    #[test]
    fn swag_given_to_a_buddy_persists_through_save_load() {
        let mut g = game();
        g.wardrobe.put_on(wardrobe::PLAYER, "hat");
        assert_eq!(g.wardrobe.hand_over(wardrobe::PLAYER, "dolphin", "hat"), HandOver::Given);
        let data = g.gather_save_data();

        let mut g2 = game();
        g2.load_from_save(&data);
        assert!(g2.swag_worn_by("dolphin").contains("hat"),
            "Echo should still be wearing the hat next session");
        assert!(g2.player_swag().is_empty(),
            "the kid gave it away, so Bolt can sell them another one");
    }

    // ── A legacy save's cosmetics land on the kid ──
    #[test]
    fn legacy_shop_owned_migrates_onto_the_kid() {
        let mut data = game().gather_save_data();
        data.shop_owned = vec!["hat".into(), "jet_boots".into()];
        data.wardrobe = Wardrobe::new();
        data.migrate_legacy();

        let mut g = game();
        g.load_from_save(&data);
        assert!(g.player_swag().contains("hat"));
        assert!(g.player_swag().contains("jet_boots"));
    }

    // ── Handing swag over puts it back on Bolt's shelf ──
    #[test]
    fn giving_swag_away_lets_bolt_sell_another_one() {
        let mut g = game();
        g.wardrobe.put_on(wardrobe::PLAYER, "hat");
        assert_eq!(
            shop::process_purchase(20, "hat", g.player_swag()),
            shop::PurchaseOutcome::AlreadyOwned,
            "no buying a second hat while you're wearing one",
        );

        g.wardrobe.hand_over(wardrobe::PLAYER, "kid_1", "hat");
        assert!(
            matches!(shop::process_purchase(20, "hat", g.player_swag()),
                shop::PurchaseOutcome::Bought { .. }),
            "once the hat is Tali's, the kid can buy themselves another",
        );
    }

    // ── The arcade pace dial survives a save → load ──
    #[test]
    fn arcade_pace_persists_through_save_load() {
        let mut g = game();
        assert_eq!(g.game_pace, GamePace::Steady, "new games start at the shipped pace");
        g.game_pace = GamePace::Relaxed;
        let data = g.gather_save_data();

        let mut g2 = game();
        g2.load_from_save(&data);
        assert_eq!(g2.game_pace, GamePace::Relaxed,
            "a parent sets the pace once, not every session");
    }

    /// A save written before the dial existed opens at the pace the cabinet
    /// shipped with, so nobody's game changes under them.
    #[test]
    fn a_legacy_save_opens_at_the_shipped_pace() {
        let json = r#"{
            "version": 1, "name": "Ari", "gender": "Boy",
            "map_id": "overworld", "player_x": 3, "player_y": 4, "player_dir": "Down",
            "sparky_x": 3, "sparky_y": 5,
            "dum_dums": 7, "play_time": 120.0, "timestamp": 0
        }"#;
        let save: crate::save::SaveData =
            serde_json::from_str(json).expect("legacy save should load");
        assert_eq!(save.game_pace, GamePace::Steady);
    }

    // ── Color Change comes with a color picker ──

    const SCREEN: (f32, f32) = (960.0, 720.0);

    /// Open Bolt's shop directly (skipping the walk-and-talk).
    fn open_shop(g: &mut Game) {
        g.active_shop = Some(ActiveShop {
            shop: ShopKind::Bolt,
            trading: None,
            catalog: shop::shop_catalog(),
            owned: g.player_swag().clone(),
            selected: None,
            choices: Vec::new(),
            answer: 0,
            cost: 0,
            balance_before: 0,
            message: None,
            source_npc: "shopkeeper".into(),
            picking_color: false,
        });
        g.set_state(GameState::Shop);
    }

    /// Click whatever shop element sits at the center of `rect`.
    fn click_shop(g: &mut Game, rect: ui::shop::UiRect) {
        let click = crate::input::FrameInput::empty()
            .with_mouse_click(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        g.step(&click, 1.0 / 60.0, SCREEN);
    }

    fn shop_layout(g: &Game) -> ui::shop::ShopLayout {
        let ash = g.active_shop.as_ref().expect("shop should be open");
        ui::shop::layout(&ash.catalog, &shop_view(ash, &g.color_choice), SCREEN)
    }

    #[test]
    fn buying_color_change_opens_the_picker_and_picking_sticks() {
        let mut g = game();
        g.dum_dums = 20;
        open_shop(&mut g);

        // Tap the Color Change row, then answer the purchase subtraction.
        let row = shop_layout(&g).items.iter()
            .find(|r| g.active_shop.as_ref().unwrap().catalog[r.index].id == "color_change")
            .expect("color_change in catalog").rect;
        click_shop(&mut g, row);
        let answer = g.active_shop.as_ref().unwrap().answer;
        let tile = shop_layout(&g).answers.iter()
            .find(|t| t.value == answer).expect("correct answer tile").rect;
        click_shop(&mut g, tile);

        let ash = g.active_shop.as_ref().unwrap();
        assert!(ash.owned.contains("color_change"));
        assert!(ash.picking_color, "buying Color Change should open the picker");

        // Pick the second swatch; the kid's outfit color should change.
        let swatch = shop_layout(&g).swatches[1].rect;
        click_shop(&mut g, swatch);
        assert_eq!(g.color_choice, sprites::player::OUTFIT_COLORS[1].0);

        // Done dismisses the picker but keeps the shop open.
        let close = shop_layout(&g).close_btn;
        click_shop(&mut g, close);
        let ash = g.active_shop.as_ref().unwrap();
        assert!(!ash.picking_color, "Done should close the picker first");
        assert!(g.active_shop.is_some(), "the shop itself should stay open");
    }

    #[test]
    fn changing_color_back_and_forth_sticks_each_time() {
        let mut g = game();
        g.wardrobe.put_on(wardrobe::PLAYER, "color_change");
        open_shop(&mut g);

        // Reopen the picker from the owned Color Change row.
        let row = shop_layout(&g).items.iter()
            .find(|r| g.active_shop.as_ref().unwrap().catalog[r.index].id == "color_change")
            .unwrap().rect;
        click_shop(&mut g, row);
        assert!(g.active_shop.as_ref().unwrap().picking_color);

        // Pick a sequence with repeats and back-tracking. Each pick must stick,
        // the picker must stay open, and the highlighted swatch must follow.
        for &i in &[1usize, 3, 6, 3, 1, 0, 6, 0] {
            let swatch = shop_layout(&g).swatches[i].rect;
            click_shop(&mut g, swatch);
            assert_eq!(g.color_choice, sprites::player::OUTFIT_COLORS[i].0,
                "picking swatch {i} should set color_choice to {}", sprites::player::OUTFIT_COLORS[i].0);
            assert!(g.active_shop.as_ref().unwrap().picking_color,
                "picker should stay open so the kid can keep changing colors");
            match shop_view(g.active_shop.as_ref().unwrap(), &g.color_choice) {
                ui::shop::ShopView::PickingColor { current, .. } =>
                    assert_eq!(current, i, "the highlighted swatch should track the latest pick"),
                _ => panic!("expected the PickingColor view while picking"),
            }
        }
    }

    #[test]
    fn owned_color_change_row_reopens_the_picker() {
        let mut g = game();
        g.wardrobe.put_on(wardrobe::PLAYER, "color_change");
        open_shop(&mut g);
        let row = shop_layout(&g).items.iter()
            .find(|r| g.active_shop.as_ref().unwrap().catalog[r.index].id == "color_change")
            .unwrap().rect;
        click_shop(&mut g, row);
        assert!(
            g.active_shop.as_ref().unwrap().picking_color,
            "tapping an owned Color Change should reopen the picker, not refuse the sale"
        );
    }

    #[test]
    fn color_choice_persists_through_save_load() {
        let mut g = game();
        g.color_choice = "teal".to_string();
        let data = g.gather_save_data();

        let mut g2 = game();
        g2.load_from_save(&data);
        assert_eq!(g2.color_choice, "teal");
    }

    #[test]
    fn saves_from_before_the_picker_default_to_the_original_tint() {
        let g = game();
        let mut json = serde_json::to_value(g.gather_save_data()).unwrap();
        json.as_object_mut().unwrap().remove("color_choice");
        let data: crate::save::SaveData = serde_json::from_value(json).unwrap();
        assert_eq!(data.color_choice, sprites::player::OUTFIT_COLORS[0].0);
    }
}
