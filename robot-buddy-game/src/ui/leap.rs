//! Shelly's leap panel — the strip along the bottom of the reef while a pearl
//! trip is going. Pure layout + render; the rules live in
//! `robot_buddy_domain::logic::leap`.
//!
//! It shows her call ("stone 12, four leaps") and the leap sizes on offer as
//! big tappable tiles, then swaps to a Leap / Swim back pair once the kid has
//! committed. Everything a kid needs to see stays on screen — the clue is never
//! a toast they can miss.

use crate::prelude::*;
use robot_buddy_domain::logic::leap::{LeapPhase, LeapSession};

use crate::ui::shop::UiRect;

pub struct LeapLayout {
    pub panel: UiRect,
    /// Size tiles, with the size each one commits to.
    pub choices: Vec<(UiRect, u8)>,
    /// Shown once a size is locked in.
    pub leap_btn: Option<UiRect>,
    pub back_btn: Option<UiRect>,
}

/// Panel geometry, all measured off the panel's top edge so the text rows and
/// the button row can't drift into each other.
const CALL_Y: f32 = 30.0;
const STATUS_Y: f32 = 56.0;
const ROW_Y: f32 = 70.0;
const BTN_H: f32 = 54.0;
/// Enough room for both text rows, the buttons, and a margin under them.
const PANEL_H: f32 = ROW_Y + BTN_H + 14.0;

pub fn layout(session: &LeapSession, screen: (f32, f32)) -> LeapLayout {
    let (sw, sh) = screen;
    let panel_h = PANEL_H;
    let panel = UiRect { x: 0.0, y: sh - panel_h, w: sw, h: panel_h };

    let btn_h = BTN_H;
    let gap = 12.0;
    let row_y = panel.y + ROW_Y;
    let mut choices = Vec::new();
    let mut leap_btn = None;
    let mut back_btn = None;

    if session.chosen.is_none() {
        let n = session.puzzle.choices.len().max(1);
        let btn_w = 96.0f32.min((sw - 40.0 - (n as f32 - 1.0) * gap) / n as f32);
        let row_w = n as f32 * btn_w + (n as f32 - 1.0) * gap;
        let start_x = sw / 2.0 - row_w / 2.0;
        for (i, &size) in session.puzzle.choices.iter().enumerate() {
            choices.push((
                UiRect { x: start_x + i as f32 * (btn_w + gap), y: row_y, w: btn_w, h: btn_h },
                size,
            ));
        }
    } else {
        let btn_w = 150.0;
        let row_w = btn_w * 2.0 + gap;
        let start_x = sw / 2.0 - row_w / 2.0;
        leap_btn = Some(UiRect { x: start_x, y: row_y, w: btn_w, h: btn_h });
        back_btn = Some(UiRect { x: start_x + btn_w + gap, y: row_y, w: btn_w, h: btn_h });
    }

    LeapLayout { panel, choices, leap_btn, back_btn }
}

pub enum LeapInput {
    Pick(u8),
    Leap,
    SwimBack,
}

pub fn handle_click(mx: f32, my: f32, layout: &LeapLayout) -> Option<LeapInput> {
    for (rect, size) in &layout.choices {
        if rect.contains(mx, my) {
            return Some(LeapInput::Pick(*size));
        }
    }
    if layout.leap_btn.is_some_and(|r| r.contains(mx, my)) {
        return Some(LeapInput::Leap);
    }
    if layout.back_btn.is_some_and(|r| r.contains(mx, my)) {
        return Some(LeapInput::SwimBack);
    }
    None
}

/// True when (mx, my) is over the panel at all — the map shouldn't take a
/// click-to-walk from a tap the kid aimed at Shelly's buttons.
pub fn absorbs_click(mx: f32, my: f32, layout: &LeapLayout) -> bool {
    layout.panel.contains(mx, my)
}

const PANEL_BG: Color = Color::new(0.043, 0.157, 0.216, 0.92);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);

/// Smallest font we'll shrink to before giving up and letting it clip.
const MIN_FONT: u16 = 11;

/// Largest size at or below `max` that fits `text` into `width`, measured with
/// `width_of`. Split from `fitted` so the shrink loop can be tested without a
/// macroquad context (text measurement needs a live window).
fn fitted_by(text: &str, width: f32, max: u16, width_of: impl Fn(&str, u16) -> f32) -> u16 {
    let mut size = max;
    while size > MIN_FONT && width_of(text, size) > width {
        size -= 1;
    }
    size
}

/// Largest size at or below `max` that fits `text` into `width`. Shelly's call
/// is a whole sentence and the panel is only as wide as the window, so on a
/// narrow screen it has to shrink rather than run off the edge.
fn fitted(text: &str, width: f32, max: u16) -> u16 {
    fitted_by(text, width, max, |t, size| measure_text(t, None, size, 1.0).width)
}

/// Draw `text` centered on `y`, shrunk to fit the panel's width.
fn centered(text: &str, p: UiRect, y: f32, max: u16, color: Color) {
    let room = p.w - 32.0;
    let size = fitted(text, room, max);
    let w = measure_text(text, None, size, 1.0).width;
    draw_text(text, p.x + p.w / 2.0 - w / 2.0, y, size as f32, color);
}

pub fn draw(session: &LeapSession, layout: &LeapLayout, call: &str, mouse: (f32, f32)) {
    let p = layout.panel;
    draw_rectangle(p.x, p.y, p.w, p.h, PANEL_BG);
    draw_line(p.x, p.y, p.x + p.w, p.y, 3.0, Color::new(0.30, 0.85, 0.90, 0.7));

    // Shelly's call, then a status line that tracks the trip. Both are
    // centered and shrink to fit — the call is a full sentence.
    centered(call, p, p.y + CALL_Y, 24, GOLD);

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
    centered(&status, p, p.y + STATUS_Y, 19, Color::new(1.0, 1.0, 1.0, 0.8));

    let (mx, my) = mouse;
    for (i, (rect, size)) in layout.choices.iter().enumerate() {
        let hot = rect.contains(mx, my);
        let bg = if hot { Color::from_rgba(50, 140, 190, 255) } else { Color::from_rgba(33, 96, 170, 255) };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.35));
        let label = format!("[{}] {size}", i + 1);
        let m = measure_text(&label, None, 28, 1.0);
        draw_text(&label, rect.x + rect.w / 2.0 - m.width / 2.0, rect.y + rect.h / 2.0 + 10.0,
            28.0, WHITE);
    }

    if let Some(r) = layout.leap_btn {
        let hot = r.contains(mx, my);
        let bg = if hot { Color::from_rgba(60, 170, 120, 255) } else { Color::from_rgba(42, 130, 92, 255) };
        draw_rectangle(r.x, r.y, r.w, r.h, bg);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.35));
        let label = format!("Leap {} >", session.chosen.unwrap_or(0));
        let m = measure_text(&label, None, 26, 1.0);
        draw_text(&label, r.x + r.w / 2.0 - m.width / 2.0, r.y + r.h / 2.0 + 9.0, 26.0, WHITE);
    }
    if let Some(r) = layout.back_btn {
        let hot = r.contains(mx, my);
        let bg = if hot { Color::from_rgba(90, 110, 130, 255) } else { Color::from_rgba(66, 84, 100, 255) };
        draw_rectangle(r.x, r.y, r.w, r.h, bg);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.35));
        let label = "< Swim back";
        let m = measure_text(label, None, 26, 1.0);
        draw_text(label, r.x + r.w / 2.0 - m.width / 2.0, r.y + r.h / 2.0 + 9.0, 26.0, WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robot_buddy_domain::logic::leap::{Clue, LeapPuzzle};

    fn session() -> LeapSession {
        LeapSession::new(LeapPuzzle {
            max: 12,
            pearl: 9,
            size: 3,
            count: 3,
            choices: vec![3, 6, 7],
            clue: Clue::Count { n: 3 },
        })
    }

    /// Regression: the status line used to be drawn at the panel's bottom edge,
    /// which put it underneath the buttons. Every row has to own its own band.
    #[test]
    fn the_text_rows_never_overlap_the_buttons() {
        let s = session();
        for screen in [(640.0, 480.0), (960.0, 720.0), (1600.0, 900.0)] {
            let l = layout(&s, screen);
            let top = l.panel.y;
            for (rect, _) in &l.choices {
                assert!(rect.y > top + STATUS_Y,
                    "{screen:?}: buttons must start below the status line");
                assert!(rect.y + rect.h <= l.panel.y + l.panel.h,
                    "{screen:?}: buttons must stay inside the panel");
            }
            assert!(top + STATUS_Y > top + CALL_Y, "the status sits under the call");
        }
    }

    /// Shelly's call is a full sentence; on a narrow window it has to shrink
    /// rather than run off the edge. Measured with a stand-in for macroquad's
    /// text metrics (roughly half the font size per character), since real
    /// measurement needs a live window.
    #[test]
    fn a_long_call_shrinks_to_fit_the_panel() {
        let call = "My pearl's under stone 9! You get there in 3 leaps - how big is each one?";
        let width_of = |t: &str, size: u16| t.chars().count() as f32 * size as f32 * 0.5;
        for width in [480.0f32, 640.0, 960.0, 1600.0] {
            let room = width - 32.0;
            let size = fitted_by(call, room, 24, width_of);
            assert!(width_of(call, size) <= room || size == MIN_FONT,
                "at {width}px the call still overflows at size {size}");
            assert!(size <= 24, "never bigger than asked for");
        }
        // A roomy window keeps the full-size text.
        assert_eq!(fitted_by("short", 900.0, 24, width_of), 24);
    }

    /// The panel keeps its buttons on screen at the sizes a kid actually plays at.
    #[test]
    fn buttons_stay_on_screen() {
        let s = session();
        for screen in [(480.0, 800.0), (960.0, 720.0)] {
            let l = layout(&s, screen);
            for (rect, _) in &l.choices {
                assert!(rect.x >= 0.0 && rect.x + rect.w <= screen.0,
                    "{screen:?}: a size tile runs off the edge");
            }
        }
    }
}
