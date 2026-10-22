//! The descent minigame — a vertical shaft drawn as a real number line, with
//! kick buttons under it, as a Bevy UI panel. The dive rules live in
//! `robot_buddy_domain::logic::descent`.
//!
//! The shaft runs top (surface, depth 0) to bottom (the floor), with every mark
//! labelled. The trench door glows at its depth; rock shelves jut out of the
//! wall at theirs. The diver token sits at the current depth, so the kid can
//! see "I'm at 7, the door's at 12" as a distance rather than a subtraction.
//! The shaft is procedural art in an [`ArtId::DescentShaft`] slot; the words
//! and buttons are nodes.

use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::{default, AlignSelf, Bundle, ChildSpawner, Children, JustifyContent, Node, SpawnRelated, UiRect};
use bevy::text::Justify;
use bevy::ui::{percent, px, BackgroundGradient, BorderColor, ColorStop, LinearGradient};

use crate::prelude::*;
use robot_buddy_domain::logic::descent::{DiveNudge, DivePhase, DivePuzzle, DiveSession};

use super::kit::{self, backdrop, button, col, fitted, Art, Panel};
use super::views::UiViews;
use super::{ArtId, UiAction};
use crate::input::FrameInput;

/// What the dive panel shows.
#[derive(Clone, PartialEq, Debug)]
pub struct DescentView {
    pub puzzle: DivePuzzle,
    pub depth: u8,
    /// Sparky's line, or the shaft's own nudge.
    pub message: String,
}

impl DescentView {
    pub fn new(session: &DiveSession, message: Option<&str>) -> Self {
        let msg = message.unwrap_or(match session.phase {
            DivePhase::Landed => "You found the door!",
            _ => match session.nudge {
                DiveNudge::Bumped => "Bonk! That ledge won't hold you — try another way down.",
                DiveNudge::Bottomed => "That's the sea floor! Kick back up to the door.",
                DiveNudge::None => "Kick down to land right on the door.",
            },
        });
        DescentView { puzzle: session.puzzle.clone(), depth: session.depth, message: msg.to_string() }
    }
}

/// The dive panel's buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescentId {
    Sink(u8),
    Rise(u8),
    Leave,
}

pub enum DescentInput {
    Sink(u8),
    Rise(u8),
    Leave,
}

const WATER_TOP: Color = Color::new(0.13, 0.36, 0.50, 1.0);
const WATER_BOTTOM: Color = Color::new(0.03, 0.09, 0.20, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const ROCK: Color = Color::new(0.35, 0.30, 0.26, 1.0);
const EDGE: Color = Color::new(1.0, 1.0, 1.0, 0.3);

pub struct DescentPanel;

impl Panel for DescentPanel {
    type View = DescentView;
    const Z: i32 = 40;
    fn view(views: &UiViews) -> Option<&DescentView> {
        views.descent.as_ref()
    }
    fn bundle(v: &DescentView) -> impl Bundle {
        let v = v.clone();
        (backdrop(0.55, 20.0), Children::spawn(SpawnWith(move |p: &mut ChildSpawner| spawn_panel(p, &v))))
    }
}

fn spawn_panel(p: &mut ChildSpawner, v: &DescentView) {
    let panel = Node {
        width: percent(100),
        max_width: px(620),
        height: percent(100),
        max_height: px(680),
        padding: UiRect::new(px(16), px(16), px(10), px(12)),
        row_gap: px(6),
        border: UiRect::all(px(4)),
        ..col()
    };
    let water = LinearGradient::to_bottom(vec![ColorStop::auto(kit::color(WATER_TOP)), ColorStop::auto(kit::color(WATER_BOTTOM))]);
    p.spawn((panel, BackgroundGradient::from(water), BorderColor::all(kit::color(GOLD)))).with_children(|p| {
        let line = |p: &mut ChildSpawner, s: String, size: f32, c: Color| {
            p.spawn(Node { flex_shrink: 0.0, ..col() }).with_child(fitted(s, size, 12.0, true, c, Justify::Center));
        };
        line(p, "Dive to the trench!".into(), 30.0, GOLD);
        line(p, format!("The door is {} marks down.", v.puzzle.door), 22.0, WHITE);
        // The shaft takes whatever height is left.
        p.spawn((Art(ArtId::DescentShaft), Node { flex_grow: 1.0, min_height: px(0), width: percent(100), ..default() }));
        line(p, v.message.clone(), 22.0, GOLD);
        // Sink kicks above, rise kicks under.
        for down in [true, false] {
            let row = Node { column_gap: px(10), justify_content: JustifyContent::Center, flex_shrink: 0.0, ..default() };
            p.spawn(row).with_children(|p| {
                for &n in &v.puzzle.kicks {
                    kick(p, n, down);
                }
            });
        }
        let leave = Node { width: px(116), height: px(38), flex_shrink: 0.0, align_self: AlignSelf::FlexEnd, ..kit::centered() };
        p.spawn(button(UiAction::Descent(DescentId::Leave), leave, Color::new(0.329, 0.431, 0.478, 1.0), EDGE))
            .with_child(fitted("Swim up", 22.0, 14.0, false, WHITE, Justify::Center));
    });
}

fn kick(p: &mut ChildSpawner, n: u8, down: bool) {
    let (id, fill, label) = if down {
        (DescentId::Sink(n), Color::from_rgba(33, 96, 170, 255), format!("v {n}"))
    } else {
        (DescentId::Rise(n), Color::from_rgba(54, 88, 104, 255), format!("^ {n}"))
    };
    let node = Node { width: px(120), height: px(54), min_width: px(44), flex_shrink: 1.0, ..col() };
    p.spawn(button(UiAction::Descent(id), Node { justify_content: JustifyContent::Center, align_items: bevy::prelude::AlignItems::Center, ..node }, fill, EDGE))
        .with_children(|p| {
            p.spawn(fitted(label, 26.0, 14.0, false, WHITE, Justify::Center));
            // Every other kick's key is the number on the button. The 10 isn't
            // a key, so its button names the one that is.
            if let Some(hint) = key_hint(n).filter(|_| down) {
                p.spawn(fitted(hint, 15.0, 10.0, false, Color::new(1.0, 1.0, 1.0, 0.55), Justify::Center));
            }
        });
}

pub fn handle_click(input: &FrameInput) -> Option<DescentInput> {
    let Some(UiAction::Descent(id)) = input.ui_press else { return None };
    Some(match id {
        DescentId::Sink(n) => DescentInput::Sink(n),
        DescentId::Rise(n) => DescentInput::Rise(n),
        DescentId::Leave => DescentInput::Leave,
    })
}

/// The number key for a kick of `n` marks: the key with that number on it, so
/// pressing "5" always kicks 5 — the same number the button shows. The 10-kick
/// takes "0" (its last digit), and its button says so.
pub fn key_for_kick(n: u8) -> Option<KeyCode> {
    Some(match n {
        1 => KeyCode::Key1,
        2 => KeyCode::Key2,
        3 => KeyCode::Key3,
        4 => KeyCode::Key4,
        5 => KeyCode::Key5,
        6 => KeyCode::Key6,
        7 => KeyCode::Key7,
        8 => KeyCode::Key8,
        9 => KeyCode::Key9,
        10 => KeyCode::Key0,
        _ => return None,
    })
}

/// Number keys sink by the number on the key; holding Shift (or the up arrow)
/// rises, so a keyboard player can climb back out without hunting for the
/// mouse. A number the shaft doesn't offer does nothing.
pub fn handle_key(input: &FrameInput, session: &DiveSession) -> Option<DescentInput> {
    if input.pressed(KeyCode::Escape) {
        return Some(DescentInput::Leave);
    }
    let rising = input.down(KeyCode::LeftShift) || input.down(KeyCode::RightShift)
        || input.down(KeyCode::Up);
    for &n in &session.puzzle.kicks {
        if key_for_kick(n).is_some_and(|k| input.pressed(k)) {
            return Some(if rising { DescentInput::Rise(n) } else { DescentInput::Sink(n) });
        }
    }
    None
}

/// The key label for a kick whose number doesn't fit on one key ("key 0" for
/// the 10). `None` when the button's own number is the key.
fn key_hint(n: u8) -> Option<String> {
    (n == 10).then(|| "key 0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use robot_buddy_domain::logic::descent::DivePuzzle;

    fn band4() -> DiveSession {
        DiveSession::new(DivePuzzle { door: 17, floor: 24, kicks: vec![1, 2, 3, 5, 10], shelves: vec![] })
    }

    fn sink_for(k: KeyCode) -> Option<u8> {
        match handle_key(&FrameInput::empty().with_key_pressed(k), &band4()) {
            Some(DescentInput::Sink(n)) => Some(n),
            _ => None,
        }
    }

    #[test]
    fn the_number_key_kicks_the_number_on_the_button() {
        assert_eq!(sink_for(KeyCode::Key5), Some(5), "pressing 5 sinks 5, not the fifth button");
        assert_eq!(sink_for(KeyCode::Key3), Some(3));
        assert_eq!(sink_for(KeyCode::Key1), Some(1));
        assert_eq!(sink_for(KeyCode::Key0), Some(10), "0 is the 10-kick");
        assert_eq!(sink_for(KeyCode::Key4), None, "no 4-kick in this shaft");
    }

    #[test]
    fn shift_plus_a_number_rises_by_it() {
        let input = FrameInput::empty()
            .with_key_down(KeyCode::LeftShift)
            .with_key_pressed(KeyCode::Key2);
        assert!(matches!(handle_key(&input, &band4()), Some(DescentInput::Rise(2))));
    }

    #[test]
    fn only_the_ten_needs_a_key_hint() {
        assert_eq!(key_hint(10).as_deref(), Some("key 0"));
        for n in 1..=9 {
            assert_eq!(key_hint(n), None);
        }
    }
}

// ─── The shaft (art slot) ───────────────────────────────

/// Paint the shaft into its `w`×`h` slot: walls, depth marks, the door, the
/// shelves and the diver.
pub fn draw_art(v: &DescentView, w: f32, h: f32, time: f32) {
    let floor = v.puzzle.floor;
    let (sx, top, bottom) = (w * 0.42, 14.0, (h - 14.0).max(14.0));
    let y_of = |depth: u8| if floor == 0 { top } else { top + (bottom - top) * (depth as f32 / floor as f32) };

    // Shaft walls.
    draw_line(sx - 34.0, top, sx - 34.0, bottom, 3.0, Color::new(0.20, 0.28, 0.30, 1.0));
    draw_line(sx + 34.0, top, sx + 34.0, bottom, 3.0, Color::new(0.20, 0.28, 0.30, 1.0));

    // Depth marks. Every mark gets a tick; labels thin out on deep (or
    // squashed) shafts so the numbers never collide.
    let room = (bottom - top) / floor.max(1) as f32;
    let label_every = [1u8, 2, 5, 10].into_iter().find(|&k| room * k as f32 >= 20.0).unwrap_or(10);
    for d in 0..=floor {
        let y = y_of(d);
        let is_shelf = v.puzzle.is_shelf(d);
        let is_door = d == v.puzzle.door;
        let tick = if is_door { 30.0 } else { 18.0 };
        let color = if is_door { GOLD } else { Color::new(1.0, 1.0, 1.0, 0.35) };
        draw_line(sx - tick, y, sx + tick, y, if is_door { 3.0 } else { 1.5 }, color);

        if d % label_every == 0 || is_door {
            let label = format!("{d}");
            let m = measure_text(&label, None, 20, 1.0);
            draw_text(&label, sx - 34.0 - m.width - 8.0, y + 7.0, 20.0,
                if is_door { GOLD } else { Color::new(1.0, 1.0, 1.0, 0.6) });
        }
        if is_shelf {
            // A ledge you can't rest on, jutting from the wall.
            draw_rectangle(sx + 6.0, y - 5.0, 34.0, 10.0, ROCK);
            draw_rectangle(sx - 40.0, y - 4.0, 26.0, 8.0, ROCK);
        }
        if is_door {
            let pulse = (time * 3.0).sin() * 0.5 + 0.5;
            draw_circle_lines(sx, y, 20.0 + pulse * 4.0, 3.0, Color::new(0.30, 0.85, 0.90, 0.75));
            draw_text("DOOR", sx + 44.0, y + 6.0, 20.0, GOLD);
        }
    }

    // The diver.
    let dy = y_of(v.depth) + (time * 2.0).sin() * 2.0;
    draw_circle(sx, dy, 11.0, Color::from_rgba(255, 204, 128, 255));
    draw_circle_lines(sx, dy, 15.0, 2.0, Color::new(0.75, 0.95, 1.0, 0.8));
    for i in 0..3 {
        let t = (time * 1.6 + i as f32 * 0.6) % 1.0;
        draw_circle(sx + 9.0 + i as f32 * 2.0, dy - 14.0 - t * 26.0, 2.0 + t * 1.5,
            Color::new(0.8, 0.95, 1.0, 0.5 * (1.0 - t)));
    }
}
