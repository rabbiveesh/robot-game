use macroquad::prelude::*;

use robot_buddy_game::input::FrameInput;
use robot_buddy_game::game::{Game, GAME_W, GAME_H};

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
        let screen = (screen_width(), screen_height());
        g.step(&input, dt, screen);
        g.render(screen, &input);
        next_frame().await;
    }
}
