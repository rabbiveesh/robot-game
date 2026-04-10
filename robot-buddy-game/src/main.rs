use macroquad::prelude::*;
use ::rand::SeedableRng;
use ::rand::rngs::SmallRng;

use robot_buddy_domain::challenge::challenge_state::{
    ChallengeState, DisplaySpeech, RenderHint, VoiceState,
    challenge_reducer,
};
use robot_buddy_domain::learning::challenge_generator::{
    Challenge, ChallengeProfile, generate_challenge,
};
use robot_buddy_domain::learning::operation_stats::OperationStats;
use robot_buddy_domain::types::{Phase, CraStage};

mod tilemap;
mod sprites;
mod npc;
mod ui;
mod save;

use tilemap::{Map, TILE_SIZE};
use sprites::Dir;
use ui::dialogue::{DialogueBox, DialogueLine};
use ui::challenge::{ChoiceBound, ScaffoldBounds};
use ui::title_screen::{TitleAction, NewGameAction, NewGameForm};
use save::{SaveData, Gender};

const GAME_W: f32 = 960.0;
const GAME_H: f32 = 720.0;
const MOVE_SPEED: f32 = 200.0;

#[derive(PartialEq)]
enum GameState {
    Title,
    NewGame,
    Playing,
    Dialogue,
    Challenge,
}

/// Active challenge data — the domain ChallengeState + the generated Challenge.
struct ActiveChallenge {
    state: ChallengeState,
    challenge: Challenge,
    choice_bounds: Vec<ChoiceBound>,
    scaffold: ScaffoldBounds,
}

fn make_challenge_profile() -> ChallengeProfile {
    ChallengeProfile {
        math_band: 1,
        spread_width: 0.5,
        operation_stats: OperationStats::new(),
    }
}

fn start_challenge(rng: &mut SmallRng) -> ActiveChallenge {
    let profile = make_challenge_profile();
    let challenge = generate_challenge(&profile, rng);
    let cra = CraStage::Abstract; // default starting CRA

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
                                    &mut dum_dums, &mut play_time);
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
                    form.handle_gender_click();
                    if let Some(action) = form.draw() {
                        match action {
                            NewGameAction::Start => {
                                let slot = form.slot;
                                player_name = form.name.clone();
                                player_gender = form.gender;
                                dum_dums = 0;
                                play_time = 0.0;
                                active_slot = slot;

                                // Reset game state
                                map = Map::overworld();
                                player = Entity::new(14, 12);
                                sparky = Sparky::new(14, 13);
                                npcs = npc::npcs_for_map(map.id);
                                camera = GameCamera { x: 0.0, y: 0.0 };

                                // Save immediately
                                let save = gather_save_data(&player, &sparky, &map,
                                    &player_name, player_gender, dum_dums, play_time);
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
                    // Long-press (0.3s) lets you walk through Sparky
                    let sparky_blocks = pushing_sparky && sparky_push_timer < 0.3;
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
                    if let Some(target) = npc::get_interact_target(
                        player.tile_x, player.tile_y, player.dir, &npcs
                    ) {
                        if !target.never_challenge {
                            // Challenge-eligible NPC: dialogue then challenge
                            let lines = npc_dialogue_lines(target);
                            pending_challenge = true;
                            dialogue.start(lines);
                            state = GameState::Dialogue;
                        } else {
                            let lines = npc_dialogue_lines(target);
                            dialogue.start(lines);
                            state = GameState::Dialogue;
                        }
                    } else if npc::is_facing_sparky(
                        player.tile_x, player.tile_y, player.dir,
                        sparky.entity.tile_x, sparky.entity.tile_y,
                    ) {
                        // Sparky always challenges
                        pending_challenge = true;
                        dialogue.start(sparky_dialogue_lines());
                        state = GameState::Dialogue;
                    }
                }
            }
            GameState::Dialogue => {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    dialogue.advance();
                    if !dialogue.active {
                        if pending_challenge {
                            pending_challenge = false;
                            active_challenge = Some(start_challenge(&mut rng));
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
                    // Keyboard input
                    if let Some(action) = ui::challenge::handle_key(&ac.state, &ac.challenge) {
                        ac.state = challenge_reducer(ac.state.clone(), action);
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
                        } else if ac.state.phase == Phase::Complete {
                            dismiss = true;
                        }
                    }
                }
                if dismiss {
                    active_challenge = None;
                    state = GameState::Playing;
                }
            }
        }

        // ─── UPDATE ───────────────────────────────────
        // Only track time + auto-save during actual gameplay
        if state != GameState::Title && state != GameState::NewGame {
            play_time += dt;
            auto_save_timer += dt;
            if auto_save_timer >= 30.0 {
                auto_save_timer = 0.0;
                let save = gather_save_data(&player, &sparky, &map,
                    &player_name, player_gender, dum_dums, play_time);
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
                let dest_dir = portal.dir;

                // Swap map
                map = Map::by_id(dest_map);
                npcs = npc::npcs_for_map(map.id);

                // Teleport player
                player.tile_x = dest_x;
                player.tile_y = dest_y;
                player.x = dest_x as f32 * TILE_SIZE;
                player.y = dest_y as f32 * TILE_SIZE;
                player.target_x = player.x;
                player.target_y = player.y;
                player.moving = false;
                player.dir = match dest_dir {
                    0 => Dir::Up,
                    1 => Dir::Down,
                    2 => Dir::Left,
                    _ => Dir::Right,
                };

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
                0 => sprites::player::draw_player_boy(player.x, player.y, player.dir, player.frame, game_time),
                1 => sprites::robot::draw_robot(sparky.entity.x, sparky.entity.y, sparky.entity.dir, sparky.entity.frame, game_time),
                2 => npcs[r.idx].draw(game_time),
                _ => {}
            }
        }

        // HUD
        set_default_camera();
        draw_text(&format!("FPS: {} | {} ({},{})", get_fps(), map.id, player.tile_x, player.tile_y),
            10.0, 20.0, 20.0, WHITE);

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
    name: &str, gender: Gender, dum_dums: u32, play_time: f32,
) -> SaveData {
    SaveData {
        version: 1,
        name: name.to_string(),
        gender,
        map_id: map.id.to_string(),
        player_x: player.tile_x,
        player_y: player.tile_y,
        player_dir: match player.dir {
            Dir::Up => 0,
            Dir::Down => 1,
            Dir::Left => 2,
            Dir::Right => 3,
        },
        sparky_x: sparky.entity.tile_x,
        sparky_y: sparky.entity.tile_y,
        dum_dums,
        play_time,
        timestamp: 0, // set by save_to_slot
    }
}

fn load_from_save(
    save: &SaveData, map: &mut Map, player: &mut Entity, sparky: &mut Sparky,
    npcs: &mut Vec<npc::Npc>, name: &mut String, gender: &mut Gender,
    dum_dums: &mut u32, play_time: &mut f32,
) {
    *name = save.name.clone();
    *gender = save.gender;
    *dum_dums = save.dum_dums;
    *play_time = save.play_time;

    *map = Map::by_id(&save.map_id);
    *npcs = npc::npcs_for_map(map.id);

    player.tile_x = save.player_x;
    player.tile_y = save.player_y;
    player.x = save.player_x as f32 * TILE_SIZE;
    player.y = save.player_y as f32 * TILE_SIZE;
    player.target_x = player.x;
    player.target_y = player.y;
    player.moving = false;
    player.dir = match save.player_dir {
        0 => Dir::Up,
        1 => Dir::Down,
        2 => Dir::Left,
        _ => Dir::Right,
    };

    sparky.entity.tile_x = save.sparky_x;
    sparky.entity.tile_y = save.sparky_y;
    sparky.entity.x = save.sparky_x as f32 * TILE_SIZE;
    sparky.entity.y = save.sparky_y as f32 * TILE_SIZE;
    sparky.entity.target_x = sparky.entity.x;
    sparky.entity.target_y = sparky.entity.y;
    sparky.entity.moving = false;
    sparky.follow_queue.clear();
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
