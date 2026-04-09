use macroquad::prelude::*;

mod tilemap;

const TILE_SIZE: f32 = 48.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Robot Buddy Adventure".to_string(),
        window_width: 960,
        window_height: 720,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let map = tilemap::Map::overworld();

    loop {
        clear_background(Color::from_rgba(26, 26, 46, 255));

        // Draw the tile map
        for row in 0..map.height {
            for col in 0..map.width {
                let tile_id = map.tiles[row][col];
                let color = tilemap::tile_color(tile_id);
                let x = col as f32 * TILE_SIZE;
                let y = row as f32 * TILE_SIZE;
                draw_rectangle(x, y, TILE_SIZE, TILE_SIZE, color);

                // Tile borders for visibility
                draw_rectangle_lines(x, y, TILE_SIZE, TILE_SIZE, 1.0,
                    Color::from_rgba(0, 0, 0, 30));
            }
        }

        // FPS counter
        draw_text(&format!("FPS: {}", get_fps()), 10.0, 20.0, 20.0, WHITE);

        next_frame().await
    }
}
