use macroquad::prelude::*;

mod tilemap;
mod sprites;
mod npc;
mod ui;

use tilemap::{Map, TILE_SIZE};
use sprites::Dir;
use ui::dialogue::{DialogueBox, DialogueLine};

const GAME_W: f32 = 960.0;
const GAME_H: f32 = 720.0;
const MOVE_SPEED: f32 = 200.0;

#[derive(PartialEq)]
enum GameState {
    Playing,
    Dialogue,
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
    let map = Map::overworld();
    let mut player = Entity::new(14, 12);
    let mut sparky = Sparky::new(14, 13);
    let mut camera = GameCamera { x: 0.0, y: 0.0 };
    let mut game_time: f32 = 0.0;
    let npcs = npc::npcs_for_map(map.id);
    let mut dialogue = DialogueBox::new();
    let mut state = GameState::Playing;

    loop {
        let dt = get_frame_time();
        game_time += dt;

        // ─── INPUT ────────────────────────────────────
        match state {
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
                    let player_trapped = [(0i32,1i32),(0,-1),(1,0),(-1,0)].iter().all(|(dx,dy)| {
                        let cx = player.tile_x as i32 + dx;
                        let cy = player.tile_y as i32 + dy;
                        cx < 0 || cy < 0
                            || cx as usize >= map.width || cy as usize >= map.height
                            || map.is_solid(cx as usize, cy as usize)
                            || npcs.iter().any(|n| n.tile_x == cx as usize && n.tile_y == cy as usize)
                            || (cx as usize == sparky.entity.tile_x && cy as usize == sparky.entity.tile_y)
                    });
                    let sparky_blocks = !player_trapped
                        && nx as usize == sparky.entity.tile_x && ny as usize == sparky.entity.tile_y;
                    if moved && nx >= 0 && ny >= 0
                        && (nx as usize) < map.width && (ny as usize) < map.height
                        && !map.is_solid(nx as usize, ny as usize)
                        && !sparky_blocks && !npc_blocks
                    {
                        sparky.record_player_pos(player.tile_x, player.tile_y);
                        player.start_move(nx as usize, ny as usize);
                    }
                }

                // Space: interact
                if is_key_pressed(KeyCode::Space) && !player.moving {
                    if let Some(target) = npc::get_interact_target(
                        player.tile_x, player.tile_y, player.dir, &npcs
                    ) {
                        let lines = npc_dialogue_lines(target);
                        dialogue.start(lines);
                        state = GameState::Dialogue;
                    } else if npc::is_facing_sparky(
                        player.tile_x, player.tile_y, player.dir,
                        sparky.entity.tile_x, sparky.entity.tile_y,
                    ) {
                        dialogue.start(sparky_dialogue_lines());
                        state = GameState::Dialogue;
                    }
                }
            }
            GameState::Dialogue => {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    dialogue.advance();
                    if !dialogue.active {
                        state = GameState::Playing;
                    }
                }
            }
        }

        // ─── UPDATE ───────────────────────────────────
        player.move_toward_target(dt);
        sparky.update(dt, player.tile_x, player.tile_y);
        dialogue.update(dt);
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
        draw_text(&format!("FPS: {} | Tile: {},{}", get_fps(), player.tile_x, player.tile_y),
            10.0, 20.0, 20.0, WHITE);

        // Dialogue box
        dialogue.draw();

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
