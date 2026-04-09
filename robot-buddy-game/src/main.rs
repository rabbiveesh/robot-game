use macroquad::prelude::*;

mod tilemap;
mod sprites;

use tilemap::{Map, TILE_SIZE};
use sprites::Dir;

const GAME_W: f32 = 960.0;
const GAME_H: f32 = 720.0;
const MOVE_SPEED: f32 = 200.0;

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

    fn update(&mut self, dt: f32) {
        self.entity.move_toward_target(dt);

        if !self.entity.moving && !self.follow_queue.is_empty() {
            // Keep 1 tile of distance
            if self.follow_queue.len() > 1 {
                let (nx, ny) = self.follow_queue.remove(0);
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

    loop {
        let dt = get_frame_time();
        game_time += dt;

        // Player input
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

            if moved && nx >= 0 && ny >= 0
                && (nx as usize) < map.width && (ny as usize) < map.height
                && !map.is_solid(nx as usize, ny as usize)
            {
                // Record position for Sparky before moving
                sparky.record_player_pos(player.tile_x, player.tile_y);
                player.start_move(nx as usize, ny as usize);
            }
        }

        player.move_toward_target(dt);
        sparky.update(dt);
        camera.follow(player.x, player.y, &map, GAME_W, GAME_H);

        // ─── RENDER ─────────────────────────────────────
        set_camera(&Camera2D {
            zoom: vec2(2.0 / screen_width(), 2.0 / screen_height()),
            target: vec2(camera.x + GAME_W / 2.0, camera.y + GAME_H / 2.0),
            ..Default::default()
        });

        clear_background(Color::from_rgba(26, 26, 46, 255));
        tilemap::draw_map(&map, camera.x, camera.y, GAME_W, GAME_H, game_time);

        // Draw entities in Y-order for overlap
        let mut renderables: Vec<(f32, Box<dyn Fn()>)> = vec![];
        let py = player.y;
        let pd = player.dir;
        let pf = player.frame;
        let px = player.x;
        renderables.push((py, Box::new(move || {
            sprites::player::draw_player_boy(px, py, pd, pf, 0.0);
        })));

        let sy = sparky.entity.y;
        let sx = sparky.entity.x;
        let sd = sparky.entity.dir;
        let sf = sparky.entity.frame;
        let gt = game_time;
        renderables.push((sy, Box::new(move || {
            sprites::robot::draw_robot(sx, sy, sd, sf, gt);
        })));

        renderables.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (_, draw) in &renderables {
            draw();
        }

        // HUD
        set_default_camera();
        draw_text(&format!("FPS: {} | Tile: {},{}", get_fps(), player.tile_x, player.tile_y),
            10.0, 20.0, 20.0, WHITE);

        next_frame().await
    }
}
