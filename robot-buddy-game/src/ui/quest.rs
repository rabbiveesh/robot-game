//! Quest runner UI, as a Bevy UI panel. Opt-in (feature-flagged) while the
//! quest path is being playtested.
//!
//! Self-contained: a quest plays as a sequence of narrative beats and inline
//! multiple-choice math moments, so it never has to hand control to the
//! challenge state and come back. The game builds a `QuestView` from the domain
//! `QuestSession` (+ the generated puzzle choices) each frame; this module
//! turns it into nodes and its taps back into a [`QuestClick`].

use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::{default, AlignSelf, Bundle, ChildSpawner, Children, JustifyContent, Node, SpawnRelated, UiRect, Val};
use bevy::text::Justify;
use bevy::ui::{percent, px, BackgroundColor, BorderColor};

use crate::prelude::*;

use super::kit::{self, backdrop, button, col, fitted, Panel};
use super::views::UiViews;
use super::UiAction;
use crate::input::FrameInput;
use robot_buddy_domain::economy::shop::Currency;

/// What the current quest step looks like to the player. Built by the game from
/// the domain step (+ generated puzzle choices for a MathPuzzle).
#[derive(Clone, PartialEq, Debug)]
pub enum QuestView {
    Narrative { speaker: String, lines: Vec<String> },
    Travel { label: String },
    Puzzle { prompt: String, choices: Vec<i32> },
    /// A branching decision — the player taps one of the labelled options.
    Choice { prompt: String, options: Vec<String> },
    Reward { dum_dums: u32 },
}

impl QuestView {
    /// Narrative / travel / reward beats (and a degenerate option-less Choice,
    /// so it can never soft-lock) advance with a Continue button.
    pub fn has_continue(&self) -> bool {
        match self {
            QuestView::Puzzle { .. } => false,
            QuestView::Choice { options, .. } => options.is_empty(),
            _ => true,
        }
    }

    fn body(&self) -> String {
        match self {
            QuestView::Narrative { speaker, lines } => format!("{speaker}: {}", lines.join(" ")),
            QuestView::Travel { label } => label.clone(),
            QuestView::Puzzle { prompt, .. } | QuestView::Choice { prompt, .. } => prompt.to_string(),
            QuestView::Reward { dum_dums } => format!("You earned {}!", Currency::DumDums.count(*dum_dums)),
        }
    }
}

/// The panel: the beat, plus the quest's title and Sparky's last remark.
#[derive(Clone, PartialEq, Debug)]
pub struct QuestPanelView {
    pub view: QuestView,
    pub title: String,
    pub message: Option<String>,
    pub screen_h: f32,
}

/// The quest panel's buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestId {
    Answer(usize),
    Option(usize),
    Continue,
}

pub enum QuestClick {
    Continue,
    Answer(i32),
    Choose(usize),
}

const PANEL_W: f32 = 720.0;
const PANEL_H: f32 = 420.0;
const MARGIN: f32 = 20.0;

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const TILE_BG: Color = Color::new(0.129, 0.588, 0.953, 1.0);
const TILE_EDGE: Color = Color::new(1.0, 1.0, 1.0, 0.4);

pub struct QuestPanel;

impl Panel for QuestPanel {
    type View = QuestPanelView;
    const Z: i32 = 40;
    fn view(views: &UiViews) -> Option<&QuestPanelView> {
        views.quest.as_ref()
    }
    fn bundle(v: &QuestPanelView) -> impl Bundle {
        let v = v.clone();
        (backdrop(0.5, MARGIN), Children::spawn(SpawnWith(move |p: &mut ChildSpawner| spawn_panel(p, &v))))
    }
}

/// The panel grows with a long body (up to the screen).
fn spawn_panel(p: &mut ChildSpawner, v: &QuestPanelView) {
    let panel = Node {
        width: percent(100),
        max_width: px(PANEL_W),
        min_height: px(PANEL_H.min(v.screen_h - 2.0 * MARGIN)),
        max_height: percent(100),
        padding: UiRect::new(px(28), px(28), px(14), px(20)),
        row_gap: px(12),
        border: UiRect::all(px(4)),
        ..col()
    };
    p.spawn((panel, BackgroundColor(kit::color(DARK_BG)), BorderColor::all(kit::color(GOLD)))).with_children(|p| {
        p.spawn(Node { flex_shrink: 0.0, ..col() }).with_child(fitted(v.title.clone(), 26.0, 16.0, false, GOLD, Justify::Center));
        p.spawn(kit::filled(v.view.body(), 24.0, 14.0, WHITE, Justify::Left));
        // Reserved even when empty, so the buttons don't jump on a retry.
        // Sits at the bottom with the buttons.
        p.spawn(Node { height: px(30), flex_shrink: 0.0, margin: UiRect::top(Val::Auto), ..col() })
            .with_child(kit::filled(v.message.clone().unwrap_or_default(), 22.0, 12.0, GOLD, Justify::Center));
        actions(p, &v.view);
    });
}

fn tile(p: &mut ChildSpawner, id: QuestId, node: Node, label: String, size: f32, min: f32, wrap: bool) {
    p.spawn(button(UiAction::Quest(id), Node { ..node }, TILE_BG, TILE_EDGE))
        .with_child(fitted(label, size, min, wrap, WHITE, Justify::Center));
}

fn actions(p: &mut ChildSpawner, view: &QuestView) {
    match view {
        QuestView::Puzzle { choices, .. } => {
            let row = Node { column_gap: px(18), justify_content: JustifyContent::Center, min_height: px(60), flex_shrink: 0.0, ..default() };
            p.spawn(row).with_children(|p| {
                for (i, v) in choices.iter().enumerate() {
                    let node = Node { width: px(84), height: px(84), min_width: px(48), ..kit::centered() };
                    tile(p, QuestId::Answer(i), node, v.to_string(), 34.0, 18.0, false);
                }
            });
        }
        QuestView::Choice { options, .. } if !options.is_empty() => {
            // Stacked full-width buttons.
            p.spawn(Node { row_gap: px(12), flex_shrink: 0.0, ..col() }).with_children(|p| {
                for (i, label) in options.iter().enumerate() {
                    let node = Node { height: px(46), min_height: px(36), ..kit::centered() };
                    tile(p, QuestId::Option(i), node, format!("{}. {}", i + 1, label), 22.0, 14.0, true);
                }
            });
        }
        _ => {
            let node = Node { width: px(180), height: px(44), flex_shrink: 0.0, align_self: AlignSelf::Center, ..kit::centered() };
            tile(p, QuestId::Continue, node, "Continue".into(), 22.0, 16.0, false);
        }
    }
}

pub fn handle_click(input: &FrameInput, view: &QuestView) -> Option<QuestClick> {
    let Some(UiAction::Quest(id)) = input.ui_press else { return None };
    match id {
        QuestId::Continue => Some(QuestClick::Continue),
        QuestId::Answer(i) => match view {
            QuestView::Puzzle { choices, .. } => choices.get(i).map(|&v| QuestClick::Answer(v)),
            _ => None,
        },
        QuestId::Option(i) => Some(QuestClick::Choose(i)),
    }
}

pub fn handle_key(input: &FrameInput, view: &QuestView) -> Option<QuestClick> {
    if view.has_continue() && (input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter)) {
        return Some(QuestClick::Continue);
    }
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
    let pressed = keys.iter().position(|k| input.pressed(*k))?;
    match view {
        QuestView::Puzzle { choices, .. } => choices.get(pressed).map(|&v| QuestClick::Answer(v)),
        QuestView::Choice { options, .. } if pressed < options.len() => Some(QuestClick::Choose(pressed)),
        _ => None,
    }
}
