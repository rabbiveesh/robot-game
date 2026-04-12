use macroquad::prelude::*;
use ::rand::SeedableRng;
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
use robot_buddy_domain::economy::give;
use robot_buddy_domain::economy::interaction_options::{self, NpcInfo, PlayerState};
use robot_buddy_domain::types::{Phase, CraStage, FrustrationLevel};

mod tilemap;
mod sprites;
mod npc;
mod ui;
mod save;
mod audio;
mod session;

use tilemap::{Map, TILE_SIZE};
use sprites::Dir;
use ui::dialogue::{DialogueBox, DialogueLine};
use ui::challenge::{ChoiceBound, ScaffoldBounds};
use ui::title_screen::{TitleAction, NewGameAction, NewGameForm};
use ui::hud::{DumDumHud, DebugOverlay};
use ui::interaction_menu::MenuOption;
use save::{SaveData, Gender};

const GAME_W: f32 = 960.0;
const GAME_H: f32 = 720.0;
const MOVE_SPEED: f32 = 200.0;

#[derive(PartialEq)]
enum GameState {
    Title,
    NewGame,
    Playing,
    InteractionMenu,
    Dialogue,
    Challenge,
}

/// Active challenge data — the domain ChallengeState + the generated Challenge.
struct ActiveChallenge {
    state: ChallengeState,
    challenge: Challenge,
    choice_bounds: Vec<ChoiceBound>,
    scaffold: ScaffoldBounds,
    complete_timer: f32, // counts up from 0 when Phase::Complete + correct
    start_time: f32,     // game_time when challenge was presented (for response time)
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

    // CRA stage per-operation from the profile (defaults to Concrete for new profiles)
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

struct Entity {
    x: f32,
    y: f32,
    tile_x: usize,
    tile_y: usize,
    target_x: f32,
    target_y: f32,
    moving: bool,
    dir: Dir,
    frame: u32,
}

impl Entity {
    fn new(tile_x: usize, tile_y: usize) -> Self {
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

    fn move_toward_target(&mut self, dt: f32) -> bool {
        if !self.moving { return false; }
        let dx = self.target_x - self.x;
        let dy = self.target_y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 2.0 {
            self.x = self.target_x;
            self.y = self.target_y;
            self.moving = false;
            self.frame += 1;
            return true; // arrived
        }
        let step = MOVE_SPEED * dt;
        self.x += dx / dist * step;
        self.y += dy / dist * step;
        false
    }

    fn start_move(&mut self, nx: usize, ny: usize) {
        self.tile_x = nx;
        self.tile_y = ny;
        self.target_x = nx as f32 * TILE_SIZE;
        self.target_y = ny as f32 * TILE_SIZE;
        self.moving = true;
    }
}

struct Sparky {
    entity: Entity,
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
        // Only add if different from last queued position
        if self.follow_queue.last() != Some(&(tx, ty)) {
            self.follow_queue.push((tx, ty));
        }
    }

    fn update(&mut self, dt: f32, player_tx: usize, player_ty: usize) {
        self.entity.move_toward_target(dt);

        if !self.entity.moving && !self.follow_queue.is_empty() {
            // Don't move if already adjacent to player
            let dx = (self.entity.tile_x as i32 - player_tx as i32).abs();
            let dy = (self.entity.tile_y as i32 - player_ty as i32).abs();
            if dx + dy <= 1 {
                self.follow_queue.clear();
                // Face the player
                let fdx = player_tx as i32 - self.entity.tile_x as i32;
                let fdy = player_ty as i32 - self.entity.tile_y as i32;
                if fdx < 0 { self.entity.dir = Dir::Left; }
                else if fdx > 0 { self.entity.dir = Dir::Right; }
                else if fdy < 0 { self.entity.dir = Dir::Up; }
                else if fdy > 0 { self.entity.dir = Dir::Down; }
                return;
            }

            let (nx, ny) = self.follow_queue.remove(0);
            // Don't walk onto the player's current tile
            if nx == player_tx && ny == player_ty {
                return;
            }
            let dx = nx as i32 - self.entity.tile_x as i32;
            let dy = ny as i32 - self.entity.tile_y as i32;
            if dx < 0 { self.entity.dir = Dir::Left; }
            else if dx > 0 { self.entity.dir = Dir::Right; }
            else if dy < 0 { self.entity.dir = Dir::Up; }
            else if dy > 0 { self.entity.dir = Dir::Down; }
            self.entity.start_move(nx, ny);
        }
    }
}

struct GameCamera {
    x: f32,
    y: f32,
}

impl GameCamera {
    fn follow(&mut self, target_x: f32, target_y: f32, map: &Map, view_w: f32, view_h: f32) {
        self.x = target_x - view_w / 2.0 + TILE_SIZE / 2.0;
        self.y = target_y - view_h / 2.0 + TILE_SIZE / 2.0;
        self.x = self.x.max(0.0).min((map.pixel_width() - view_w).max(0.0));
        self.y = self.y.max(0.0).min((map.pixel_height() - view_h).max(0.0));
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Robot Buddy Adventure".to_string(),
        window_width: GAME_W as i32,
        window_height: GAME_H as i32,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut map = Map::overworld();
    let mut player = Entity::new(14, 12);
    let mut sparky = Sparky::new(14, 13);
    let mut camera = GameCamera { x: 0.0, y: 0.0 };
    let mut game_time: f32 = 0.0;
    let mut play_time: f32 = 0.0;
    let mut npcs = npc::npcs_for_map(map.id);
    let mut dialogue = DialogueBox::new();
    let mut state = GameState::Title;
    let mut rng = SmallRng::seed_from_u64(macroquad::rand::rand() as u64);
    let mut active_challenge: Option<ActiveChallenge> = None;
    let mut pending_challenge = false;
    let mut sparky_push_timer: f32 = 0.0;

    // Title/save state
    let mut save_slots = save::load_all_slots();
    let mut new_game_form: Option<NewGameForm> = None;
    let mut active_slot: usize = 0;
    let mut player_name = String::new();
    let mut player_gender = Gender::Boy;
    let mut dum_dums: u32 = 0;
    let mut auto_save_timer: f32 = 0.0;
    let mut dum_dum_hud = DumDumHud::new();
    let mut debug_overlay = DebugOverlay::new();
    let mut gifts_given: HashMap<String, u32> = HashMap::new();
    let mut menu_options: Vec<MenuOption> = vec![];
    let mut menu_target_id: String = String::new(); // NPC id or "sparky" or "chest"
    let mut menu_target_name: String = String::new();
    let mut menu_can_challenge = false;
    let mut profile = LearnerProfile::new();
    let mut behavior_signals: Vec<BehaviorSignal> = vec![];
    let mut dreaming = false; // persists dream overlay across map transitions
    let mut session_log = session::SessionLog::new();

    loop {
        let dt = get_frame_time();
        game_time += dt;

        // ─── INPUT + RENDER (Title/NewGame are self-contained) ──
        match state {
            GameState::Title => {
                if let Some(action) = ui::title_screen::draw_title_screen(&save_slots, game_time) {
                    match action {
                        TitleAction::NewGame(slot) => {
                            new_game_form = Some(NewGameForm::new(slot));
                            state = GameState::NewGame;
                        }
                        TitleAction::LoadGame(slot) => {
                            if let Some(ref save) = save_slots[slot] {
                                load_from_save(save, &mut map, &mut player, &mut sparky,
                                    &mut npcs, &mut player_name, &mut player_gender,
                                    &mut profile, &mut dum_dums, &mut play_time,
                                    &mut gifts_given);
                                active_slot = slot;
                                auto_save_timer = 0.0;

                                // Welcome back dialogue
                                dialogue.start(vec![
                                    DialogueLine {
                                        speaker: "Sparky".into(),
                                        text: format!("BEEP BOOP! Welcome back, {}! I missed you!", save.name),
                                    },
                                ]);
                                state = GameState::Dialogue;
                            }
                        }
                        TitleAction::DeleteSlot(slot) => {
                            save::delete_slot(slot);
                            save_slots = save::load_all_slots();
                        }
                    }
                }
                next_frame().await;
                continue;
            }
            GameState::NewGame => {
                if let Some(ref mut form) = new_game_form {
                    form.update(dt);
                    form.handle_form_clicks();
                    if let Some(action) = form.draw() {
                        match action {
                            NewGameAction::Start => {
                                let slot = form.slot;
                                player_name = form.name.clone();
                                player_gender = form.gender;
                                profile = LearnerProfile::new();
                                profile.math_band = form.math_band;
                                dum_dums = 0;
                                play_time = 0.0;
                                active_slot = slot;
                                behavior_signals.clear();

                                // Reset game state
                                map = Map::overworld();
                                player = Entity::new(14, 12);
                                sparky = Sparky::new(14, 13);
                                npcs = npc::npcs_for_map(map.id);
                                camera = GameCamera { x: 0.0, y: 0.0 };

                                // Save immediately
                                let save = gather_save_data(&player, &sparky, &map,
                                    &player_name, player_gender, &profile, dum_dums, play_time,
                                    &gifts_given);
                                save::save_to_slot(slot, &save);
                                save_slots = save::load_all_slots();
                                auto_save_timer = 0.0;

                                // Welcome dialogue
                                dialogue.start(vec![
                                    DialogueLine {
                                        speaker: "Sparky".into(),
                                        text: format!("BEEP BOOP! Hi {}! I'm Sparky, your robot buddy!", player_name),
                                    },
                                    DialogueLine {
                                        speaker: "Sparky".into(),
                                        text: "Let's go on an ADVENTURE! Talk to people by pressing SPACE!".into(),
                                    },
                                ]);
                                new_game_form = None;
                                state = GameState::Dialogue;
                            }
                            NewGameAction::Back => {
                                new_game_form = None;
                                state = GameState::Title;
                            }
                        }
                    }
                }
                next_frame().await;
                continue;
            }
            GameState::Playing => {
                // Movement
                if !player.moving {
                    let mut nx = player.tile_x as i32;
                    let mut ny = player.tile_y as i32;
                    let mut moved = false;

                    if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
                        ny -= 1; player.dir = Dir::Up; moved = true;
                    } else if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
                        ny += 1; player.dir = Dir::Down; moved = true;
                    } else if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                        nx -= 1; player.dir = Dir::Left; moved = true;
                    } else if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                        nx += 1; player.dir = Dir::Right; moved = true;
                    }

                    let npc_blocks = npcs.iter().any(|n| n.tile_x == nx as usize && n.tile_y == ny as usize);
                    let pushing_sparky = moved
                        && nx as usize == sparky.entity.tile_x && ny as usize == sparky.entity.tile_y;
                    if pushing_sparky {
                        sparky_push_timer += dt;
                    } else {
                        sparky_push_timer = 0.0;
                    }
                    // Brief hold (0.12s) lets you walk through Sparky — tap just turns
                    let sparky_blocks = pushing_sparky && sparky_push_timer < 0.12;
                    if moved && nx >= 0 && ny >= 0
                        && (nx as usize) < map.width && (ny as usize) < map.height
                        && !map.is_solid(nx as usize, ny as usize)
                        && !sparky_blocks && !npc_blocks
                    {
                        sparky_push_timer = 0.0;
                        sparky.record_player_pos(player.tile_x, player.tile_y);
                        player.start_move(nx as usize, ny as usize);
                    }
                }

                // Space: interact
                if is_key_pressed(KeyCode::Space) && !player.moving {
                    // Check for chest tile in front
                    let facing = facing_tile(player.tile_x, player.tile_y, player.dir);
                    let facing_chest = facing.0 < map.width && facing.1 < map.height
                        && map.tiles[facing.1][facing.0] == tilemap::Tile::Chest;

                    if facing_chest {
                        // Chest: auto-trigger challenge with intro dialogue
                        menu_target_id = "chest".into();
                        menu_target_name = "Sparky".into();
                        dialogue.start(vec![DialogueLine {
                            speaker: "Sparky".into(),
                            text: "OOOOH a treasure chest! But it has a LOCK! We need to solve the puzzle to open it!".into(),
                        }]);
                        pending_challenge = true;
                        state = GameState::Dialogue;
                    } else if let Some(target) = npc::get_interact_target(
                        player.tile_x, player.tile_y, player.dir, &npcs
                    ) {
                        // Build interaction options from domain
                        let npc_info = NpcInfo {
                            id: target.id.to_string(),
                            can_receive_gifts: Some(target.can_receive_gifts),
                            has_shop: None,
                        };
                        let player_st = PlayerState { dum_dums };
                        let opts = interaction_options::get_interaction_options(&npc_info, &player_st);

                        menu_target_id = target.id.to_string();
                        menu_target_name = target.name.to_string();
                        menu_can_challenge = !target.never_challenge;

                        // Single option (Talk only) → auto-trigger
                        if opts.len() == 1 {
                            let lines = npc_dialogue_lines(target);
                            if menu_can_challenge && macroquad::rand::gen_range(0.0, 1.0) < 0.4 {
                                pending_challenge = true;
                            }
                            dialogue.start(lines);
                            state = GameState::Dialogue;
                        } else {
                            menu_options = opts.iter().enumerate().map(|(i, o)| MenuOption {
                                option_type: o.option_type.clone(),
                                label: o.label.clone(),
                                key: i + 1,
                            }).collect();
                            state = GameState::InteractionMenu;
                        }
                    } else if npc::is_facing_sparky(
                        player.tile_x, player.tile_y, player.dir,
                        sparky.entity.tile_x, sparky.entity.tile_y,
                    ) {
                        // Sparky interaction menu
                        let npc_info = NpcInfo {
                            id: "sparky".to_string(),
                            can_receive_gifts: Some(true),
                            has_shop: None,
                        };
                        let player_st = PlayerState { dum_dums };
                        let opts = interaction_options::get_interaction_options(&npc_info, &player_st);
                        menu_target_id = "sparky".into();
                        menu_target_name = "Sparky".into();
                        menu_can_challenge = true;

                        if opts.len() == 1 {
                            if macroquad::rand::gen_range(0.0, 1.0) < 0.5 {
                                pending_challenge = true;
                            }
                            dialogue.start(sparky_dialogue_lines());
                            state = GameState::Dialogue;
                        } else {
                            menu_options = opts.iter().enumerate().map(|(i, o)| MenuOption {
                                option_type: o.option_type.clone(),
                                label: o.label.clone(),
                                key: i + 1,
                            }).collect();
                            state = GameState::InteractionMenu;
                        }
                    }
                }
            }
            GameState::InteractionMenu => {
                // Handled in render section (draw + input combined)
            }
            GameState::Dialogue => {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    // Track text skipping (Space while typewriter still running)
                    if dialogue.is_typewriting() {
                        behavior_signals.push(BehaviorSignal {
                            signal: "text_skipped".into(),
                            timestamp: Some(game_time as f64 * 1000.0),
                        });
                        profile = learner_reducer(profile, LearnerEvent::Behavior {
                            signal: "text_skipped".into(),
                        });
                    }
                    dialogue.advance();
                    if !dialogue.active {
                        if pending_challenge {
                            pending_challenge = false;
                            let ac = start_challenge(&mut rng, &profile, game_time);
                            audio::tts::speak("Sparky", &ac.challenge.speech_text);
                            active_challenge = Some(ac);
                            state = GameState::Challenge;
                        } else {
                            state = GameState::Playing;
                        }
                    }
                }
            }
            GameState::Challenge => {
                let mut dismiss = false;
                if let Some(ref mut ac) = active_challenge {
                    // Auto-dismiss timer for correct answers
                    if ac.state.phase == Phase::Complete && ac.state.correct == Some(true) {
                        ac.complete_timer += dt;
                        if ac.complete_timer >= 2.5 {
                            dismiss = true;
                        }
                    }

                    // Keyboard input
                    if let Some(action) = ui::challenge::handle_key(&ac.state, &ac.challenge) {
                        ac.state = challenge_reducer(ac.state.clone(), action);
                        speak_challenge_feedback(&ac.state);
                    } else if ac.state.phase == Phase::Complete
                        && (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter))
                    {
                        dismiss = true;
                    }

                    // Mouse click input
                    if !dismiss && is_mouse_button_pressed(MouseButton::Left) {
                        let (mx, my) = mouse_position();
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
                    if let Some(ref ac) = active_challenge {
                        let was_correct = ac.state.correct == Some(true);
                        let response_ms = ((game_time - ac.start_time) as f64 * 1000.0).min(30000.0);

                        // Log challenge to session
                        session_log.record_challenge(session::ChallengeRecord {
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
                            source: menu_target_id.clone(),
                            play_time_at_event: play_time,
                        });

                        // Feed result into adaptive learning system
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
                            timestamp: Some(game_time as f64 * 1000.0),
                        };
                        profile = learner_reducer(profile, event);

                        // Rapid clicking detection: answer in < 1s is a click-through
                        if response_ms < 1000.0 && !was_correct {
                            let sig = BehaviorSignal {
                                signal: "rapid_clicking".into(),
                                timestamp: Some(game_time as f64 * 1000.0),
                            };
                            behavior_signals.push(sig);
                            profile = learner_reducer(profile, LearnerEvent::Behavior {
                                signal: "rapid_clicking".into(),
                            });
                        }

                        // Frustration detection
                        let frustration = detect_frustration(
                            &profile.rolling_window, &behavior_signals,
                        );
                        if frustration.level == FrustrationLevel::High {
                            profile = learner_reducer(profile, LearnerEvent::FrustrationDetected {
                                level: "high".into(),
                            });
                        }

                        // Award dum dums if challenge had a reward
                        if let Some(ref reward) = ac.state.reward {
                            dum_dums += reward.amount;
                            dum_dum_hud.flash();
                        }
                    }
                    active_challenge = None;
                    state = GameState::Playing;
                }
            }
        }

        // P key: toggle debug overlay (any gameplay state)
        if is_key_pressed(KeyCode::P) && state != GameState::Title && state != GameState::NewGame {
            debug_overlay.toggle();
        }
        dum_dum_hud.update(dt);

        // ─── UPDATE ───────────────────────────────────
        // Only track time + auto-save during actual gameplay
        if state != GameState::Title && state != GameState::NewGame {
            play_time += dt;
            auto_save_timer += dt;
            if auto_save_timer >= 30.0 {
                auto_save_timer = 0.0;
                let save = gather_save_data(&player, &sparky, &map,
                    &player_name, player_gender, &profile, dum_dums, play_time,
                    &gifts_given);
                save::save_to_slot(active_slot, &save);
            }
        }

        let arrived = player.move_toward_target(dt);
        sparky.update(dt, player.tile_x, player.tile_y);
        dialogue.update(dt);

        // Portal check — after player arrives at a new tile
        if arrived && state == GameState::Playing {
            if let Some(portal) = tilemap::check_portal(map.id, player.tile_x, player.tile_y) {
                let secret = portal.secret;
                let dest_map = portal.to_map;
                let dest_x = portal.to_x;
                let dest_y = portal.to_y;

                // Track dream state across transitions
                if dest_map == "dream" {
                    dreaming = true;
                } else if portal.from_map == "dream" && dest_map == "overworld" {
                    // Exiting dream back to overworld ends the dream
                    dreaming = false;
                }

                // Swap map
                map = Map::by_id(dest_map);
                // Apply dream overlay if dreaming (affects non-dream maps too)
                if dreaming && map.render_mode == tilemap::RenderMode::Normal {
                    map.render_mode = tilemap::RenderMode::Dream;
                }
                npcs = npc::npcs_for_map(map.id);

                // Teleport player
                player.tile_x = dest_x;
                player.tile_y = dest_y;
                player.x = dest_x as f32 * TILE_SIZE;
                player.y = dest_y as f32 * TILE_SIZE;
                player.target_x = player.x;
                player.target_y = player.y;
                player.moving = false;
                player.dir = portal.dir;

                // Place Sparky adjacent to player
                let sparky_pos = find_sparky_spot(dest_x, dest_y, &map, &npcs);
                sparky.entity.tile_x = sparky_pos.0;
                sparky.entity.tile_y = sparky_pos.1;
                sparky.entity.x = sparky_pos.0 as f32 * TILE_SIZE;
                sparky.entity.y = sparky_pos.1 as f32 * TILE_SIZE;
                sparky.entity.target_x = sparky.entity.x;
                sparky.entity.target_y = sparky.entity.y;
                sparky.entity.moving = false;
                sparky.follow_queue.clear();

                // Secret area entry dialogue
                if secret {
                    let lines = secret_entry_dialogue(dest_map);
                    if !lines.is_empty() {
                        dialogue.start(lines);
                        state = GameState::Dialogue;
                    }
                }
            }
        }

        camera.follow(player.x, player.y, &map, GAME_W, GAME_H);

        // ─── RENDER ─────────────────────────────────────
        set_camera(&Camera2D {
            zoom: vec2(2.0 / screen_width(), 2.0 / screen_height()),
            target: vec2(camera.x + GAME_W / 2.0, camera.y + GAME_H / 2.0),
            ..Default::default()
        });

        clear_background(Color::from_rgba(26, 26, 46, 255));
        tilemap::draw_map(&map, camera.x, camera.y, GAME_W, GAME_H, game_time);

        // Collect all renderables for Y-sorting
        struct Renderable { y: f32, kind: u8, idx: usize }
        let mut renderables: Vec<Renderable> = vec![];

        renderables.push(Renderable { y: player.y, kind: 0, idx: 0 }); // player
        renderables.push(Renderable { y: sparky.entity.y, kind: 1, idx: 0 }); // sparky
        for (i, n) in npcs.iter().enumerate() {
            renderables.push(Renderable { y: n.pixel_y(), kind: 2, idx: i });
        }
        renderables.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());

        for r in &renderables {
            match r.kind {
                0 => match player_gender {
                    Gender::Boy => sprites::player::draw_player_boy(player.x, player.y, player.dir, player.frame, game_time),
                    Gender::Girl => sprites::player::draw_player_girl(player.x, player.y, player.dir, player.frame, game_time),
                },
                1 => sprites::robot::draw_robot(sparky.entity.x, sparky.entity.y, sparky.entity.dir, sparky.entity.frame, game_time),
                2 => npcs[r.idx].draw(game_time),
                _ => {}
            }
        }

        // HUD (screen space)
        set_default_camera();
        ui::hud::draw_area_name(map.id, player.tile_x, player.tile_y);
        dum_dum_hud.draw(dum_dums);
        let export_clicked = debug_overlay.draw(
            map.id, player.tile_x, player.tile_y, dum_dums, play_time,
            &profile, session_log.challenge_count(), session_log.correct_count(),
        );
        // Export session: button click or E key while overlay is visible
        if export_clicked || (debug_overlay.visible && is_key_pressed(KeyCode::E)) {
            let json = session::build_export(
                &player_name, &session_log, &gifts_given,
                dum_dums, play_time, &profile, map.id,
            );
            let filename = format!("robot-buddy-session-{}.json",
                play_time as u64);
            session::download_json(&json, &filename);
        }

        // Interaction menu (draw + handle input here since it's screen-space)
        if state == GameState::InteractionMenu {
            if let Some(action) = ui::interaction_menu::draw_interaction_menu(&menu_options) {
                match action {
                    ui::interaction_menu::MenuAction::Select(opt_type) => match opt_type.as_str() {
                        "talk" => {
                            if menu_target_id == "sparky" {
                                if menu_can_challenge && macroquad::rand::gen_range(0.0, 1.0) < 0.5 {
                                    pending_challenge = true;
                                }
                                dialogue.start(sparky_dialogue_lines());
                            } else if let Some(target) = npcs.iter().find(|n| n.id == menu_target_id) {
                                if menu_can_challenge && macroquad::rand::gen_range(0.0, 1.0) < 0.4 {
                                    pending_challenge = true;
                                }
                                dialogue.start(npc_dialogue_lines(target));
                            }
                            state = GameState::Dialogue;
                        }
                        "give" => {
                            if let Some(result) = give::process_give(dum_dums, &menu_target_id, &gifts_given) {
                                session_log.record_give(session::GiveRecord {
                                    recipient_id: menu_target_id.clone(),
                                    recipient_name: menu_target_name.clone(),
                                    dum_dums_before: dum_dums,
                                    play_time_at_event: play_time,
                                });
                                dum_dums = result.new_dum_dums;
                                gifts_given = result.new_total_gifts;
                                dum_dum_hud.flash();
                                let reaction = give_reaction_dialogue(&menu_target_id, &menu_target_name, &result.milestone);
                                dialogue.start(reaction);
                                state = GameState::Dialogue;
                            } else {
                                state = GameState::Playing;
                            }
                        }
                        _ => { state = GameState::Playing; }
                    },
                    ui::interaction_menu::MenuAction::Dismiss => {
                        state = GameState::Playing;
                    }
                }
            }
        }

        // Dialogue box
        dialogue.draw();

        // Challenge overlay
        if let Some(ref mut ac) = active_challenge {
            let (bounds, scaffold) = ui::challenge::draw_challenge(&ac.state, &ac.challenge, game_time);
            ac.choice_bounds = bounds;
            ac.scaffold = scaffold;
        }

        next_frame().await
    }
}

fn sparky_dialogue_lines() -> Vec<DialogueLine> {
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
    let idx = macroquad::rand::gen_range(0, lines.len());
    vec![DialogueLine { speaker: "Sparky".into(), text: lines[idx].into() }]
}

fn npc_dialogue_lines(npc: &npc::Npc) -> Vec<DialogueLine> {
    let lines: &[&str] = match npc.id {
        "mommy" => &[
            "Hi sweetie! I'm so proud of you for exploring!",
            "You and Sparky make the best team!",
            "I love you! Keep being amazing!",
        ],
        "sage" | "sage_lab" => &[
            "Ahhhh, young adventurer! The stars told me you'd come!",
            "Welcome! I am Professor Gizmo, master of numbers!",
            "The ancient scrolls speak of a hero... and I think it's YOU!",
        ],
        "kid_1" => &[
            "Wanna see me do a cartwheel? Watch! ...okay I can't actually do one yet.",
            "Sparky is SO COOL! I wish I had a robot friend!",
            "Did you know frogs can jump SUPER far? Like, really far!",
        ],
        "kid_2" => &[
            "Hi... um... do you like bugs? I found a really cool one.",
            "Sparky beeped at me and I think that means he likes me!",
            "Do you think clouds are soft? I think they're soft.",
        ],
        "shopkeeper" => &[
            "Welcome to my shop! Everything costs Dum Dums!",
            "I've got the finest wares in all of Robot Village!",
        ],
        "dream_sage" => &[
            "You are dreaming... or are you? The numbers whisper here...",
            "In dreams, 2 + 2 can be anything... but it's still 4.",
        ],
        "glitch_dog" => &[
            "BORK BORK! sys.treat.exe... GOOD BOY overflow!",
            "Woof! *static* I am... a good boy? BORK.dll loaded!",
            "fetch(ball) returned: UNDEFINED... but I still love you!",
        ],
        "grove_spirit" => &[
            "How... did you find this place? The trees have hidden it for ages...",
            "It's dangerous to go alone... take this!",
            "The leaves whisper your name... they say you are very clever.",
        ],
        _ => &["Hello there!"],
    };
    let idx = macroquad::rand::gen_range(0, lines.len());
    vec![DialogueLine { speaker: npc.name.into(), text: lines[idx].into() }]
}

/// Find the best adjacent tile for Sparky after a portal transition.
fn find_sparky_spot(player_x: usize, player_y: usize, map: &Map, npcs: &[npc::Npc]) -> (usize, usize) {
    // Try: below, above, right, left
    let candidates = [
        (player_x, player_y + 1),
        (player_x, player_y.wrapping_sub(1)),
        (player_x + 1, player_y),
        (player_x.wrapping_sub(1), player_y),
    ];
    for (cx, cy) in candidates {
        if cx < map.width && cy < map.height
            && !map.is_solid(cx, cy)
            && !npcs.iter().any(|n| n.tile_x == cx && n.tile_y == cy)
        {
            return (cx, cy);
        }
    }
    // Fallback: same tile as player
    (player_x, player_y)
}

fn gather_save_data(
    player: &Entity, sparky: &Sparky, map: &Map,
    name: &str, gender: Gender, profile: &LearnerProfile, dum_dums: u32, play_time: f32,
    gifts_given: &HashMap<String, u32>,
) -> SaveData {
    SaveData {
        version: 2,
        name: name.to_string(),
        gender,
        map_id: map.id.to_string(),
        player_x: player.tile_x,
        player_y: player.tile_y,
        player_dir: player.dir,
        sparky_x: sparky.entity.tile_x,
        sparky_y: sparky.entity.tile_y,
        math_band: None,
        dum_dums,
        play_time,
        timestamp: 0,
        gifts_given: gifts_given.clone(),
        profile: profile.clone(),
    }
}

fn load_from_save(
    save: &SaveData, map: &mut Map, player: &mut Entity, sparky: &mut Sparky,
    npcs: &mut Vec<npc::Npc>, name: &mut String, gender: &mut Gender,
    profile: &mut LearnerProfile, dum_dums: &mut u32, play_time: &mut f32,
    gifts_given: &mut HashMap<String, u32>,
) {
    *name = save.name.clone();
    *gender = save.gender;
    *profile = save.profile.clone();
    *dum_dums = save.dum_dums;
    *play_time = save.play_time;
    *gifts_given = save.gifts_given.clone();

    *map = Map::by_id(&save.map_id);
    *npcs = npc::npcs_for_map(map.id);

    player.tile_x = save.player_x;
    player.tile_y = save.player_y;
    player.x = save.player_x as f32 * TILE_SIZE;
    player.y = save.player_y as f32 * TILE_SIZE;
    player.target_x = player.x;
    player.target_y = player.y;
    player.moving = false;
    player.dir = save.player_dir;

    sparky.entity.tile_x = save.sparky_x;
    sparky.entity.tile_y = save.sparky_y;
    sparky.entity.x = save.sparky_x as f32 * TILE_SIZE;
    sparky.entity.y = save.sparky_y as f32 * TILE_SIZE;
    sparky.entity.target_x = sparky.entity.x;
    sparky.entity.target_y = sparky.entity.y;
    sparky.entity.moving = false;
    sparky.follow_queue.clear();
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
) -> Vec<DialogueLine> {
    let text = if let Some(ms) = milestone {
        match (target_id, ms.reaction.as_str()) {
            ("sparky", "first") => "My FIRST Dum Dum?! This is the BEST DAY of my robot LIFE!".into(),
            ("sparky", "spin") => "FIVE DUM DUMS! Watch me spin! *spins* WHOAAAA!".into(),
            ("sparky", "accessory") => "TEN?! I'm wearing a bow tie now! Do I look fancy?!".into(),
            ("sparky", "color_change") => "TWENTY! My chest light is changing color! BZZZT!".into(),
            ("sparky", "ultimate") => "FIFTY DUM DUMS. Boss. I... I don't have words. BEEP.".into(),
            (_, "first") => format!("My first Dum Dum! Thank you so much, you're the best!"),
            _ => format!("WOW! You've given me {} Dum Dums! You're amazing!", ms.total),
        }
    } else {
        match target_id {
            "sparky" => {
                let lines = ["MMMMM! *crunch* Circuits... BUZZING!", "Dum Dum Dum Dum! That's my favorite song!", "BZZZT! Sugar rush! BEEP BOOP BEEP!"];
                lines[macroquad::rand::gen_range(0, lines.len())].into()
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
