//! Settings overlay (T / the gear): read-aloud, text speed, and the
//! parent-only experimental section, as a Bevy UI panel.

use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::{default, AlignItems, Bundle, ChildSpawner, Children, Node, SpawnRelated, UiRect, Val};
use bevy::text::Justify;
use bevy::ui::{percent, px, BackgroundColor, BorderColor, BorderRadius};

use crate::prelude::*;
use crate::settings::{self, TextSpeed};
use robot_buddy_domain::types::GamePace;
use crate::game::FeatureFlags;
use crate::input::FrameInput;
use super::kit::{self, backdrop, button, col, fitted, Panel};
use super::views::UiViews;
use super::UiAction;

/// Which experimental in-development feature a parent toggled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    Encounters,
    Quest,
}

pub enum SettingsResult {
    Close,
    BackToTitle,
    /// Show/hide the parent-only experimental section.
    ToggleParentPanel,
    /// Flip an experimental feature flag on the live `Game`.
    ToggleFeature(Feature),
    /// Download the session JSON (event log + profile). Mouse-reachable so it
    /// doesn't depend on the backtick/debug-overlay keybind.
    ExportSession,
    /// Set how fast the arcade cabinet runs. Parent-only: it changes how long
    /// a kid has to think, never which numbers they're asked for.
    SetPace(GamePace),
}

const PANEL_BG: Color = Color::new(0.086, 0.129, 0.243, 1.0);      // #16213E
const ACCENT: Color = Color::new(0.0, 0.902, 0.463, 1.0);          // #00E676
const LABEL_GRAY: Color = Color::new(0.690, 0.745, 0.773, 1.0);    // #B0BEC5
const BTN_OFF: Color = Color::new(0.216, 0.278, 0.310, 1.0);       // #37474F
const BTN_TXT_OFF: Color = Color::new(0.565, 0.643, 0.682, 1.0);   // #90A4AE
const HINT_GRAY: Color = Color::new(0.329, 0.431, 0.478, 1.0);     // #546E7A
const DARK_TXT: Color = Color::new(26.0 / 255.0, 26.0 / 255.0, 46.0 / 255.0, 1.0);

const FEATURES: [(Feature, &str); 2] = [
    (Feature::Encounters, "Random encounters"),
    (Feature::Quest, "Quests"),
];

const SPEEDS: [TextSpeed; 3] = [TextSpeed::Slow, TextSpeed::Normal, TextSpeed::Fast];

/// The overlay's buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsId {
    Tts,
    Speed(TextSpeed),
    Parent,
    Feature(Feature),
    Export,
    Pace(GamePace),
    BackToTitle,
    Done,
}

/// What the overlay shows: the live flags plus the parent-panel reveal.
#[derive(Clone, Copy, PartialEq)]
pub struct SettingsModel {
    pub features: FeatureFlags,
    pub parent_open: bool,
    pub pace: GamePace,
}

/// The overlay as shown: the game's settings plus the player's own.
#[derive(Clone, Copy, PartialEq)]
pub struct SettingsView {
    pub model: SettingsModel,
    pub tts: bool,
    pub speed: TextSpeed,
    pub screen_h: f32,
}

impl SettingsView {
    pub fn new(model: SettingsModel, screen: (f32, f32)) -> Self {
        SettingsView { model, tts: settings::tts_enabled(), speed: settings::text_speed(), screen_h: screen.1 }
    }
}

fn feature_on(features: FeatureFlags, f: Feature) -> bool {
    match f {
        Feature::Encounters => features.encounters,
        Feature::Quest => features.quest,
    }
}

/// Buttons give up height (down to a still-tappable 32px) before anything
/// is clipped on a short window.
const MIN_BUTTON_H: f32 = 32.0;

fn on_off(on: bool) -> (Color, Color) {
    if on { (ACCENT, DARK_TXT) } else { (BTN_OFF, BTN_TXT_OFF) }
}

/// A full-width button `h` tall: its fill, and its label's colour.
fn toggle_row(p: &mut ChildSpawner, id: SettingsId, label: String, size: f32, h: f32, (fill, ink): (Color, Color)) {
    let node = Node {
        height: px(h),
        min_height: px(MIN_BUTTON_H),
        border: UiRect::ZERO,
        border_radius: BorderRadius::all(px(8)),
        ..kit::centered()
    };
    p.spawn(button(UiAction::Settings(id), node, fill, fill)).with_child(fitted(label, size, 12.0, false, ink, Justify::Center));
}

/// Three equal buttons side by side, `h` tall; the row gives up height
/// (down to MIN_BUTTON_H) on a short window and the buttons stretch to it.
fn thirds(p: &mut ChildSpawner, h: f32, size: f32, buttons: Vec<(SettingsId, &'static str, bool)>) {
    let row = Node { column_gap: px(10), align_items: AlignItems::Stretch, height: px(h), min_height: px(MIN_BUTTON_H), ..default() };
    p.spawn(row).with_children(|p| {
        for (id, label, on) in buttons {
            let (fill, ink) = on_off(on);
            let node = Node { flex_grow: 1.0, flex_basis: px(0), min_width: px(0), border: UiRect::ZERO, border_radius: BorderRadius::all(px(8)), ..kit::centered() };
            p.spawn(button(UiAction::Settings(id), node, fill, fill)).with_child(fitted(label, size, 12.0, false, ink, Justify::Center));
        }
    });
}

fn label(p: &mut ChildSpawner, s: &str) {
    p.spawn(Node { flex_shrink: 0.0, ..col() }).with_child(fitted(s, 18.0, 12.0, false, LABEL_GRAY, Justify::Left));
}

pub struct SettingsPanel;

impl Panel for SettingsPanel {
    type View = SettingsView;
    // Above every other panel: it opens over any of them.
    const Z: i32 = 90;
    fn view(views: &UiViews) -> Option<&SettingsView> {
        views.settings.as_ref()
    }
    fn bundle(v: &SettingsView) -> impl Bundle {
        let v = *v;
        (backdrop(0.75, 20.0), Children::spawn(SpawnWith(move |p: &mut ChildSpawner| spawn_panel(p, v))))
    }
}

fn spawn_panel(p: &mut ChildSpawner, v: SettingsView) {
    let m = v.model;
    let gap = if v.screen_h < 600.0 { 6.0 } else { 10.0 };
    let panel = Node {
        width: percent(100),
        max_width: px(480),
        min_height: px(540.0_f32.min(v.screen_h - 40.0)),
        max_height: percent(100),
        padding: UiRect::new(px(28), px(28), px(18), px(10)),
        row_gap: px(gap),
        border: UiRect::all(px(3)),
        border_radius: BorderRadius::all(px(16)),
        ..col()
    };
    p.spawn((panel, BackgroundColor(kit::color(PANEL_BG)), BorderColor::all(kit::color(ACCENT)))).with_children(|p| {
        p.spawn(Node { flex_shrink: 0.0, ..col() }).with_child(fitted("Settings", 36.0, 20.0, false, ACCENT, Justify::Center));
        // The parent section is its own page: it replaces the kid's settings
        // while open (both together never fit a default window).
        if !m.parent_open {
            let tts = if v.tts { "Read dialogue aloud: ON" } else { "Read dialogue aloud: OFF" };
            toggle_row(p, SettingsId::Tts, tts.into(), 22.0, 56.0, on_off(v.tts));
            p.spawn(Node { row_gap: px(6), min_height: px(0), padding: UiRect::top(px(8)), ..col() }).with_children(|p| {
                label(p, "Text speed");
                thirds(p, 48.0, 22.0, SPEEDS.iter().map(|&ts| (SettingsId::Speed(ts), ts.label(), v.speed == ts)).collect());
            });
        }
        let parent_label = if m.parent_open { "Parent options  ▾" } else { "Parent options  ▸" };
        toggle_row(p, SettingsId::Parent, parent_label.into(), 22.0, 44.0, (BTN_OFF, LABEL_GRAY));
        if m.parent_open {
            // A short window scrolls the parent section rather than squash it.
            p.spawn(kit::scroll_list("parent", gap)).with_children(|p| {
                for &(f, name) in &FEATURES {
                    let on = feature_on(m.features, f);
                    let state = if on { "ON" } else { "OFF" };
                    toggle_row(p, SettingsId::Feature(f), format!("{name}: {state}"), 20.0, 44.0, on_off(on));
                }
                // Export the session data (parent dashboard action).
                toggle_row(p, SettingsId::Export, "Export session data".into(), 20.0, 44.0, (BTN_OFF, ACCENT));
                // Arcade pace: parents need to know this is the arcade's
                // speed, not the child's level.
                label(p, "Arcade speed");
                thirds(p, 44.0, 20.0, GamePace::ALL.iter().map(|&pc| (SettingsId::Pace(pc), pc.label(), m.pace == pc)).collect());
                p.spawn(Node { flex_shrink: 0.0, ..col() })
                    .with_child(fitted("Experimental — for playtesting", 16.0, 11.0, false, HINT_GRAY, Justify::Left));
            });
        }
        toggle_row(p, SettingsId::BackToTitle, "Back to title screen".into(), 22.0, 48.0, (BTN_OFF, BTN_TXT_OFF));
        // Done sits at the bottom of the panel.
        p.spawn(Node { margin: UiRect::top(Val::Auto), ..col() })
            .with_children(|p| toggle_row(p, SettingsId::Done, "Done".into(), 26.0, 52.0, (ACCENT, DARK_TXT)));
        p.spawn(Node { flex_shrink: 0.0, ..col() }).with_child(fitted("Press T or ESC to close", 18.0, 12.0, false, HINT_GRAY, Justify::Center));
    });
}

/// Handle input; returns a result if the overlay state should change. The
/// caller owns the live feature flags and the parent-panel reveal state.
pub fn handle_input(input: &FrameInput) -> Option<SettingsResult> {
    if input.pressed(KeyCode::Escape) || input.pressed(KeyCode::T) {
        return Some(SettingsResult::Close);
    }
    let Some(UiAction::Settings(id)) = input.ui_press else { return None };
    match id {
        SettingsId::Tts => {
            settings::toggle_tts();
            if !settings::tts_enabled() {
                crate::audio::tts::cancel();
            }
            None
        }
        SettingsId::Speed(ts) => {
            settings::set_text_speed(ts);
            None
        }
        SettingsId::Pace(p) => Some(SettingsResult::SetPace(p)),
        SettingsId::Parent => Some(SettingsResult::ToggleParentPanel),
        SettingsId::Feature(f) => Some(SettingsResult::ToggleFeature(f)),
        SettingsId::Export => Some(SettingsResult::ExportSession),
        SettingsId::BackToTitle => Some(SettingsResult::BackToTitle),
        SettingsId::Done => Some(SettingsResult::Close),
    }
}
