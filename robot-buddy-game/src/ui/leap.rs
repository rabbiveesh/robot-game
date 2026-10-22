//! Shelly's leap panel — the strip along the bottom of the reef while a pearl
//! trip is going, as a Bevy UI panel. The rules live in
//! `robot_buddy_domain::logic::leap`.
//!
//! It shows her call ("stone 12, four leaps") and the leap sizes on offer as
//! big tappable tiles, then swaps to a Leap / Swim back pair once the kid has
//! committed. Everything a kid needs to see stays on screen — the clue is never
//! a toast they can miss.

use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::{default, Bundle, ChildSpawner, Children, JustifyContent, Node, PositionType, SpawnRelated, UiRect};
use bevy::text::Justify;
use bevy::ui::{percent, px, BackgroundColor, BorderColor};

use crate::prelude::*;
use robot_buddy_domain::logic::leap::{LeapPhase, LeapSession};

use super::kit::{self, button, col, fitted, Panel, Tint};
use super::views::UiViews;
use super::UiAction;
use crate::input::FrameInput;

/// What the strip shows: Shelly's call, a status line, and the buttons.
#[derive(Clone, PartialEq, Debug)]
pub struct LeapView {
    pub call: String,
    pub status: String,
    /// Sizes on offer, until one is picked.
    pub choices: Vec<u8>,
    /// The size the kid committed to (shows Leap / Swim back).
    pub chosen: Option<u8>,
}

impl LeapView {
    pub fn new(session: &LeapSession, call: String) -> Self {
        let status = match session.phase {
            LeapPhase::Choosing => "How big is each leap?".to_string(),
            LeapPhase::Overshot => format!(
                "Stone {} — past the pearl! Swim back and pick again.", session.position,
            ),
            _ => match session.next_stone() {
                Some(next) => format!(
                    "On stone {}. Leaping by {}s — next stop {next}.",
                    session.position,
                    session.chosen.unwrap_or(0),
                ),
                None => format!("On stone {}.", session.position),
            },
        };
        let choices = if session.chosen.is_none() { session.puzzle.choices.clone() } else { vec![] };
        LeapView { call, status, choices, chosen: session.chosen }
    }
}

/// The leap panel's buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeapId {
    /// A size tile; carries the size it commits to.
    Size(u8),
    Leap,
    SwimBack,
}

pub enum LeapInput {
    Pick(u8),
    Leap,
    SwimBack,
}

const PANEL_BG: Color = Color::new(0.043, 0.157, 0.216, 0.92);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const EDGE: Color = Color::new(1.0, 1.0, 1.0, 0.35);

pub struct LeapPanel;

impl Panel for LeapPanel {
    type View = LeapView;
    const Z: i32 = 10;
    fn view(views: &UiViews) -> Option<&LeapView> {
        views.leap.as_ref()
    }
    fn bundle(v: &LeapView) -> impl Bundle {
        let v = v.clone();
        let strip = Node {
            position_type: PositionType::Absolute,
            bottom: px(0),
            width: percent(100),
            padding: UiRect::new(px(16), px(16), px(10), px(14)),
            row_gap: px(6),
            border: UiRect::top(px(3)),
            ..col()
        };
        (
            strip,
            BackgroundColor(kit::color(PANEL_BG)),
            BorderColor::all(kit::color(Color::new(0.30, 0.85, 0.90, 0.7))),
            Children::spawn(SpawnWith(move |p: &mut ChildSpawner| spawn_strip(p, &v))),
        )
    }
}

fn spawn_strip(p: &mut ChildSpawner, v: &LeapView) {
    // Shelly's call is a full sentence: it shrinks on a narrow window.
    p.spawn(Node { flex_shrink: 0.0, ..col() }).with_child(fitted(v.call.clone(), 24.0, 12.0, true, GOLD, Justify::Center));
    p.spawn(Node { flex_shrink: 0.0, ..col() })
        .with_child(fitted(v.status.clone(), 19.0, 12.0, true, Color::new(1.0, 1.0, 1.0, 0.8), Justify::Center));
    let row = Node { column_gap: px(12), justify_content: JustifyContent::Center, margin: UiRect::top(px(4)), ..default() };
    p.spawn(row).with_children(|p| {
        for (i, &size) in v.choices.iter().enumerate() {
            tile(p, LeapId::Size(size), 96.0, format!("[{}] {size}", i + 1), 28.0, (33, 96, 170), (50, 140, 190));
        }
        if let Some(size) = v.chosen {
            tile(p, LeapId::Leap, 150.0, format!("Leap {size} >"), 26.0, (42, 130, 92), (60, 170, 120));
            tile(p, LeapId::SwimBack, 150.0, "< Swim back".into(), 26.0, (66, 84, 100), (90, 110, 130));
        }
    });
}

fn tile(p: &mut ChildSpawner, id: LeapId, w: f32, label: String, size: f32, idle: (u8, u8, u8), hot: (u8, u8, u8)) {
    let rgb = |(r, g, b): (u8, u8, u8)| Color::from_rgba(r, g, b, 255);
    // Tiles share the row when it's narrow, down to a thumb's width.
    let node = Node { width: px(w), height: px(54), min_width: px(44), flex_shrink: 1.0, ..kit::centered() };
    p.spawn((
        button(UiAction::Leap(id), node, rgb(idle), EDGE),
        Tint { idle: kit::color(rgb(idle)), hot: kit::color(rgb(hot)) },
    ))
    .with_child(fitted(label, size, 14.0, false, WHITE, Justify::Center));
}

pub fn handle_click(input: &FrameInput) -> Option<LeapInput> {
    let Some(UiAction::Leap(id)) = input.ui_press else { return None };
    Some(match id {
        LeapId::Size(n) => LeapInput::Pick(n),
        LeapId::Leap => LeapInput::Leap,
        LeapId::SwimBack => LeapInput::SwimBack,
    })
}
