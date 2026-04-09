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
        // Set up camera so 1 world unit = 1 pixel at our target resolution
        set_camera(&Camera2D {
            zoom: vec2(2.0 / screen_width(), -2.0 / screen_height()),
            target: vec2(cam_x, cam_y),
            ..Default::default()
        });

        clear_background(Color::from_rgba(26, 26, 46, 255));

        // Draw the tile map
        for row in 0..map.height {
            for col in 0..map.width {
                let tile_id = map.tiles[row][col];
                let color = tilemap::tile_color(tile_id);
                let x = col as f32 * TILE_SIZE;
                let y = row as f32 * TILE_SIZE;
                draw_rectangle(x, y, TILE_SIZE, TILE_SIZE, color);
            }
        }

        // HUD: switch to screen-space for overlays
        set_default_camera();
        draw_text(&format!("FPS: {}", get_fps()), 10.0, 20.0, 20.0, WHITE);

        next_frame().await
    }
}
