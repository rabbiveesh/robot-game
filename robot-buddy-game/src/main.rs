use macroquad::prelude::*;

mod tilemap;

use tilemap::{Map, TILE_SIZE};

const GAME_W: f32 = 960.0;
const GAME_H: f32 = 720.0;
const MOVE_SPEED: f32 = 200.0; // pixels per second

struct Player {
    x: f32,
    y: f32,
    tile_x: usize,
    tile_y: usize,
    // Smooth movement
    target_x: f32,
    target_y: f32,
    moving: bool,
}

impl Player {
    fn new(tile_x: usize, tile_y: usize) -> Self {
        Player {
            x: tile_x as f32 * TILE_SIZE,
            y: tile_y as f32 * TILE_SIZE,
            tile_x,
            tile_y,
            target_x: tile_x as f32 * TILE_SIZE,
            target_y: tile_y as f32 * TILE_SIZE,
            moving: false,
        }
    }

    fn update(&mut self, dt: f32, map: &Map) {
        // Smooth interpolation to target
        if self.moving {
            let dx = self.target_x - self.x;
            let dy = self.target_y - self.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 2.0 {
                self.x = self.target_x;
                self.y = self.target_y;
                self.moving = false;
            } else {
                let step = MOVE_SPEED * dt;
                self.x += dx / dist * step;
                self.y += dy / dist * step;
            }
            return; // don't accept new input while moving
        }

        // Grid-based movement
        let mut nx = self.tile_x as i32;
        let mut ny = self.tile_y as i32;

        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) { ny -= 1; }
        else if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) { ny += 1; }
        else if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) { nx -= 1; }
        else if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) { nx += 1; }

        if (nx != self.tile_x as i32 || ny != self.tile_y as i32)
            && nx >= 0 && ny >= 0
            && (nx as usize) < map.width && (ny as usize) < map.height
            && !map.is_solid(nx as usize, ny as usize)
        {
            self.tile_x = nx as usize;
            self.tile_y = ny as usize;
            self.target_x = self.tile_x as f32 * TILE_SIZE;
            self.target_y = self.tile_y as f32 * TILE_SIZE;
            self.moving = true;
        }
    }

    fn draw(&self) {
        // Simple colored rectangle for now — sprites come in step 3
        draw_rectangle(self.x + 4.0, self.y + 4.0, TILE_SIZE - 8.0, TILE_SIZE - 8.0,
            Color::from_rgba(0, 230, 118, 255));
        // Eyes
        draw_rectangle(self.x + 14.0, self.y + 12.0, 6.0, 6.0, WHITE);
        draw_rectangle(self.x + 28.0, self.y + 12.0, 6.0, 6.0, WHITE);
        draw_rectangle(self.x + 16.0, self.y + 14.0, 3.0, 3.0, BLACK);
        draw_rectangle(self.x + 30.0, self.y + 14.0, 3.0, 3.0, BLACK);
    }
}

struct Camera {
    x: f32,
    y: f32,
}

impl Camera {
    fn follow(&mut self, target_x: f32, target_y: f32, map: &Map, view_w: f32, view_h: f32) {
        // Center on target
        self.x = target_x - view_w / 2.0 + TILE_SIZE / 2.0;
        self.y = target_y - view_h / 2.0 + TILE_SIZE / 2.0;
        // Clamp to map bounds
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
    let mut player = Player::new(14, 12);
    let mut camera = Camera { x: 0.0, y: 0.0 };
    let mut game_time: f32 = 0.0;

    loop {
        let dt = get_frame_time();
        game_time += dt;

        // Update
        player.update(dt, &map);
        camera.follow(player.x, player.y, &map, GAME_W, GAME_H);

        // Set up world camera
        set_camera(&Camera2D {
            zoom: vec2(2.0 / screen_width(), -2.0 / screen_height()),
            target: vec2(camera.x + GAME_W / 2.0, camera.y + GAME_H / 2.0),
            ..Default::default()
        });

        clear_background(Color::from_rgba(26, 26, 46, 255));

        // Draw map (only visible tiles)
        tilemap::draw_map(&map, camera.x, camera.y, GAME_W, GAME_H, game_time);

        // Draw player
        player.draw();

        // HUD (screen space)
        set_default_camera();
        draw_text(&format!("FPS: {} | Tile: {},{}", get_fps(), player.tile_x, player.tile_y),
            10.0, 20.0, 20.0, WHITE);

        next_frame().await
    }
}
