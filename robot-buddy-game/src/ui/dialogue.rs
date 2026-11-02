use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::{
    children, default, AlignItems, App, Bundle, ChildSpawner, Children, Component, IntoScheduleConfigs, JustifyContent,
    Node, Pickable, PositionType, Query, Res, Spawn, SpawnRelated, TextColor, TextFont, TextSpan, UiRect, Update, Val,
    With, Without, ZIndex,
};
use bevy::text::{Justify, LineHeight};
use bevy::ui::{percent, px, BackgroundColor, BorderColor};

use crate::prelude::*;
use crate::audio;
use super::kit::{self, Panel};
use super::views::UiViews;
use crate::settings;

pub struct DialogueLine {
    pub speaker: String,
    pub text: String,
}

pub struct DialogueBox {
    lines: Vec<DialogueLine>,
    current_line: usize,
    char_index: usize,
    char_timer: f32,
    waiting_for_input: bool,
    pub active: bool,
}

impl DialogueBox {
    pub fn new() -> Self {
        DialogueBox {
            lines: vec![],
            current_line: 0,
            char_index: 0,
            char_timer: 0.0,
            waiting_for_input: false,
            active: false,
        }
    }

    pub fn start(&mut self, lines: Vec<DialogueLine>) {
        // Speak the first line
        if let Some(line) = lines.first() {
            audio::tts::speak(&line.speaker, &line.text);
        }
        self.lines = lines;
        self.current_line = 0;
        self.char_index = 0;
        self.char_timer = 0.0;
        self.waiting_for_input = false;
        self.active = true;
    }

    /// True if the typewriter is still revealing characters (not yet waiting for input).
    pub fn is_typewriting(&self) -> bool {
        self.active && !self.waiting_for_input
    }

    /// Read-only view of the queued lines. Used by integration tests to assert
    /// on who says what during a scene without driving the typewriter.
    pub fn lines(&self) -> &[DialogueLine] {
        &self.lines
    }

    pub fn advance(&mut self) {
        if !self.active { return; }
        if !self.waiting_for_input {
            // Skip typewriter — show full line
            if let Some(line) = self.lines.get(self.current_line) {
                self.char_index = line.text.len();
                self.waiting_for_input = true;
            }
            return;
        }
        self.current_line += 1;
        if self.current_line >= self.lines.len() {
            self.active = false;
            audio::tts::cancel();
            return;
        }
        // Speak the next line
        if let Some(line) = self.lines.get(self.current_line) {
            audio::tts::speak(&line.speaker, &line.text);
        }
        self.char_index = 0;
        self.char_timer = 0.0;
        self.waiting_for_input = false;
    }

    pub fn update(&mut self, dt: f32) {
        if !self.active || self.waiting_for_input { return; }
        if let Some(line) = self.lines.get(self.current_line) {
            self.char_timer += dt;
            let char_speed = settings::char_speed_seconds();
            while self.char_timer >= char_speed && self.char_index < line.text.len() {
                self.char_timer -= char_speed;
                // Advance by one character (handle UTF-8 properly)
                let remaining = &line.text[self.char_index..];
                if let Some(c) = remaining.chars().next() {
                    self.char_index += c.len_utf8();
                }
            }
            if self.char_index >= line.text.len() {
                self.waiting_for_input = true;
            }
        }
    }

    /// What the dialogue panel shows, if a line is up.
    pub fn view(&self) -> Option<DialogueView> {
        if !self.active {
            return None;
        }
        let line = self.lines.get(self.current_line)?;
        Some(DialogueView { speaker: line.speaker.clone(), text: line.text.clone(), waiting: self.waiting_for_input })
    }

    /// Characters of the current line the typewriter has revealed.
    pub fn revealed(&self) -> usize {
        self.lines.get(self.current_line).map_or(0, |l| l.text[..self.char_index.min(l.text.len())].chars().count())
    }
}

/// How far the speaker's name tab rises above the box's top edge.
const TAB_OVERHANG: f32 = 18.0;

/// One dialogue line, as the panel shows it.
#[derive(Clone, PartialEq, Debug)]
pub struct DialogueView {
    pub speaker: String,
    pub text: String,
    /// The whole line is out; "SPACE >" blinks.
    pub waiting: bool,
}

/// The dialogue box along the bottom of the screen. At least as tall as it
/// always was, it grows (up to 60% of the screen) for a long line. The
/// typewriter reveals a prefix of the full line, which is laid out whole from
/// the start, so words never hop to the next line mid-sentence and the box
/// never grows mid-line. "SPACE >" has its own footer row.
pub struct DialoguePanel;

/// The part of the line not yet revealed (laid out, but transparent).
#[derive(Component)]
struct Unrevealed;

/// The part of the line revealed so far.
#[derive(Component)]
struct Revealed;

impl Panel for DialoguePanel {
    type View = DialogueView;
    const Z: i32 = 20;
    fn view(views: &UiViews) -> Option<&DialogueView> {
        views.dialogue.as_ref()
    }
    fn bundle(v: &DialogueView) -> impl Bundle {
        let v = v.clone();
        let border = kit::color(speaker_color(&v.speaker));
        let root = Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(10)),
            justify_content: JustifyContent::End,
            ..kit::col()
        };
        // The dialogue never takes a tap: a click anywhere advances it.
        (root, Pickable::IGNORE, Children::spawn(SpawnWith(move |p: &mut ChildSpawner| {
            let area = Node { width: percent(100), min_width: px(0), ..kit::col() };
            p.spawn((area, Pickable::IGNORE)).with_children(|p| {
                // The name tab sits across the box's top edge, in front of it.
                p.spawn((Node { height: px(34), flex_shrink: 0.0, padding: UiRect::left(px(15)), ..default() }, ZIndex(1), Pickable::IGNORE))
                    .with_child((
                        Node { height: px(34), padding: UiRect::axes(px(12), px(0)), flex_shrink: 1.0, min_width: px(0), align_items: AlignItems::Center, ..default() },
                        BackgroundColor(border),
                        Pickable::IGNORE,
                        Children::spawn(Spawn(kit::fitted(v.speaker.clone(), 26.0, 14.0, false, Color::from_rgba(26, 26, 46, 255), Justify::Left))),
                    ));
                let body = Node {
                    min_height: px(170.0),
                    max_height: Val::Vh(60.0),
                    margin: UiRect::top(px(-TAB_OVERHANG)),
                    padding: UiRect::new(px(20), px(20), px(12.0 + TAB_OVERHANG), px(8)),
                    row_gap: px(6),
                    border: UiRect::all(px(3)),
                    ..kit::col()
                };
                p.spawn((body, BackgroundColor(kit::color(Color::from_rgba(20, 20, 40, 230))), BorderColor::all(border), Pickable::IGNORE))
                    .with_children(|p| {
                        p.spawn((
                            kit::filled("", 28.0, 16.0, WHITE, Justify::Left),
                            children![
                                (TextSpan::new(""), TextFont::from_font_size(28.0), LineHeight::RelativeToFont(1.15), TextColor(kit::color(WHITE)), Revealed),
                                (TextSpan::new(v.text.clone()), TextFont::from_font_size(28.0), LineHeight::RelativeToFont(1.15), TextColor(bevy::color::Color::NONE), Unrevealed),
                            ],
                        ));
                        if v.waiting {
                            // Pinned to the bottom of the box (an auto top margin, not a
                            // grow spacer: taffy over-sizes an auto-height column that
                            // holds one).
                            let footer = Node { justify_content: JustifyContent::End, flex_shrink: 0.0, margin: UiRect::top(Val::Auto), ..default() };
                            p.spawn((footer, kit::Blink(6.0), Pickable::IGNORE))
                                .with_child(kit::fitted("SPACE >", 20.0, 14.0, false, Color::from_rgba(150, 150, 150, 255), Justify::Right));
                        }
                    });
            });
        })))
    }
    fn setup(app: &mut App) {
        app.add_systems(Update, typewriter.after(kit::PanelSystems));
    }
}

/// Move the typewriter's split point: the revealed prefix is white, the rest
/// is laid out but invisible.
fn typewriter(
    views: Res<UiViews>,
    mut shown: Query<&mut TextSpan, (With<Revealed>, Without<Unrevealed>)>,
    mut hidden: Query<&mut TextSpan, (With<Unrevealed>, Without<Revealed>)>,
) {
    let Some(v) = &views.dialogue else { return };
    let split = v.text.char_indices().nth(views.dialogue_reveal).map_or(v.text.len(), |(i, _)| i);
    let (a, b) = v.text.split_at(split);
    for mut s in &mut shown {
        if s.0 != a {
            s.0 = a.to_string();
        }
    }
    for mut s in &mut hidden {
        if s.0 != b {
            s.0 = b.to_string();
        }
    }
}

fn speaker_color(speaker: &str) -> Color {
    match speaker {
        "Sparky" => Color::from_rgba(0, 230, 118, 255),
        "Mommy" => Color::from_rgba(224, 64, 251, 255),
        "Professor Gizmo" => Color::from_rgba(179, 136, 255, 255),
        "Bolt the Shopkeeper" => Color::from_rgba(255, 183, 77, 255),
        "???" => Color::from_rgba(206, 147, 216, 255),
        "B0RK.exe" => Color::from_rgba(118, 255, 3, 255),
        "Old Oak" => Color::from_rgba(165, 214, 167, 255),
        _ => Color::from_rgba(255, 213, 79, 255),
    }
}
