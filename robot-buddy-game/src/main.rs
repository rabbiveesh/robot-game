use macroquad::prelude::*;

mod tilemap;
mod sprites;
mod npc;
mod ui;
mod save;
mod audio;
mod session;
mod settings;
mod input;
mod game;

use input::FrameInput;
use game::{Game, GAME_W, GAME_H};

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
    let seed = macroquad::rand::rand() as u64;
    let mut g = Game::new(seed);

    loop {
        let dt = get_frame_time();
        let input = FrameInput::capture();
        g.step(&input, dt);
        next_frame().await;
    }
}
