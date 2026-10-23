use bevy::prelude::*;

use robot_buddy_game::app::{clock_seed, primary_window, GamePlugin};
use robot_buddy_game::game::Game;

fn main() {
    let mut game = Game::new(clock_seed());
    game.refresh_save_slots();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(primary_window()),
            ..default()
        }))
        .insert_non_send(game)
        .add_plugins(GamePlugin)
        .run();
}
