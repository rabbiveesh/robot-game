use crate::prelude::*;
use crate::settings::{self, TextSpeed};
use robot_buddy_domain::types::GamePace;
use crate::game::FeatureFlags;
use crate::input::FrameInput;

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

struct Row {
    rect: (f32, f32, f32, f32),
    action: RowAction,
}

#[derive(Clone, Copy)]
enum RowAction {
    ToggleTts,
    SetSpeed(TextSpeed),
    ToggleParentPanel,
    ToggleFeature(Feature),
    ExportSession,
    SetPace(GamePace),
    BackToTitle,
    Done,
}

fn round_rect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    draw_rectangle(x + r, y, w - 2.0 * r, h, color);
    draw_rectangle(x, y + r, w, h - 2.0 * r, color);
    draw_circle(x + r, y + r, r, color);
    draw_circle(x + w - r, y + r, r, color);
    draw_circle(x + r, y + h - r, r, color);
    draw_circle(x + w - r, y + h - r, r, color);
}

const FEATURES: [(Feature, &str); 2] = [
    (Feature::Encounters, "Random encounters"),
    (Feature::Quest, "Quests"),
];

/// Lay the overlay out top-down. The parent section (and its feature rows) only
/// take space when `parent_open`, so the panel grows to fit.
fn layout(screen: (f32, f32), parent_open: bool) -> (f32, f32, f32, f32, Vec<Row>) {
    let (sw, sh) = screen;
    let pad = 28.0;
    let feature_h = 44.0;
    let feature_gap = 10.0;
    // Panel height: base content + the parent section when expanded.
    let base_h = 540.0;
    let extra = if parent_open {
        // Feature toggles + the Export-session button + the arcade-pace row
        // (which carries a section label above it).
        (FEATURES.len() as f32 + 1.0) * (feature_h + feature_gap) + 24.0
            + feature_h + feature_gap + 26.0
    } else {
        0.0
    };
    let panel_w = (sw - 80.0).min(480.0);
    let panel_h = base_h + extra;
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;
    let inner_w = panel_w - pad * 2.0;
    let mut rows = Vec::new();

    // TTS toggle
    let ts_y = panel_y + 80.0;
    let ts_h = 56.0;
    rows.push(Row { rect: (panel_x + pad, ts_y, inner_w, ts_h), action: RowAction::ToggleTts });

    // Text speed — three side-by-side buttons
    let speed_y = ts_y + ts_h + 54.0;
    let speed_h = 48.0;
    let speed_gap = 10.0;
    let speed_w = (inner_w - speed_gap * 2.0) / 3.0;
    for (i, ts) in [TextSpeed::Slow, TextSpeed::Normal, TextSpeed::Fast].iter().enumerate() {
        let x = panel_x + pad + i as f32 * (speed_w + speed_gap);
        rows.push(Row { rect: (x, speed_y, speed_w, speed_h), action: RowAction::SetSpeed(*ts) });
    }

    // Parent-options reveal
    let parent_y = speed_y + speed_h + 40.0;
    let parent_h = 44.0;
    rows.push(Row { rect: (panel_x + pad, parent_y, inner_w, parent_h), action: RowAction::ToggleParentPanel });

    // Experimental feature toggles (only when expanded)
    let mut cursor = parent_y + parent_h + 12.0;
    if parent_open {
        for (feature, _) in FEATURES {
            rows.push(Row { rect: (panel_x + pad, cursor, inner_w, feature_h), action: RowAction::ToggleFeature(feature) });
            cursor += feature_h + feature_gap;
        }
        // Export the session data (parent dashboard action).
        rows.push(Row { rect: (panel_x + pad, cursor, inner_w, feature_h), action: RowAction::ExportSession });
        cursor += feature_h + feature_gap;

        // Arcade pace — three side-by-side buttons under their own label.
        cursor += 26.0; // room for the "Arcade speed" label
        let pace_gap = 10.0;
        let pace_w = (inner_w - pace_gap * 2.0) / 3.0;
        for (i, p) in GamePace::ALL.iter().enumerate() {
            let x = panel_x + pad + i as f32 * (pace_w + pace_gap);
            rows.push(Row { rect: (x, cursor, pace_w, feature_h), action: RowAction::SetPace(*p) });
        }
        cursor += feature_h + feature_gap;
        cursor += 12.0;
    }

    // Back to title
    let btt_h = 48.0;
    rows.push(Row { rect: (panel_x + pad, cursor, inner_w, btt_h), action: RowAction::BackToTitle });

    // Done — bottom
    let done_y = panel_y + panel_h - 72.0;
    let done_h = 52.0;
    rows.push(Row { rect: (panel_x + pad, done_y, inner_w, done_h), action: RowAction::Done });

    (panel_x, panel_y, panel_w, panel_h, rows)
}

fn center((x, y, w, h): (f32, f32, f32, f32)) -> (f32, f32) {
    (x + w / 2.0, y + h / 2.0)
}

/// Screen-space center of the "Parent options" reveal row (for input/tests).
pub fn parent_toggle_center(screen: (f32, f32)) -> (f32, f32) {
    let (_, _, _, _, rows) = layout(screen, false);
    rows.iter()
        .find_map(|r| matches!(r.action, RowAction::ToggleParentPanel).then(|| center(r.rect)))
        .expect("parent toggle row always present")
}

/// Center of an arcade-pace button (the parent panel must be open).
pub fn pace_button_center(screen: (f32, f32), pace: GamePace) -> (f32, f32) {
    let (_, _, _, _, rows) = layout(screen, true);
    rows.iter()
        .find_map(|r| match r.action {
            RowAction::SetPace(p) if p == pace => Some(center(r.rect)),
            _ => None,
        })
        .expect("pace buttons present when parent panel open")
}

/// Center of a feature toggle row (the parent panel must be open to show them).
pub fn feature_toggle_center(screen: (f32, f32), feature: Feature) -> (f32, f32) {
    let (_, _, _, _, rows) = layout(screen, true);
    rows.iter()
        .find_map(|r| match r.action {
            RowAction::ToggleFeature(f) if f == feature => Some(center(r.rect)),
            _ => None,
        })
        .expect("feature toggle row present when parent panel open")
}

pub fn draw(screen: (f32, f32), features: FeatureFlags, parent_open: bool, pace: GamePace) {
    let (sw, sh) = screen;
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.75));

    let (panel_x, panel_y, panel_w, panel_h, rows) = layout(screen, parent_open);

    round_rect(panel_x, panel_y, panel_w, panel_h, 16.0, PANEL_BG);
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 3.0, ACCENT);

    let title = "Settings";
    let tw = measure_text(title, None, 36, 1.0).width;
    draw_text(title, panel_x + panel_w / 2.0 - tw / 2.0, panel_y + 48.0, 36.0, ACCENT);

    let feature_on = |f: Feature| match f {
        Feature::Encounters => features.encounters,
        Feature::Quest => features.quest,
    };
    let feature_label = |f: Feature| FEATURES.iter().find(|(ff, _)| *ff == f).map(|(_, l)| *l).unwrap_or("");

    for row in &rows {
        let (x, y, w, h) = row.rect;
        match row.action {
            RowAction::ToggleTts => {
                let on = settings::tts_enabled();
                let bg = if on { ACCENT } else { BTN_OFF };
                let fg = if on { Color::from_rgba(26, 26, 46, 255) } else { BTN_TXT_OFF };
                round_rect(x, y, w, h, 8.0, bg);
                let label = if on { "Read dialogue aloud: ON" } else { "Read dialogue aloud: OFF" };
                let lw = measure_text(label, None, 22, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 8.0, 22.0, fg);
            }
            RowAction::SetSpeed(ts) => {
                let active = settings::text_speed() == ts;
                let bg = if active { ACCENT } else { BTN_OFF };
                let fg = if active { Color::from_rgba(26, 26, 46, 255) } else { BTN_TXT_OFF };
                round_rect(x, y, w, h, 8.0, bg);
                let label = ts.label();
                let lw = measure_text(label, None, 22, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 8.0, 22.0, fg);
            }
            RowAction::ToggleParentPanel => {
                round_rect(x, y, w, h, 8.0, BTN_OFF);
                let label = if parent_open { "Parent options  ▾" } else { "Parent options  ▸" };
                let lw = measure_text(label, None, 22, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 8.0, 22.0, LABEL_GRAY);
            }
            RowAction::ToggleFeature(f) => {
                let on = feature_on(f);
                let bg = if on { ACCENT } else { BTN_OFF };
                let fg = if on { Color::from_rgba(26, 26, 46, 255) } else { BTN_TXT_OFF };
                round_rect(x, y, w, h, 8.0, bg);
                let label = format!("{}: {}", feature_label(f), if on { "ON" } else { "OFF" });
                let lw = measure_text(&label, None, 20, 1.0).width;
                draw_text(&label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 7.0, 20.0, fg);
            }
            RowAction::SetPace(p) => {
                // The first button carries the section label — parents need to
                // know this is the arcade's speed, not the child's level.
                if p == GamePace::ALL[0] {
                    draw_text("Arcade speed", x, y - 8.0, 18.0, LABEL_GRAY);
                }
                let active = pace == p;
                let bg = if active { ACCENT } else { BTN_OFF };
                let fg = if active { Color::from_rgba(26, 26, 46, 255) } else { BTN_TXT_OFF };
                round_rect(x, y, w, h, 8.0, bg);
                let label = p.label();
                let lw = measure_text(label, None, 20, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 7.0, 20.0, fg);
            }
            RowAction::ExportSession => {
                round_rect(x, y, w, h, 8.0, BTN_OFF);
                let label = "Export session data";
                let lw = measure_text(label, None, 20, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 7.0, 20.0, ACCENT);
            }
            RowAction::BackToTitle => {
                round_rect(x, y, w, h, 8.0, BTN_OFF);
                let label = "Back to title screen";
                let lw = measure_text(label, None, 22, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 8.0, 22.0, BTN_TXT_OFF);
            }
            RowAction::Done => {
                round_rect(x, y, w, h, 10.0, ACCENT);
                let label = "Done";
                let lw = measure_text(label, None, 26, 1.0).width;
                draw_text(label, x + w / 2.0 - lw / 2.0, y + h / 2.0 + 9.0, 26.0, Color::from_rgba(26, 26, 46, 255));
            }
        }
    }

    // "Text speed" section label, just above the speed buttons.
    let speed_label_y = panel_y + 80.0 + 56.0 + 34.0;
    draw_text("Text speed", panel_x + 28.0, speed_label_y, 18.0, LABEL_GRAY);

    if parent_open {
        let note = "Experimental — for playtesting";
        draw_text(note, panel_x + 28.0, panel_y + panel_h - 92.0, 16.0, HINT_GRAY);
    }

    let hint = "Press T or ESC to close";
    let hw = measure_text(hint, None, 18, 1.0).width;
    draw_text(hint, panel_x + panel_w / 2.0 - hw / 2.0, panel_y + panel_h - 12.0, 18.0, HINT_GRAY);
}

/// Handle input; returns a result if the overlay state should change. The
/// caller owns the live feature flags and the parent-panel reveal state.
pub fn handle_input(input: &FrameInput, screen: (f32, f32), parent_open: bool) -> Option<SettingsResult> {
    if input.pressed(KeyCode::Escape) || input.pressed(KeyCode::T) {
        return Some(SettingsResult::Close);
    }
    if !input.mouse_clicked {
        return None;
    }
    let (mx, my) = input.mouse_pos;
    let (_, _, _, _, rows) = layout(screen, parent_open);
    for row in rows {
        let (x, y, w, h) = row.rect;
        if mx >= x && mx <= x + w && my >= y && my <= y + h {
            match row.action {
                RowAction::ToggleTts => {
                    settings::toggle_tts();
                    if !settings::tts_enabled() {
                        crate::audio::tts::cancel();
                    }
                    return None;
                }
                RowAction::SetPace(p) => return Some(SettingsResult::SetPace(p)),
                RowAction::SetSpeed(ts) => {
                    settings::set_text_speed(ts);
                    return None;
                }
                RowAction::ToggleParentPanel => return Some(SettingsResult::ToggleParentPanel),
                RowAction::ToggleFeature(f) => return Some(SettingsResult::ToggleFeature(f)),
                RowAction::ExportSession => return Some(SettingsResult::ExportSession),
                RowAction::BackToTitle => return Some(SettingsResult::BackToTitle),
                RowAction::Done => return Some(SettingsResult::Close),
            }
        }
    }
    None
}
