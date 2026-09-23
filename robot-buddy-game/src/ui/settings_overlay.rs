//! Settings overlay (T / the gear): read-aloud, text speed, and the
//! parent-only experimental section. Laid out through `ui::layout`; the same
//! `Frame` is painted and hit-tested, section labels included.

use crate::prelude::*;
use crate::settings::{self, TextSpeed};
use robot_buddy_domain::types::GamePace;
use crate::game::FeatureFlags;
use crate::input::FrameInput;
use crate::ui::layout::{self, button, col, paint, row, spacer, text, Fit, Frame, Justify, Kind, Node};

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

/// Every element of the overlay. Buttons carry what they do; `*Label` ids are
/// the text inside them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsId {
    Panel,
    Title,
    Tts,
    TtsLabel,
    SpeedLabel,
    Speed(TextSpeed),
    SpeedText(TextSpeed),
    Parent,
    ParentLabel,
    Feature(Feature),
    FeatureLabel(Feature),
    Export,
    ExportLabel,
    PaceLabel,
    Pace(GamePace),
    PaceText(GamePace),
    Note,
    BackToTitle,
    BackLabel,
    Done,
    DoneLabel,
    Hint,
}

/// What the overlay shows: the live flags plus the parent-panel reveal.
#[derive(Clone, Copy)]
pub struct SettingsModel {
    pub features: FeatureFlags,
    pub parent_open: bool,
    pub pace: GamePace,
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

fn toggle_row(id: SettingsId, label_id: SettingsId, label: String, size: u16, h: f32) -> Node<SettingsId> {
    button(id, label_id, label, size, Fit::shrink(12)).h(h).min_h(MIN_BUTTON_H)
}

/// Three equal buttons side by side, `h` tall; the row gives up height
/// (down to MIN_BUTTON_H) on a short window and the buttons stretch to it.
fn thirds(h: f32, buttons: impl IntoIterator<Item = Node<SettingsId>>) -> Node<SettingsId> {
    row()
        .gap(10.0)
        .align(layout::Align::Stretch)
        .h(h)
        .min_h(MIN_BUTTON_H)
        .children(buttons.into_iter().map(|b| b.grow(1.0).min_w(0.0)))
}

fn build(m: SettingsModel, screen: (f32, f32)) -> Node<SettingsId> {
    let tts = if settings::tts_enabled() { "Read dialogue aloud: ON" } else { "Read dialogue aloud: OFF" };
    let parent_label = if m.parent_open { "Parent options  ▾" } else { "Parent options  ▸" };
    let label = |s: &str, id| text(s, 18, Fit::shrink(12)).id(id).fixed();
    let gap = if screen.1 < 600.0 { 6.0 } else { 10.0 };

    let parent_section = m.parent_open.then(|| {
        // min_h(0): lets the section's buttons give up height on a short window.
        col()
            .min_h(0.0)
            .gap(gap)
            .children(FEATURES.iter().map(|&(f, name)| {
                let state = if feature_on(m.features, f) { "ON" } else { "OFF" };
                toggle_row(SettingsId::Feature(f), SettingsId::FeatureLabel(f), format!("{name}: {state}"), 20, 44.0)
            }))
            // Export the session data (parent dashboard action).
            .child(toggle_row(SettingsId::Export, SettingsId::ExportLabel, "Export session data".into(), 20, 44.0))
            // Arcade pace: parents need to know this is the arcade's speed,
            // not the child's level.
            .child(label("Arcade speed", SettingsId::PaceLabel))
            .child(thirds(44.0, GamePace::ALL.iter().map(|&p| {
                button(SettingsId::Pace(p), SettingsId::PaceText(p), p.label(), 20, Fit::shrink(12))
            })))
            .child(text("Experimental — for playtesting", 16, Fit::shrink(11)).id(SettingsId::Note).fixed())
    });

    let panel = col()
        .id(SettingsId::Panel)
        .w_pct(1.0)
        .max_w(480.0)
        .min_h(540.0_f32.min(screen.1 - 40.0))
        .pad_edges(28.0, 18.0, 28.0, 10.0)
        .gap(gap)
        .child(text("Settings", 36, Fit::shrink(20)).id(SettingsId::Title).center_text().fixed())
        // The parent section is its own page: it replaces the kid's settings
        // while open (both together never fit a default window).
        .maybe((!m.parent_open).then(|| toggle_row(SettingsId::Tts, SettingsId::TtsLabel, tts.into(), 22, 56.0)))
        .maybe((!m.parent_open).then(|| {
            col().gap(6.0).min_h(0.0).pad_edges(0.0, 8.0, 0.0, 0.0).children([
                label("Text speed", SettingsId::SpeedLabel),
                thirds(48.0, SPEEDS.iter().map(|&ts| {
                    button(SettingsId::Speed(ts), SettingsId::SpeedText(ts), ts.label(), 22, Fit::shrink(12))
                })),
            ])
        }))
        .child(toggle_row(SettingsId::Parent, SettingsId::ParentLabel, parent_label.into(), 22, 44.0))
        .maybe(parent_section)
        .child(toggle_row(SettingsId::BackToTitle, SettingsId::BackLabel, "Back to title screen".into(), 22, 48.0))
        .child(spacer())
        .child(toggle_row(SettingsId::Done, SettingsId::DoneLabel, "Done".into(), 26, 52.0))
        .child(text("Press T or ESC to close", 18, Fit::shrink(12)).id(SettingsId::Hint).center_text().fixed());
    col().pad(20.0).align(layout::Align::Center).justify(Justify::Center).child(panel)
}

/// Lay the overlay out top-down. The parent section only takes space when
/// open, so the panel grows to fit (up to the screen). Pure.
pub fn layout(screen: (f32, f32), m: SettingsModel) -> Frame<SettingsId> {
    layout::layout(&build(m, screen), layout::screen_rect(screen))
}

fn center_of(screen: (f32, f32), parent_open: bool, id: SettingsId) -> (f32, f32) {
    let m = SettingsModel { features: FeatureFlags::default(), parent_open, pace: GamePace::ALL[0] };
    layout(screen, m).rect(id).unwrap_or_else(|| panic!("{id:?} not laid out")).center()
}

/// Screen-space center of the "Parent options" reveal row (for input/tests).
pub fn parent_toggle_center(screen: (f32, f32)) -> (f32, f32) {
    center_of(screen, false, SettingsId::Parent)
}

/// Center of an arcade-pace button (the parent panel must be open).
pub fn pace_button_center(screen: (f32, f32), pace: GamePace) -> (f32, f32) {
    center_of(screen, true, SettingsId::Pace(pace))
}

/// Center of a feature toggle row (the parent panel must be open to show them).
pub fn feature_toggle_center(screen: (f32, f32), feature: Feature) -> (f32, f32) {
    center_of(screen, true, SettingsId::Feature(feature))
}

pub fn draw(screen: (f32, f32), m: SettingsModel) {
    let f = layout(screen, m);
    paint::dim(f.bounds, 0.75);
    let on_off = |on: bool| if on { (ACCENT, DARK_TXT) } else { (BTN_OFF, BTN_TXT_OFF) };
    for el in f.elements() {
        let Some(id) = el.id else { continue };
        let r = el.rect;
        if let Kind::Text(t) = &el.kind {
            let color = match id {
                SettingsId::Title | SettingsId::ExportLabel => ACCENT,
                SettingsId::SpeedLabel | SettingsId::PaceLabel | SettingsId::ParentLabel => LABEL_GRAY,
                SettingsId::Note | SettingsId::Hint => HINT_GRAY,
                SettingsId::TtsLabel => on_off(settings::tts_enabled()).1,
                SettingsId::SpeedText(ts) => on_off(settings::text_speed() == ts).1,
                SettingsId::FeatureLabel(ft) => on_off(feature_on(m.features, ft)).1,
                SettingsId::PaceText(p) => on_off(m.pace == p).1,
                SettingsId::DoneLabel => DARK_TXT,
                _ => BTN_TXT_OFF,
            };
            paint::text(t, color);
            continue;
        }
        match id {
            SettingsId::Panel => {
                paint::round_rect(r, 16.0, PANEL_BG);
                paint::outline(r, 3.0, ACCENT);
            }
            SettingsId::Tts => paint::round_rect(r, 8.0, on_off(settings::tts_enabled()).0),
            SettingsId::Speed(ts) => paint::round_rect(r, 8.0, on_off(settings::text_speed() == ts).0),
            SettingsId::Feature(ft) => paint::round_rect(r, 8.0, on_off(feature_on(m.features, ft)).0),
            SettingsId::Pace(p) => paint::round_rect(r, 8.0, on_off(m.pace == p).0),
            SettingsId::Parent | SettingsId::Export | SettingsId::BackToTitle => paint::round_rect(r, 8.0, BTN_OFF),
            SettingsId::Done => paint::round_rect(r, 10.0, ACCENT),
            _ => {}
        }
    }
}

/// Handle input; returns a result if the overlay state should change. The
/// caller owns the live feature flags and the parent-panel reveal state.
pub fn handle_input(input: &FrameInput, screen: (f32, f32), m: SettingsModel) -> Option<SettingsResult> {
    if input.pressed(KeyCode::Escape) || input.pressed(KeyCode::T) {
        return Some(SettingsResult::Close);
    }
    if !input.mouse_clicked {
        return None;
    }
    let (mx, my) = input.mouse_pos;
    match layout(screen, m).hit_at(mx, my)? {
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
        _ => None,
    }
}
