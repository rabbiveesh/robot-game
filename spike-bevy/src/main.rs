//! Spike binary: the logic plugin plus rendering. `SPIKE_SHOT=out.png` takes a
//! screenshot after a scripted walk into Sparky, then exits.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy_vector_shapes::prelude::*;
use spike_bevy::*;

mod mq;

const VIEW_W: f32 = MAP_W as f32 * TILE;
const VIEW_H: f32 = MAP_H as f32 * TILE;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Robot Buddy (Bevy spike)".into(),
                resolution: (VIEW_W as u32, VIEW_H as u32).into(),
                canvas: Some("#bevy".into()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(Shape2dPlugin::default())
        .add_plugins(SpikeLogicPlugin { seed: 42 })
        .add_systems(Startup, setup)
        .add_systems(Update, (sync_tiles, draw_sparky))
        .add_systems(PreUpdate, shot_script.after(bevy::input::InputSystems))
        .run();
}

fn tile_to_world(x: f32, y: f32) -> Vec2 {
    Vec2::new(x * TILE - VIEW_W / 2.0 + TILE / 2.0, VIEW_H / 2.0 - y * TILE - TILE / 2.0)
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            let edge = x == 0 || y == 0 || x == MAP_W - 1 || y == MAP_H - 1;
            let color = if edge { Color::srgb_u8(46, 125, 50) } else if (x + y) % 5 == 0 { Color::srgb_u8(129, 199, 132) } else { Color::srgb_u8(102, 187, 106) };
            commands.spawn((
                Sprite::from_color(color, Vec2::splat(TILE)),
                Transform::from_translation(tile_to_world(x as f32, y as f32).extend(0.0)),
            ));
        }
    }
}

/// Player is a plain sprite; Transform follows the logical tile. z from y is
/// the whole y-sort.
fn sync_tiles(mut commands: Commands, q: Query<(Entity, &TilePos, Option<&Transform>), With<Player>>) {
    for (e, tp, tf) in &q {
        let p = tile_to_world(tp.x as f32, tp.y as f32);
        let t = Transform::from_translation(p.extend(10.0 - p.y * 0.001));
        if tf.is_none() {
            commands.entity(e).insert((Sprite::from_color(Color::srgb_u8(255, 167, 38), Vec2::new(28.0, 40.0)), t));
        } else {
            commands.entity(e).insert(t);
        }
    }
}

/// Sparky, drawn by the macroquad sprite function ported line for line.
fn draw_sparky(mut painter: ShapePainter, time: Res<Time>, q: Query<&TilePos, With<Sparky>>) {
    let Ok(tp) = q.single() else { return };
    let mut mq = mq::Mq::new(&mut painter, VIEW_W, VIEW_H);
    mq::draw_robot(&mut mq, tp.x as f32 * TILE, tp.y as f32 * TILE, time.elapsed_secs());
}

/// Screenshot mode for headless checks: walk into Sparky, wait for the panel
/// to lay out, save a PNG, quit.
fn shot_script(
    mut frame: Local<u32>,
    mut commands: Commands,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(path) = std::env::var("SPIKE_SHOT") else { return };
    *frame += 1;
    match *frame {
        10 | 20 | 30 | 40 => keys.press(KeyCode::ArrowRight),
        11 | 21 | 31 | 41 => keys.release(KeyCode::ArrowRight),
        90 => { commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path)); }
        120 => { exit.write(AppExit::Success); }
        _ => {}
    }
}
