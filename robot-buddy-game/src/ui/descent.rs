//! The descent minigame — a vertical shaft drawn as a real number line, with
//! kick buttons under it. Pure layout + render; the dive rules live in
//! `robot_buddy_domain::logic::descent`.
//!
//! The shaft runs top (surface, depth 0) to bottom (the floor), with every mark
//! labelled. The trench door glows at its depth; rock shelves jut out of the
//! wall at theirs. The diver token sits at the current depth, so the kid can
//! see "I'm at 7, the door's at 12" as a distance rather than a subtraction.

use crate::prelude::*;
use robot_buddy_domain::logic::descent::{DiveNudge, DivePhase, DiveSession};

use crate::input::FrameInput;
use crate::ui::shop::UiRect;

pub struct KickButton {
    pub rect: UiRect,
    pub n: u8,
    /// True for a sink (down) kick, false for a rise (up) kick.
    pub down: bool,
}

pub struct DescentLayout {
    pub panel: UiRect,
    /// The shaft's drawing column: x centre, top y, bottom y.
    pub shaft: (f32, f32, f32),
    pub kicks: Vec<KickButton>,
    pub leave_btn: UiRect,
}

impl DescentLayout {
    /// Screen y of a depth mark.
    pub fn y_of(&self, depth: u8, floor: u8) -> f32 {
        let (_, top, bottom) = self.shaft;
        if floor == 0 {
            return top;
        }
        top + (bottom - top) * (depth as f32 / floor as f32)
    }
}

pub fn layout(session: &DiveSession, screen: (f32, f32)) -> DescentLayout {
    let (sw, sh) = screen;
    let panel_w = (sw - 40.0).min(620.0);
    let panel_h = (sh - 40.0).min(680.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;

    // Buttons live in a two-row block at the bottom; the shaft gets the rest.
    let btn_h = 54.0;
    let gap = 10.0;
    let rows_h = btn_h * 2.0 + gap;
    let shaft_top = panel_y + 74.0;
    let shaft_bottom = panel_y + panel_h - rows_h - 74.0;
    let shaft_x = panel_x + panel_w * 0.42;

    let n = session.puzzle.kicks.len().max(1);
    let block_w = panel_w - 64.0;
    let btn_w = ((block_w - (n as f32 - 1.0) * gap) / n as f32).min(120.0).max(56.0);
    let row_w = n as f32 * btn_w + (n as f32 - 1.0) * gap;
    let start_x = panel_x + (panel_w - row_w) / 2.0;
    let down_y = panel_y + panel_h - rows_h - 56.0;
    let up_y = down_y + btn_h + gap;

    let mut kicks = Vec::with_capacity(n * 2);
    for (i, &k) in session.puzzle.kicks.iter().enumerate() {
        let x = start_x + i as f32 * (btn_w + gap);
        kicks.push(KickButton {
            rect: UiRect { x, y: down_y, w: btn_w, h: btn_h },
            n: k,
            down: true,
        });
        kicks.push(KickButton {
            rect: UiRect { x, y: up_y, w: btn_w, h: btn_h },
            n: k,
            down: false,
        });
    }

    DescentLayout {
        panel: UiRect { x: panel_x, y: panel_y, w: panel_w, h: panel_h },
        shaft: (shaft_x, shaft_top, shaft_bottom),
        kicks,
        leave_btn: UiRect { x: panel_x + panel_w - 132.0, y: panel_y + panel_h - 50.0, w: 116.0, h: 38.0 },
    }
}

pub enum DescentInput {
    Sink(u8),
    Rise(u8),
    Leave,
}

pub fn handle_click(mx: f32, my: f32, layout: &DescentLayout) -> Option<DescentInput> {
    if layout.leave_btn.contains(mx, my) {
        return Some(DescentInput::Leave);
    }
    layout.kicks.iter().find(|b| b.rect.contains(mx, my)).map(|b| {
        if b.down { DescentInput::Sink(b.n) } else { DescentInput::Rise(b.n) }
    })
}

/// Number keys sink; holding Shift (or the up-arrow modifier keys) rises, so a
/// keyboard player can climb back out without hunting for the mouse.
pub fn handle_key(input: &FrameInput, session: &DiveSession) -> Option<DescentInput> {
    if input.pressed(KeyCode::Escape) {
        return Some(DescentInput::Leave);
    }
    let keys = [
        KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5,
    ];
    let rising = input.down(KeyCode::LeftShift) || input.down(KeyCode::RightShift)
        || input.down(KeyCode::Up);
    for (i, key) in keys.iter().take(session.puzzle.kicks.len()).enumerate() {
        if input.pressed(*key) {
            let n = session.puzzle.kicks[i];
            return Some(if rising { DescentInput::Rise(n) } else { DescentInput::Sink(n) });
        }
    }
    None
}

// ─── Drawing ────────────────────────────────────────────

const WATER_TOP: Color = Color::new(0.13, 0.36, 0.50, 1.0);
const WATER_BOTTOM: Color = Color::new(0.03, 0.09, 0.20, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const ROCK: Color = Color::new(0.35, 0.30, 0.26, 1.0);

/// Largest size at or below `max` that fits `text` into `width`, measured with
/// `width_of`. Split out so the shrink loop is testable without a window.
fn fitted_by(text: &str, width: f32, max: u16, width_of: impl Fn(&str, u16) -> f32) -> u16 {
    let mut size = max;
    while size > 11 && width_of(text, size) > width {
        size -= 1;
    }
    size
}

fn fitted(text: &str, width: f32, max: u16) -> u16 {
    fitted_by(text, width, max, |t, size| measure_text(t, None, size, 1.0).width)
}

/// Draw `text` centered on `y`, shrunk to fit the panel.
fn centered(text: &str, p: UiRect, y: f32, max: u16, color: Color) {
    let room = p.w - 32.0;
    let size = fitted(text, room, max);
    let w = measure_text(text, None, size, 1.0).width;
    draw_text(text, p.x + p.w / 2.0 - w / 2.0, y, size as f32, color);
}

pub fn draw(session: &DiveSession, layout: &DescentLayout, message: Option<&str>, time: f32) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.55));

    let p = layout.panel;
    let floor = session.puzzle.floor;
    // Water gets darker with depth — bands rather than a real gradient.
    let steps = 14;
    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let c = Color::new(
            WATER_TOP.r + (WATER_BOTTOM.r - WATER_TOP.r) * t,
            WATER_TOP.g + (WATER_BOTTOM.g - WATER_TOP.g) * t,
            WATER_TOP.b + (WATER_BOTTOM.b - WATER_TOP.b) * t,
            1.0,
        );
        draw_rectangle(p.x, p.y + p.h * t, p.w, p.h / steps as f32 + 1.0, c);
    }
    draw_rectangle_lines(p.x, p.y, p.w, p.h, 4.0, GOLD);

    centered("Dive to the trench!", p, p.y + 40.0, 30, GOLD);
    let goal = format!("The door is {} marks down.", session.puzzle.door);
    centered(&goal, p, p.y + 66.0, 22, WHITE);

    let (sx, top, bottom) = layout.shaft;
    // Shaft walls.
    draw_line(sx - 34.0, top, sx - 34.0, bottom, 3.0, Color::new(0.20, 0.28, 0.30, 1.0));
    draw_line(sx + 34.0, top, sx + 34.0, bottom, 3.0, Color::new(0.20, 0.28, 0.30, 1.0));

    // Depth marks. Every mark gets a tick; labels thin out on deep shafts so
    // the numbers never collide.
    let label_every = if floor > 20 { 5 } else if floor > 12 { 2 } else { 1 };
    for d in 0..=floor {
        let y = layout.y_of(d, floor);
        let is_shelf = session.puzzle.is_shelf(d);
        let is_door = d == session.puzzle.door;
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
    let dy = layout.y_of(session.depth, floor) + (time * 2.0).sin() * 2.0;
    draw_circle(sx, dy, 11.0, Color::from_rgba(255, 204, 128, 255));
    draw_circle_lines(sx, dy, 15.0, 2.0, Color::new(0.75, 0.95, 1.0, 0.8));
    for i in 0..3 {
        let t = (time * 1.6 + i as f32 * 0.6) % 1.0;
        draw_circle(sx + 9.0 + i as f32 * 2.0, dy - 14.0 - t * 26.0, 2.0 + t * 1.5,
            Color::new(0.8, 0.95, 1.0, 0.5 * (1.0 - t)));
    }

    // Kick buttons.
    for b in &layout.kicks {
        let r = b.rect;
        let bg = if b.down {
            Color::from_rgba(33, 96, 170, 255)
        } else {
            Color::from_rgba(54, 88, 104, 255)
        };
        draw_rectangle(r.x, r.y, r.w, r.h, bg);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.3));
        let label = if b.down { format!("v {}", b.n) } else { format!("^ {}", b.n) };
        let m = measure_text(&label, None, 26, 1.0);
        draw_text(&label, r.x + r.w / 2.0 - m.width / 2.0, r.y + r.h / 2.0 + 9.0, 26.0, WHITE);
    }

    let msg = message.unwrap_or(match session.phase {
        DivePhase::Landed => "You found the door!",
        _ => match session.nudge {
            DiveNudge::Bumped => "Bonk! That ledge won't hold you — try another way down.",
            DiveNudge::Bottomed => "That's the sea floor! Kick back up to the door.",
            DiveNudge::None => "Kick down to land right on the door.",
        },
    });
    centered(msg, p, bottom + 32.0, 22, GOLD);

    let c = layout.leave_btn;
    draw_rectangle(c.x, c.y, c.w, c.h, Color::new(0.329, 0.431, 0.478, 1.0));
    draw_rectangle_lines(c.x, c.y, c.w, c.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.3));
    let cl = "Swim up";
    let clw = measure_text(cl, None, 22, 1.0).width;
    draw_text(cl, c.x + c.w / 2.0 - clw / 2.0, c.y + c.h / 2.0 + 8.0, 22.0, WHITE);
}
