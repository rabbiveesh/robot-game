//! The Bevy app around the pure game core.
//!
//! ```text
//!  PreUpdate (after Bevy's input)   Update                         PostUpdate
//!  ──────────────────────────────   ──────                         ──────────
//!  capture: Bevy input ──► FrameInput ──► Game::step(input, dt, screen)
//!                                         Game::render ──► draw::Cmd list ──► paint: ShapePainter
//!                                                                              + pooled Text2d
//! ```
//!
//! `Game` holds `Rc`s (the save backend), so it lives as a non-send
//! resource and every system touching it runs on the main thread.

use bevy::camera::ScalingMode;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::{FontSmoothing, LineHeight};
use bevy::window::PrimaryWindow;
use bevy_vector_shapes::prelude::*;

use crate::draw::{self, Cmd};
use crate::game::{Game, GAME_H, GAME_W};
use crate::input::{FrameInput, KeyCode as GameKey};
use crate::ui::layout::{FontMetrics, TextMetrics};

/// The input `capture` built this frame, read by `run_frame`.
#[derive(Resource, Default)]
pub struct CurrentInput(pub FrameInput);

/// Draw commands for this frame (recorded by `run_frame`), consumed by
/// `paint`. Anything may fill it: the screenshots example paints lists it
/// recorded headlessly.
#[derive(Resource, Default)]
pub struct DrawList(pub Vec<Cmd>);

/// A reusable `Text2d` for one `Cmd::Text` per frame.
#[derive(Component)]
struct PooledText;

/// Paints the [`DrawList`] every frame: camera, default font, shapes, text.
pub struct DrawPlugin;

impl Plugin for DrawPlugin {
    fn build(&self, app: &mut App) {
        // The bundled Unifont becomes Bevy's default font, so every text gets
        // −, ×, ÷, ★ and emoji without naming a font.
        app.world_mut()
            .resource_mut::<Assets<Font>>()
            .insert(AssetId::default(), Font::from_bytes(crate::text::FONT_BYTES.to_vec()))
            .expect("default font slot");
        app.add_plugins(Shape2dPlugin::default())
            .insert_resource(ClearColor(Color::srgb_u8(10, 10, 26)))
            .init_resource::<DrawList>()
            .add_systems(Startup, spawn_camera)
            .add_systems(PostUpdate, paint);
    }
}

/// Wires the `Game` (a non-send resource the app inserts) into Bevy: input
/// capture, step, render into the [`DrawList`].
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DrawPlugin)
            .init_resource::<CurrentInput>()
            .add_systems(PreUpdate, capture.after(bevy::input::InputSystems))
            .add_systems(Update, run_frame);
    }
}

/// The window the game opens in on desktop; the web build fills its page.
pub fn primary_window() -> Window {
    Window {
        title: "Robot Buddy Adventure".into(),
        resolution: (GAME_W as u32, GAME_H as u32).into(),
        canvas: Some("#bevy".into()),
        fit_canvas_to_parent: true,
        prevent_default_event_handling: true,
        ..default()
    }
}

/// A seed from the wall clock, so each launch plays differently.
pub fn clock_seed() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64 ^ (js_sys::Math::random() * u32::MAX as f64) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42)
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::WindowSize,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Screen size in logical pixels (what the game lays out in).
fn screen_of(window: &Window) -> (f32, f32) {
    (window.width(), window.height())
}

fn capture(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut typed: MessageReader<KeyboardInput>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut out: ResMut<CurrentInput>,
) {
    let mut input = FrameInput::empty();
    for k in keys.get_just_pressed().filter_map(|&k| GameKey::from_bevy(k)) {
        input = input.with_key_pressed(k);
    }
    for k in keys.get_pressed().filter_map(|&k| GameKey::from_bevy(k)) {
        input = input.with_key_down(k);
    }
    for ev in typed.read() {
        if ev.state.is_pressed() {
            for c in ev.text.iter().flat_map(|t| t.chars()).filter(|c| !c.is_control()) {
                input = input.with_char(c);
            }
        }
    }
    // A finger landing counts as a click where it landed; between taps the
    // pointer stays where the last one was.
    let last = out.0.mouse_pos;
    let cursor = window.single().ok().and_then(|w| w.cursor_position());
    input.mouse_pos = cursor.map_or(last, |p| (p.x, p.y));
    if mouse.just_pressed(MouseButton::Left) {
        input.mouse_clicked = true;
    } else if let Some(t) = touches.iter_just_pressed().next() {
        let p = t.position();
        input = input.with_mouse_click(p.x, p.y);
    }
    out.0 = input;
}

fn run_frame(
    mut game: NonSendMut<Game>,
    input: Res<CurrentInput>,
    time: Res<Time>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut list: ResMut<DrawList>,
) {
    let Ok(window) = window.single() else { return };
    let screen = screen_of(window);
    game.step(&input.0, time.delta_secs(), screen);
    draw::begin_frame(screen, time.elapsed_secs_f64());
    game.render(screen, &input.0);
    list.0 = draw::take_commands();
}

/// Painter's order: each command sits a hair in front of the one before.
const Z_START: f32 = -900.0;
const Z_STEP: f32 = 0.01;

fn paint(
    mut commands: Commands,
    mut painter: ShapePainter,
    mut list: ResMut<DrawList>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut pool: Query<
        (&mut Text2d, &mut TextFont, &mut TextColor, &mut Transform, &mut Visibility),
        With<PooledText>,
    >,
) {
    let Ok(window) = window.single() else { return };
    let (sw, sh) = screen_of(window);
    // Top-left, y-down pixels → Bevy world (origin at the screen's center, y up).
    let to_world = |x: f32, y: f32| Vec2::new(x - sw / 2.0, sh / 2.0 - y);
    let metrics = FontMetrics::bundled();

    let cmds = std::mem::take(&mut list.0);
    let mut slots = pool.iter_mut();
    let mut z = Z_START;
    for cmd in &cmds {
        z += Z_STEP;
        painter.reset();
        painter.cap = Cap::None;
        match cmd {
            Cmd::Rect { x, y, w, h, color } => {
                painter.set_translation(to_world(x + w / 2.0, y + h / 2.0).extend(z));
                painter.set_color(bevy_color(*color));
                painter.rect(Vec2::new(*w, *h));
            }
            Cmd::RectLines { x, y, w, h, thickness, color } => {
                painter.set_translation(to_world(x + w / 2.0, y + h / 2.0).extend(z));
                painter.set_color(bevy_color(*color));
                painter.hollow = true;
                painter.thickness = *thickness;
                painter.rect(Vec2::new(*w, *h));
            }
            Cmd::Circle { x, y, r, color } => {
                painter.set_translation(to_world(*x, *y).extend(z));
                painter.set_color(bevy_color(*color));
                painter.circle(*r);
            }
            Cmd::CircleLines { x, y, r, thickness, color } => {
                // The ring straddles the radius, as macroquad drew it.
                painter.set_translation(to_world(*x, *y).extend(z));
                painter.set_color(bevy_color(*color));
                painter.hollow = true;
                painter.thickness = *thickness;
                painter.circle(r + thickness / 2.0);
            }
            Cmd::Ellipse { x, y, rx, ry, rotation, color } => {
                if *rx <= 0.0 || *ry <= 0.0 {
                    continue;
                }
                painter.set_translation(to_world(*x, *y).extend(z));
                painter.set_rotation(Quat::from_rotation_z(-rotation.to_radians()));
                painter.set_scale(Vec3::new(1.0, ry / rx, 1.0));
                painter.set_color(bevy_color(*color));
                painter.circle(*rx);
            }
            Cmd::Line { x1, y1, x2, y2, thickness, color } => {
                painter.set_translation(Vec3::new(0.0, 0.0, z));
                painter.set_color(bevy_color(*color));
                painter.thickness = *thickness;
                painter.line(to_world(*x1, *y1).extend(0.0), to_world(*x2, *y2).extend(0.0));
            }
            Cmd::Triangle { a, b, c, color } => {
                painter.set_translation(Vec3::new(0.0, 0.0, z));
                painter.set_color(bevy_color(*color));
                painter.triangle(to_world(a.x, a.y), to_world(b.x, b.y), to_world(c.x, c.y));
            }
            Cmd::Text { text, x, y, size, color } => {
                // Anchored at the top-left of a one-em line box, so the
                // baseline lands `ascent` below it, where macroquad put it.
                let top = to_world(*x, y - metrics.ascent(*size));
                let font = TextFont::from_font_size(*size as f32).with_font_smoothing(FontSmoothing::AntiAliased);
                let tf = Transform::from_translation(top.extend(z));
                match slots.next() {
                    Some((mut t, mut f, mut c, mut transform, mut vis)) => {
                        if t.0 != *text {
                            t.0.clone_from(text);
                        }
                        f.set_if_neq(font);
                        c.set_if_neq(TextColor(bevy_color(*color)));
                        transform.set_if_neq(tf);
                        vis.set_if_neq(Visibility::Inherited);
                    }
                    None => {
                        commands.spawn((
                            PooledText,
                            Text2d::new(text.clone()),
                            font,
                            LineHeight::RelativeToFont(1.0),
                            TextColor(bevy_color(*color)),
                            Anchor::TOP_LEFT,
                            tf,
                        ));
                    }
                }
            }
        }
    }
    for (.., mut vis) in slots {
        vis.set_if_neq(Visibility::Hidden);
    }
}

fn bevy_color(c: draw::Color) -> Color {
    Color::srgba(c.r, c.g, c.b, c.a)
}
