//! The Goyish Map's number-bond space shooter — full-screen renderer.
//!
//! Pure drawing from a domain `ShooterSession`. The logical play field is
//! `0..FIELD_W` × `0..FIELD_H`; everything maps into a centered play rect. There
//! is no clock and no lives on screen (Invariant 4) — just the target, the
//! aliens, a shield gauge, and gentle guidance.

use crate::prelude::*;
use robot_buddy_domain::logic::shooter::{ShooterSession, ShooterPhase, FIELD_W, FIELD_H, BREACH_Y, DOT_MAX};
use robot_buddy_domain::types::CraStage;

const VOID: Color = color_u8!(8, 10, 24, 255);
const PIP: Color = color_u8!(255, 238, 170, 255);

/// The centered play rect (x, y, w, h) the field maps into. Shared by `draw`
/// and `field_x_at` so click-to-aim lands exactly where things are drawn.
fn play_rect(screen: (f32, f32)) -> (f32, f32, f32, f32) {
    let (sw, sh) = screen;
    let pad = 24.0;
    let top = 78.0;
    (pad, top, sw - 2.0 * pad, sh - top - 46.0)
}

/// Map a screen-space click to a logical field column, or `None` if the click
/// landed outside the play area. Used for click/tap-to-shoot.
pub fn field_x_at(screen: (f32, f32), mx: f32, my: f32) -> Option<f32> {
    let (px, py, pw, ph) = play_rect(screen);
    if mx < px || mx > px + pw || my < py || my > py + ph {
        return None;
    }
    Some(((mx - px) / pw * FIELD_W).clamp(0.0, FIELD_W))
}

pub fn draw(session: &ShooterSession, screen: (f32, f32), time: f32) {
    let (sw, sh) = screen;

    // Backdrop: deep space with a drifting starfield.
    draw_rectangle(0.0, 0.0, sw, sh, VOID);
    draw_starfield(sw, sh, time);

    // Centered play rect, leaving room for the banner up top and hints below.
    let (play_x, play_y, play_w, play_h) = play_rect(screen);
    let to_screen = |fx: f32, fy: f32| -> (f32, f32) {
        (play_x + fx / FIELD_W * play_w, play_y + fy / FIELD_H * play_h)
    };
    let unit = play_w / FIELD_W; // one logical x-unit in pixels
    let alien_r = (unit * 5.0).clamp(16.0, 30.0);

    // Faint frame + the danger line the aliens must not cross.
    draw_rectangle_lines(play_x, play_y, play_w, play_h, 2.0, color_u8!(60, 70, 110, 120));
    let (_, breach_py) = to_screen(0.0, BREACH_Y);
    let dashed = color_u8!(120, 90, 150, 90);
    let mut dx = play_x;
    while dx < play_x + play_w {
        draw_line(dx, breach_py, (dx + 14.0).min(play_x + play_w), breach_py, 2.0, dashed);
        dx += 26.0;
    }

    // ── Target banner ── shown as a numeral (Abstract/Representational) or as a
    // row of pips to count (Concrete), matching how the aliens read.
    draw_target_banner(session.target, session.representation, sw);

    // ── Aliens ──
    for a in &session.aliens {
        let (cx, cy) = to_screen(a.x, a.y);
        draw_alien(cx, cy, alien_r, a.value, a.selected, session.representation, time);
    }

    // ── Bolts in flight ──
    for shot in &session.shots {
        let (bx, by) = to_screen(shot.x, shot.y);
        let len = (unit * 4.0).clamp(10.0, 22.0);
        // Glowing tail behind a bright core.
        draw_line(bx, by, bx, by + len, 6.0, color_u8!(120, 230, 255, 70));
        draw_line(bx, by, bx, by + len, 3.0, color_u8!(200, 250, 255, 230));
        draw_circle(bx, by, 3.5, color_u8!(255, 255, 255, 255));
    }

    // ── Ship on the bottom rail ──
    let (ship_x, ship_y) = to_screen(session.ship_x, FIELD_H);
    draw_ship(ship_x, ship_y - alien_r * 0.6, alien_r);

    // Aiming guide: a soft beam up the ship's column so kids see where a shot goes.
    draw_line(ship_x, ship_y - alien_r, ship_x, play_y, 2.0, color_u8!(120, 200, 255, 40));

    // ── Shield gauge (top-left) ──
    draw_shield(play_x + 6.0, 30.0, session.shield, session.max_shield);

    // ── Score (top-right) ──
    let score_label = format!("★ {}", session.score);
    let score_size = 26.0;
    let sw_txt = measure_text(&score_label, None, score_size as u16, 1.0).width;
    draw_text(&score_label, play_x + play_w - sw_txt - 6.0, 34.0, score_size,
              color_u8!(180, 220, 255, 255));

    // ── Bottom guidance / states ──
    if session.phase == ShooterPhase::Complete {
        centered_banner(sw, sh, "ALL CLEAR!", color_u8!(140, 240, 160, 255));
    } else if session.shield == 0 {
        // Shield empty: the aliens hover (drift frozen) — reassure, don't scare.
        let msg = "Shield low — clear a pair to power back up!";
        hint(sw, sh, msg, color_u8!(255, 200, 120, 255));
    } else {
        hint(sw, sh, "TAP a target  \u{2022}  or \u{2190}\u{2192} + SPACE  \u{2022}  ESC leave",
             color_u8!(150, 165, 210, 220));
    }
}

/// The target banner. Concrete learners get a countable pip row; everyone else
/// gets the numeral.
fn draw_target_banner(target: u32, rep: CraStage, sw: f32) {
    let gold = color_u8!(255, 214, 90, 255);
    if rep == CraStage::Concrete && target <= DOT_MAX {
        let word = "MAKE";
        let size = 40.0;
        let ww = measure_text(word, None, size as u16, 1.0).width;
        let pip_span = target as f32 * 20.0;
        let total = ww + 16.0 + pip_span;
        let x0 = sw / 2.0 - total / 2.0;
        draw_text(word, x0, 52.0, size, gold);
        // A single row of dots to count, to the right of the word.
        let pip_cx = x0 + ww + 16.0 + pip_span / 2.0;
        draw_pips_row(pip_cx, 40.0, target, PIP);
    } else {
        let label = format!("MAKE {}", target);
        let size = 44.0;
        let tw = measure_text(&label, None, size as u16, 1.0).width;
        draw_text(&label, sw / 2.0 - tw / 2.0, 52.0, size, gold);
    }
}

fn draw_alien(cx: f32, cy: f32, r: f32, value: u32, selected: bool, rep: CraStage, time: f32) {
    let bob = (time * 2.2 + cx * 0.05).sin() * 2.0;
    let cy = cy + bob;

    // Selection glow ring.
    if selected {
        let pulse = (time * 6.0).sin() * 0.5 + 0.5;
        draw_circle(cx, cy, r + 5.0 + pulse * 3.0, color_u8!(255, 240, 130, 70));
        draw_circle_lines(cx, cy, r + 4.0, 2.5, color_u8!(255, 240, 130, 220));
    }

    // A little green alien — brighter when tagged.
    let body = if selected { color_u8!(150, 235, 130, 255) } else { color_u8!(110, 200, 110, 255) };
    let dark = color_u8!(24, 60, 30, 255);

    // Wobbling antennae with glowing tips.
    let wob = (time * 4.0 + cx * 0.1).sin() * 2.0;
    for side in [-1.0_f32, 1.0] {
        let ax = cx + side * r * 0.42;
        let tx = cx + side * r * 0.62;
        let ty = cy - r * 1.05 + wob;
        draw_line(ax, cy - r * 0.55, tx, ty, 2.5, body);
        draw_circle(tx, ty, 3.0, color_u8!(255, 240, 150, 255));
    }

    // Round body + a couple of little feet.
    draw_circle(cx - r * 0.4, cy + r * 0.7, r * 0.22, body);
    draw_circle(cx + r * 0.4, cy + r * 0.7, r * 0.22, body);
    draw_circle(cx, cy, r * 0.92, body);

    // Big friendly eyes near the top.
    let eye_y = cy - r * 0.42;
    for side in [-1.0_f32, 1.0] {
        let ex = cx + side * r * 0.36;
        draw_circle(ex, eye_y, r * 0.2, color_u8!(250, 255, 250, 255));
        draw_circle(ex + side * r * 0.05, eye_y, r * 0.09, dark);
    }

    // The value sits on the belly, below the eyes. How it reads follows the
    // learner's CRA stage — but above DOT_MAX, dots are harder to count than the
    // numeral, so we always show the numeral (the domain floors most big-number
    // bands out of Concrete already; this is the per-value safety net).
    let face_cy = cy + r * 0.2;
    let dots_ok = value <= DOT_MAX;
    match rep {
        // Bare numeral — the abstract symbol.
        CraStage::Abstract => draw_numeral(cx, face_cy, r * 0.85, value),
        // Grouped dots plus a small numeral bridging count → symbol.
        CraStage::Representational if dots_ok => {
            draw_pips_grid(cx, face_cy - r * 0.05, r * 0.5, value, dark);
            let label = value.to_string();
            let size = (r * 0.5).max(10.0);
            let m = measure_text(&label, None, size as u16, 1.0);
            draw_text(&label, cx - m.width / 2.0, cy + r * 0.92, size, dark);
        }
        // Pure countable objects — no symbol at all.
        CraStage::Concrete if dots_ok => draw_pips_grid(cx, face_cy, r * 0.55, value, dark),
        // Too big for dots → fall back to the numeral.
        _ => draw_numeral(cx, face_cy, r * 0.85, value),
    }
}

fn draw_numeral(cx: f32, cy: f32, r: f32, value: u32) {
    let label = value.to_string();
    let size = (r * 1.15).max(18.0);
    let m = measure_text(&label, None, size as u16, 1.0);
    draw_text(&label, cx - m.width / 2.0, cy + m.height / 2.0 - 1.0, size, color_u8!(20, 16, 40, 255));
}

/// Lay `count` dots in a centered grid (rows of up to 5) within a box of
/// half-extent `half` around (cx, cy).
fn draw_pips_grid(cx: f32, cy: f32, half: f32, count: u32, color: Color) {
    if count == 0 { return; }
    let cols = count.min(5);
    let rows = (count + cols - 1) / cols;
    let gx = (2.0 * half) / cols as f32;
    let gy = (2.0 * half) / rows as f32;
    let dot = (gx.min(gy) * 0.28).clamp(1.5, 4.5);
    let y0 = cy - half + gy * 0.5;
    let mut left = count;
    for row in 0..rows {
        let in_row = left.min(cols);
        let row_w = in_row as f32 * gx;
        let rx0 = cx - row_w / 2.0 + gx * 0.5;
        for c in 0..in_row {
            draw_circle(rx0 + c as f32 * gx, y0 + row as f32 * gy, dot, color);
        }
        left -= in_row;
    }
}

/// A single horizontal row of `count` dots centered on (cx, cy).
fn draw_pips_row(cx: f32, cy: f32, count: u32, color: Color) {
    if count == 0 { return; }
    let step = 20.0;
    let x0 = cx - (count as f32 * step) / 2.0 + step / 2.0;
    for i in 0..count {
        draw_circle(x0 + i as f32 * step, cy, 5.0, color);
    }
}

fn draw_ship(cx: f32, cy: f32, r: f32) {
    let hull = color_u8!(90, 200, 230, 255);
    let trim = color_u8!(230, 250, 255, 255);
    // The player's rocket, pointing up at the alien swarm.
    draw_triangle(
        vec2(cx, cy - r * 0.9),
        vec2(cx - r * 0.7, cy + r * 0.6),
        vec2(cx + r * 0.7, cy + r * 0.6),
        hull,
    );
    draw_circle(cx, cy - r * 0.1, r * 0.22, trim);
    // Thruster flame.
    draw_triangle(
        vec2(cx - r * 0.3, cy + r * 0.6),
        vec2(cx + r * 0.3, cy + r * 0.6),
        vec2(cx, cy + r * 1.0),
        color_u8!(255, 170, 80, 220),
    );
}

fn draw_shield(x: f32, y: f32, shield: u8, max: u8) {
    draw_text("Shield", x, y, 22.0, color_u8!(150, 165, 210, 255));
    let base_x = x + measure_text("Shield", None, 22, 1.0).width + 10.0;
    let pip_r = 7.0;
    for i in 0..max {
        let px = base_x + i as f32 * (pip_r * 2.0 + 6.0);
        let py = y - pip_r;
        if i < shield {
            draw_circle(px, py, pip_r, color_u8!(90, 210, 255, 255));
            draw_circle(px, py, pip_r, color_u8!(255, 255, 255, 40));
        } else {
            draw_circle_lines(px, py, pip_r, 2.0, color_u8!(80, 90, 130, 200));
        }
    }
}

fn draw_starfield(sw: f32, sh: f32, time: f32) {
    // Deterministic scattered stars with a slow twinkle.
    let mut i = 0.0_f32;
    while i < 90.0 {
        let sx = ((i * 71.3).sin() * 0.5 + 0.5) * sw;
        let sy = ((i * 129.1).cos() * 0.5 + 0.5) * sh;
        let tw = ((time * 1.5 + i * 0.7).sin() * 0.5 + 0.5) * 0.6 + 0.2;
        draw_circle(sx, sy, 1.2, Color::new(1.0, 1.0, 0.95, tw));
        i += 1.0;
    }
}

fn hint(sw: f32, sh: f32, msg: &str, color: Color) {
    let size = 22.0;
    let w = measure_text(msg, None, size as u16, 1.0).width;
    draw_text(msg, sw / 2.0 - w / 2.0, sh - 18.0, size, color);
}

fn centered_banner(sw: f32, sh: f32, msg: &str, color: Color) {
    let size = 56.0;
    let w = measure_text(msg, None, size as u16, 1.0).width;
    // Soft plate behind the text.
    draw_rectangle(sw / 2.0 - w / 2.0 - 24.0, sh / 2.0 - 48.0, w + 48.0, 78.0,
                   color_u8!(10, 14, 30, 200));
    draw_text(msg, sw / 2.0 - w / 2.0, sh / 2.0 + 8.0, size, color);
    let sub = "Tap to continue";
    let subs = 22.0;
    let sw2 = measure_text(sub, None, subs as u16, 1.0).width;
    draw_text(sub, sw / 2.0 - sw2 / 2.0, sh / 2.0 + 40.0, subs, color_u8!(180, 190, 220, 220));
}
