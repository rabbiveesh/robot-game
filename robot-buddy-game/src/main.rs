use macroquad::prelude::*;

mod tilemap;

const TILE_SIZE: f32 = 48.0;
const GAME_W: f32 = 960.0;
const GAME_H: f32 = 720.0;

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
    let map = tilemap::Map::overworld();

    // Camera centered on the map, showing GAME_W x GAME_H world units
    let cam_x = GAME_W / 2.0;
    let cam_y = GAME_H / 2.0;

    loop {
        // Scale tiles to fit the screen
        let sw = screen_width();
        let sh = screen_height();
        let map_w = map.width as f32 * TILE_SIZE;
        let map_h = map.height as f32 * TILE_SIZE;
        let scale = (sw / map_w).min(sh / map_h);
        let offset_x = (sw - map_w * scale) / 2.0;
        let offset_y = (sh - map_h * scale) / 2.0;
        let ts = TILE_SIZE * scale;

        clear_background(Color::from_rgba(26, 26, 46, 255));

        // Draw the tile map scaled to fit
        for row in 0..map.height {
            for col in 0..map.width {
                let tile_id = map.tiles[row][col];
                let color = tilemap::tile_color(tile_id);
                let x = offset_x + col as f32 * ts;
                let y = offset_y + row as f32 * ts;
                draw_rectangle(x, y, ts, ts, color);
            }
        }

        draw_text(&format!("FPS: {} | {}x{}", get_fps(), sw as i32, sh as i32),
            10.0, 20.0, 20.0, WHITE);

        next_frame().await
    }
}
