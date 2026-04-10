use macroquad::prelude::*;
use robot_buddy_domain::challenge::challenge_state::{ChallengeState, ChallengeAction};
use robot_buddy_domain::learning::challenge_generator::Challenge;
use robot_buddy_domain::types::{Phase, CraStage};

use super::visuals;

// ─── LAYOUT (testable) ─────────────────────────────────

pub struct ChoiceBound {
    pub rect: (f32, f32, f32, f32), // x, y, w, h
    pub answer: i32,
    pub correct: bool,
}

pub struct ScaffoldBounds {
    pub show_me: Option<(f32, f32, f32, f32)>,
    pub tell_me: Option<(f32, f32, f32, f32)>,
}


// ─── DRAWING ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);       // #141430
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);            // #FFD54F
const ORANGE: Color = Color::new(1.0, 0.541, 0.396, 1.0);          // #FF8A65
const BLUE_BTN: Color = Color::new(0.129, 0.588, 0.953, 1.0);      // #2196F3
const GREEN_BTN: Color = Color::new(0.298, 0.686, 0.314, 1.0);     // #4CAF50
const DIM_BTN: Color = Color::new(0.216, 0.278, 0.310, 1.0);       // #37474F
const SCAFFOLD_BG: Color = Color::new(0.329, 0.431, 0.478, 1.0);   // #546E7A
const SCAFFOLD_DIM: Color = Color::new(0.271, 0.353, 0.392, 1.0);  // #455A64
const SCAFFOLD_TXT: Color = Color::new(0.690, 0.745, 0.773, 1.0);  // #B0BEC5
const SCAFFOLD_TXT_DIM: Color = Color::new(0.565, 0.643, 0.682, 1.0); // #90A4AE
const PRAISE_COLOR: Color = Color::new(1.0, 0.835, 0.310, 1.0);    // #FFD54F
const GREEN_ANS: Color = Color::new(0.412, 0.941, 0.682, 1.0);     // #69F0AE
const HINT_GRAY: Color = Color::new(0.471, 0.565, 0.604, 1.0);     // #78909C

fn round_rect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    // Center rect + corner circles for rounded appearance
    draw_rectangle(x + r, y, w - 2.0 * r, h, color);
    draw_rectangle(x, y + r, w, h - 2.0 * r, color);
    draw_circle(x + r, y + r, r, color);
    draw_circle(x + w - r, y + r, r, color);
    draw_circle(x + r, y + h - r, r, color);
    draw_circle(x + w - r, y + h - r, r, color);
}

fn round_rect_lines(x: f32, y: f32, w: f32, h: f32, _r: f32, thickness: f32, color: Color) {
    draw_rectangle_lines(x, y, w, h, thickness, color);
}

/// Render the full challenge overlay. Returns click bounds for hit testing.
pub fn draw_challenge(cs: &ChallengeState, challenge: &Challenge, time: f32) -> (Vec<ChoiceBound>, ScaffoldBounds) {
    let sw = screen_width();
    let sh = screen_height();

    // Dim background
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));

    if cs.phase == Phase::Teaching {
        return draw_teaching_phase(cs, challenge, time, sw, sh);
    }

    let panel_w = (sw - 40.0).min(650.0);
    let panel_h = if cs.hint_used { 480.0 } else { 360.0 };
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0 - 10.0;

    // Panel
    round_rect(panel_x, panel_y, panel_w, panel_h, 16.0, DARK_BG);
    round_rect_lines(panel_x, panel_y, panel_w, panel_h, 16.0, 4.0, GOLD);

    // Question
    let q_text = &cs.question.display;
    let q_size = 30.0;
    let q_w = measure_text(q_text, None, q_size as u16, 1.0).width;
    draw_text(q_text, panel_x + panel_w / 2.0 - q_w / 2.0, panel_y + 60.0, q_size, WHITE);

    // CRA visual hint (if show-me was used)
    let mut hint_offset = 0.0;
    if cs.hint_used {
        hint_offset = 70.0;
        let viz_cx = panel_x + panel_w / 2.0;
        let viz_cy = panel_y + 95.0;
        visuals::draw_visual(challenge, viz_cx, viz_cy, time);
    }

    // Feedback text
    let mut feedback_offset = 0.0;
    if cs.phase == Phase::Feedback {
        if let Some(fb) = &cs.feedback {
            feedback_offset = 35.0;
            let fb_w = measure_text(&fb.display, None, 22, 1.0).width;
            draw_text(&fb.display, panel_x + panel_w / 2.0 - fb_w / 2.0,
                panel_y + 120.0 + hint_offset, 22.0, ORANGE);
        }
    }

    // Choice buttons
    let btn_w = ((panel_w - 80.0) / 3.0).min(160.0);
    let btn_h = 70.0;
    let btn_y = panel_y + 130.0 + hint_offset + feedback_offset;
    let total_btn_w = btn_w * 3.0 + 20.0 * 2.0;
    let btn_start_x = panel_x + (panel_w - total_btn_w) / 2.0;

    let mut choice_bounds = Vec::new();
    for (i, choice) in challenge.choices.iter().enumerate() {
        let bx = btn_start_x + i as f32 * (btn_w + 20.0);
        let by = btn_y;

        let btn_color = if cs.phase == Phase::Complete && cs.correct == Some(true) {
            if choice.correct { GREEN_BTN } else { DIM_BTN }
        } else {
            BLUE_BTN
        };

        round_rect(bx, by, btn_w, btn_h, 12.0, btn_color);
        round_rect_lines(bx, by, btn_w, btn_h, 12.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.3));

        let text_size = 28.0;
        let tw = measure_text(&choice.text, None, text_size as u16, 1.0).width;
        draw_text(&choice.text, bx + btn_w / 2.0 - tw / 2.0, by + btn_h / 2.0 + 10.0, text_size, WHITE);

        // Key hint (1, 2, 3)
        let key_label = format!("{}", i + 1);
        draw_text(&key_label, bx + 8.0, by + 18.0, 14.0, Color::new(1.0, 1.0, 1.0, 0.4));

        let answer: i32 = choice.text.parse().unwrap_or(0);
        choice_bounds.push(ChoiceBound {
            rect: (bx, by, btn_w, btn_h),
            answer,
            correct: choice.correct,
        });
    }

    // Scaffold buttons (Show Me / Tell Me)
    let mut scaffold = ScaffoldBounds { show_me: None, tell_me: None };
    if cs.phase == Phase::Presented || cs.phase == Phase::Feedback {
        let scaff_y = btn_y + btn_h + 12.0;
        let scaff_btn_w = 90.0;
        let scaff_btn_h = 30.0;
        let scaff_gap = 10.0;

        // Show Me (only if not already at Concrete)
        if cs.render_hint.cra_stage != CraStage::Concrete {
            let sm_x = panel_x + panel_w / 2.0 - scaff_btn_w - scaff_gap / 2.0;
            round_rect(sm_x, scaff_y, scaff_btn_w, scaff_btn_h, 6.0, SCAFFOLD_BG);
            let sm_tw = measure_text("Show me", None, 13, 1.0).width;
            draw_text("Show me", sm_x + scaff_btn_w / 2.0 - sm_tw / 2.0, scaff_y + scaff_btn_h / 2.0 + 4.0, 13.0, SCAFFOLD_TXT);
            scaffold.show_me = Some((sm_x, scaff_y, scaff_btn_w, scaff_btn_h));
        }

        // Tell Me
        let tm_x = if cs.render_hint.cra_stage != CraStage::Concrete {
            panel_x + panel_w / 2.0 + scaff_gap / 2.0
        } else {
            panel_x + panel_w / 2.0 - scaff_btn_w / 2.0
        };
        round_rect(tm_x, scaff_y, scaff_btn_w, scaff_btn_h, 6.0, SCAFFOLD_DIM);
        let tm_tw = measure_text("Tell me", None, 13, 1.0).width;
        draw_text("Tell me", tm_x + scaff_btn_w / 2.0 - tm_tw / 2.0, scaff_y + scaff_btn_h / 2.0 + 4.0, 13.0, SCAFFOLD_TXT_DIM);
        scaffold.tell_me = Some((tm_x, scaff_y, scaff_btn_w, scaff_btn_h));
    }

    // Celebration / Dismiss
    if cs.phase == Phase::Complete {
        if cs.correct == Some(true) {
            let praises = ["AMAZING!", "WOW!", "GENIUS!", "SO SMART!", "INCREDIBLE!", "YOU GOT IT!"];
            let praise = praises[(cs.correct_answer.unsigned_abs() as usize) % praises.len()];
            let pw = measure_text(praise, None, 32, 1.0).width;
            draw_text(praise, panel_x + panel_w / 2.0 - pw / 2.0, btn_y + btn_h + 55.0, 32.0, PRAISE_COLOR);
            draw_star_burst(panel_x + panel_w / 2.0, btn_y + btn_h + 35.0, time);
        }

        // Dismiss hint (for both correct and post-teaching)
        let dismiss = "Press SPACE to continue";
        let dw = measure_text(dismiss, None, 14, 1.0).width;
        let blink = (get_time() * 4.0).sin() > 0.0;
        if blink {
            let dismiss_y = if cs.correct == Some(true) { btn_y + btn_h + 85.0 } else { btn_y + btn_h + 55.0 };
            draw_text(dismiss, panel_x + panel_w / 2.0 - dw / 2.0, dismiss_y, 14.0, HINT_GRAY);
        }
    }

    (choice_bounds, scaffold)
}

fn draw_teaching_phase(cs: &ChallengeState, challenge: &Challenge, time: f32, sw: f32, sh: f32) -> (Vec<ChoiceBound>, ScaffoldBounds) {
    let panel_w = (sw - 40.0).min(650.0);
    let panel_h = 380.0;
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0 - 10.0;

    // Panel with orange border
    round_rect(panel_x, panel_y, panel_w, panel_h, 16.0, DARK_BG);
    round_rect_lines(panel_x, panel_y, panel_w, panel_h, 16.0, 4.0, ORANGE);

    // Header
    let header = if cs.told_me { "Here's how it works!" } else { "Let's figure it out!" };
    let hw = measure_text(header, None, 16, 1.0).width;
    draw_text(header, panel_x + panel_w / 2.0 - hw / 2.0, panel_y + 30.0, 16.0, ORANGE);

    // Question
    let qw = measure_text(&cs.question.display, None, 24, 1.0).width;
    draw_text(&cs.question.display, panel_x + panel_w / 2.0 - qw / 2.0, panel_y + 65.0, 24.0, WHITE);

    // Visual walkthrough (always concrete in teaching)
    let viz_cx = panel_x + panel_w / 2.0;
    let viz_cy = panel_y + 90.0;
    visuals::draw_visual(challenge, viz_cx, viz_cy, time);

    // Answer
    let answer_text = format!("= {}", challenge.correct_answer);
    let aw = measure_text(&answer_text, None, 40, 1.0).width;
    draw_text(&answer_text, panel_x + panel_w / 2.0 - aw / 2.0, panel_y + panel_h - 70.0, 40.0, GREEN_ANS);

    // Feedback text
    if let Some(fb) = &cs.feedback {
        let fw = measure_text(&fb.display, None, 18, 1.0).width;
        draw_text(&fb.display, panel_x + panel_w / 2.0 - fw / 2.0, panel_y + panel_h - 40.0, 18.0, GOLD);
    }

    // Dismiss hint
    let dismiss = "Press SPACE or click to continue";
    let dw = measure_text(dismiss, None, 14, 1.0).width;
    draw_text(dismiss, panel_x + panel_w / 2.0 - dw / 2.0, panel_y + panel_h - 15.0, 14.0, HINT_GRAY);

    (vec![], ScaffoldBounds { show_me: None, tell_me: None })
}

fn draw_star_burst(cx: f32, cy: f32, time: f32) {
    let num_stars = 8;
    for i in 0..num_stars {
        let angle = (i as f32 / num_stars as f32) * std::f32::consts::TAU + time * 2.0;
        let dist = 30.0 + (time * 3.0).sin().abs() * 20.0;
        let sx = cx + angle.cos() * dist;
        let sy = cy + angle.sin() * dist;
        let size = 4.0 + ((time * 5.0 + i as f32).sin().abs()) * 3.0;
        let alpha = 0.5 + ((time * 4.0 + i as f32 * 0.7).sin().abs()) * 0.5;
        let color = Color::new(1.0, 0.835, 0.310, alpha);
        // Draw a simple 4-point star
        draw_line(sx - size, sy, sx + size, sy, 2.0, color);
        draw_line(sx, sy - size, sx, sy + size, 2.0, color);
    }
}

// ─── INPUT HANDLING ─────────────────────────────────────

pub fn handle_key(cs: &ChallengeState, challenge: &Challenge) -> Option<ChallengeAction> {
    // In complete or teaching phase, Space/Enter dismisses
    if cs.phase == Phase::Complete || cs.phase == Phase::Teaching {
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
            if cs.phase == Phase::Teaching {
                return Some(ChallengeAction::TeachingComplete);
            }
            // Complete → signal to caller to dismiss (handled in main)
            return None;
        }
        return None;
    }

    // Number keys 1-3 to pick choices
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3];
    for (i, key) in keys.iter().enumerate() {
        if is_key_pressed(*key) {
            if let Some(choice) = challenge.choices.get(i) {
                let answer: i32 = choice.text.parse().unwrap_or(0);
                return Some(ChallengeAction::AnswerSubmitted { answer });
            }
        }
    }

    None
}

pub fn handle_click(
    mx: f32, my: f32,
    cs: &ChallengeState,
    _challenge: &Challenge,
    choice_bounds: &[ChoiceBound],
    scaffold: &ScaffoldBounds,
) -> Option<ChallengeAction> {
    // Teaching/Complete: click anywhere to dismiss
    if cs.phase == Phase::Teaching {
        return Some(ChallengeAction::TeachingComplete);
    }
    if cs.phase == Phase::Complete {
        return None; // Signal caller to dismiss
    }

    // Show Me button
    if let Some((x, y, w, h)) = scaffold.show_me {
        if mx >= x && mx <= x + w && my >= y && my <= y + h {
            return Some(ChallengeAction::ShowMe);
        }
    }

    // Tell Me button
    if let Some((x, y, w, h)) = scaffold.tell_me {
        if mx >= x && mx <= x + w && my >= y && my <= y + h {
            return Some(ChallengeAction::TellMe);
        }
    }

    // Choice buttons
    for bound in choice_bounds {
        let (x, y, w, h) = bound.rect;
        if mx >= x && mx <= x + w && my >= y && my <= y + h {
            return Some(ChallengeAction::AnswerSubmitted { answer: bound.answer });
        }
    }

    None
}
