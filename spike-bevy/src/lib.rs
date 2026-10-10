//! Spike: the game's logic as a Bevy plugin with no rendering, so it runs
//! under `MinimalPlugins` in tests and under `DefaultPlugins` in `main.rs`.
//!
//! Proves three things the port needs:
//! - the domain crate plugs in unchanged (challenge generation, seeded RNG);
//! - Bevy `States` replace the hand-wired `GameState` + `set_state`;
//! - a challenge panel is Bevy UI (taffy) with picking, and the click path is
//!   an entity event a headless test can fire the same way a tap does.

use bevy::prelude::*;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use robot_buddy_domain::learning::challenge_generator::{generate_challenge, Challenge, ChallengeProfile};
use robot_buddy_domain::learning::operation_stats::OperationStats;

pub const MAP_W: i32 = 10;
pub const MAP_H: i32 = 8;
pub const TILE: f32 = 48.0;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    #[default]
    Exploring,
    Challenge,
}

/// The assertion surface, same idea as `GameEvent` in the real game.
#[derive(Debug, Clone, PartialEq)]
pub enum SpikeEvent {
    ChallengeStarted { question: String },
    ChallengeResolved { correct: bool },
}

#[derive(Resource, Default)]
pub struct EventLog(pub Vec<SpikeEvent>);

#[derive(Resource)]
pub struct GameRng(pub SmallRng);

#[derive(Resource, Default)]
pub struct CurrentChallenge(pub Option<Challenge>);

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct TilePos { pub x: i32, pub y: i32 }

#[derive(Component)]
pub struct Player;

/// Sparky stands still in the spike; walking into him starts a challenge.
#[derive(Component)]
pub struct Sparky;

#[derive(Component)]
pub struct ChoiceButton { pub correct: bool }

/// "That's not it" line — a gentle in-world nudge, never a red X (Invariant 7).
#[derive(Component)]
pub struct Nudge;

/// Fired on a choice button when it's tapped. Picking triggers it in the real
/// app; tests trigger it directly, so both go through `on_choice_picked`.
#[derive(EntityEvent)]
pub struct ChoicePicked { pub entity: Entity }

pub struct SpikeLogicPlugin { pub seed: u64 }

impl Plugin for SpikeLogicPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Mode>()
            .insert_resource(GameRng(SmallRng::seed_from_u64(self.seed)))
            .init_resource::<EventLog>()
            .init_resource::<CurrentChallenge>()
            .add_systems(Startup, spawn_actors)
            .add_systems(Update, walk.run_if(in_state(Mode::Exploring)))
            .add_systems(OnEnter(Mode::Challenge), open_challenge)
            .add_observer(on_choice_picked);
    }
}

fn spawn_actors(mut commands: Commands) {
    commands.spawn((Player, TilePos { x: 2, y: 4 }));
    commands.spawn((Sparky, TilePos { x: 6, y: 4 }));
}

fn walk(
    keys: Res<ButtonInput<KeyCode>>,
    mut player: Query<&mut TilePos, (With<Player>, Without<Sparky>)>,
    sparky: Query<&TilePos, With<Sparky>>,
    mut next: ResMut<NextState<Mode>>,
) {
    let (dx, dy) = if keys.just_pressed(KeyCode::ArrowRight) { (1, 0) }
        else if keys.just_pressed(KeyCode::ArrowLeft) { (-1, 0) }
        else if keys.just_pressed(KeyCode::ArrowUp) { (0, -1) }
        else if keys.just_pressed(KeyCode::ArrowDown) { (0, 1) }
        else { return };
    let mut p = player.single_mut().unwrap();
    let to = TilePos { x: (p.x + dx).clamp(0, MAP_W - 1), y: (p.y + dy).clamp(0, MAP_H - 1) };
    if sparky.iter().any(|s| *s == to) {
        next.set(Mode::Challenge); // bumping Sparky = talking to him
    } else {
        *p = to;
    }
}

fn open_challenge(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    mut current: ResMut<CurrentChallenge>,
    mut log: ResMut<EventLog>,
) {
    let profile = ChallengeProfile { math_band: 2, spread_width: 0.5, operation_stats: OperationStats::new() };
    let ch = generate_challenge(&profile, &mut rng.0);
    log.0.push(SpikeEvent::ChallengeStarted { question: ch.question.clone() });

    // The whole panel: flexbox via taffy, no hand-written layout engine.
    commands
        .spawn((
            DespawnOnExit(Mode::Challenge),
            Node {
                width: percent(100), height: percent(100),
                align_items: AlignItems::End, justify_content: JustifyContent::Center,
                padding: UiRect::all(px(16)),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: percent(100), max_width: px(560),
                    flex_direction: FlexDirection::Column, row_gap: px(12),
                    padding: UiRect::all(px(16)), align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.2, 0.92)),
            ))
            .with_children(|panel| {
                panel.spawn((Text::new(format!("Sparky asks: {}", ch.display_text)), TextFont::from_font_size(28.0)));
                panel.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(12), flex_wrap: FlexWrap::Wrap, justify_content: JustifyContent::Center, ..default() })
                    .with_children(|row| {
                        for c in &ch.choices {
                            row.spawn((
                                Button,
                                ChoiceButton { correct: c.correct },
                                Node { min_width: px(88), min_height: px(64), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border_radius: BorderRadius::all(px(10)), ..default() },
                                BackgroundColor(Color::srgb(0.25, 0.45, 0.85)),
                            ))
                            .with_child((Text::new(c.text.clone()), TextFont::from_font_size(32.0)))
                            // A tap (mouse or touch) becomes the same event tests fire.
                            .observe(|click: On<Pointer<Click>>, mut commands: Commands| {
                                commands.trigger(ChoicePicked { entity: click.entity });
                            });
                        }
                    });
            });
        });
    current.0 = Some(ch);
}

fn on_choice_picked(
    picked: On<ChoicePicked>,
    buttons: Query<&ChoiceButton>,
    nudges: Query<(), With<Nudge>>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    mut current: ResMut<CurrentChallenge>,
    mut next: ResMut<NextState<Mode>>,
) {
    let Ok(button) = buttons.get(picked.entity) else { return };
    log.0.push(SpikeEvent::ChallengeResolved { correct: button.correct });
    if button.correct {
        current.0 = None;
        next.set(Mode::Exploring);
    } else if nudges.is_empty() {
        commands.spawn((
            Nudge,
            DespawnOnExit(Mode::Challenge),
            Text::new("Hmm, Sparky's battery flickers. Try another!"),
            Node { position_type: PositionType::Absolute, top: px(16), left: px(16), ..default() },
        ));
    }
}
