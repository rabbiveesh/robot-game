use robot_buddy_game::prelude::*;

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
    // Load the bundled glyph font now that the GL context exists, so every
    // draw_text call (math symbols, ★, emoji) renders instead of tofu.
    robot_buddy_game::text::init_or_die();

    let seed = macroquad::rand::rand() as u64;
    let mut g = Game::new(seed);
    g.refresh_save_slots();

    robot_buddy_game::trace::init();
    let raw_input = robot_buddy_game::trace::enabled()
        .then(macroquad::input::utils::register_input_subscriber);
    let mut last_input = FrameInput::empty();

    loop {
        let dt = get_frame_time();
        robot_buddy_game::trace::next_frame();
        if let Some(sub) = raw_input {
            macroquad::input::utils::repeat_all_miniquad_input(
                &mut robot_buddy_game::trace::RawInputLog, sub);
        }
        let input = FrameInput::capture();
        if robot_buddy_game::trace::enabled() {
            trace_frame_input(&last_input, &input);
            last_input = input.clone();
        }
        let screen = (screen_width(), screen_height());
        g.step(&input, dt, screen);
        g.render(screen, &input);
        next_frame().await;
    }
}

/// One line whenever the pointer part of the frame's input is interesting:
/// a press, a release, the held state flipping, or movement while held.
fn trace_frame_input(prev: &FrameInput, now: &FrameInput) {
    let moved_while_down = now.mouse_down && now.mouse_pos != prev.mouse_pos;
    if now.mouse_clicked || now.mouse_released || now.mouse_down != prev.mouse_down || moved_while_down {
        robot_buddy_game::trace!(
            "frame clicked={} down={} released={} pos=({:.0},{:.0})",
            now.mouse_clicked, now.mouse_down, now.mouse_released, now.mouse_pos.0, now.mouse_pos.1,
        );
    }
}
