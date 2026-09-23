//! The challenge overlay: question, answer buttons, Show me / Tell me, the
//! celebration, and the teaching walkthrough.
//!
//! Layout is declarative (see `ui::layout`): [`layout`] is pure and returns a
//! `Frame` that `Game::step` hit-tests and `Game::render` paints, so the
//! buttons you see are exactly the buttons you can tap — a long wrapped word
//! problem pushes both down together.

use crate::prelude::*;
use robot_buddy_domain::challenge::challenge_state::{ChallengeAction, ChallengeState};
use robot_buddy_domain::learning::challenge_generator::Challenge;
use robot_buddy_domain::types::Phase;

use super::visuals;
use crate::input::FrameInput;
use crate::ui::layout::{self, col, paint, region, row, text, Align, Fit, Frame, Justify, Kind, Node};
pub use crate::ui::layout::UiRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeId {
    Panel,
    /// Teaching-screen header ("Let's figure it out!").
    Header,
    Question,
    /// The CRA visual (Show me / teaching).
    Visual,
    Feedback,
    Choice(usize),
    ChoiceKey(usize),
    ChoiceLabel(usize),
    ShowMe,
    ShowMeLabel,
    TellMe,
    TellMeLabel,
    /// Room for the praise and its star burst.
    Celebrate,
    Praise,
    /// Teaching: "= 12".
    Answer,
    Dismiss,
}

pub struct ChallengeLayout {
    pub frame: Frame<ChallengeId>,
}

impl ChallengeLayout {
    pub fn choice(&self, index: usize) -> Option<UiRect> {
        self.frame.rect(ChallengeId::Choice(index))
    }
    pub fn show_me(&self) -> Option<UiRect> {
        self.frame.rect(ChallengeId::ShowMe)
    }
    pub fn tell_me(&self) -> Option<UiRect> {
        self.frame.rect(ChallengeId::TellMe)
    }
}

const PANEL_W: f32 = 760.0;
const MARGIN: f32 = 20.0;
const PRAISES: [&str; 6] = ["AMAZING!", "WOW!", "GENIUS!", "SO SMART!", "INCREDIBLE!", "YOU GOT IT!"];

/// Replace Unicode math symbols with ASCII equivalents that render in macroquad's default font.
fn sanitize_math_text(text: &str) -> String {
    text.replace('\u{2212}', "-") // minus sign → hyphen-minus
        .replace('\u{00d7}', "x") // multiplication sign → letter x
        .replace('\u{00f7}', "/") // division sign → slash
}

/// The CRA visual's slot: the panel's full inner width (stretched), `h` tall.
/// [`layout`] measures the width first, then asks the visual how tall it is
/// at that width — the same `visuals::plan` that draws it.
fn visual(h: f32) -> Node<ChallengeId> {
    region(0.0, h).auto_w().id(ChallengeId::Visual).fixed()
}

/// A labelled scaffold button ("Show me" / "Tell me").
fn scaffold(id: ChallengeId, label_id: ChallengeId, label: &str) -> Node<ChallengeId> {
    // Preferred 150 wide; squeezes (never clips) on a narrow phone.
    layout::button(id, label_id, label, 22, Fit::shrink(14)).w(150.0).min_w(96.0).shrink(1.0)
}

fn answer_button(i: usize, label: &str) -> Node<ChallengeId> {
    col()
        .id(ChallengeId::Choice(i))
        .hit()
        .w(200.0)
        .min_w(64.0)
        .pad_xy(10.0, 4.0)
        // Key hint (1, 2, 3) top-left; the answer centered between it and a
        // matching bottom gap.
        .child(text(format!("{}", i + 1), 20, Fit::shrink(12)).id(ChallengeId::ChoiceKey(i)).fixed())
        .child(text(label, 40, Fit::shrink(18)).id(ChallengeId::ChoiceLabel(i)).center_text().grow(1.0))
        // Balances the key hint; gives way first when a short screen squeezes the button.
        .child(region(0.0, 20.0).min_h(0.0))
}

/// Lay the overlay out for `screen`. Pure.
pub fn layout(cs: &ChallengeState, challenge: &Challenge, screen: (f32, f32)) -> ChallengeLayout {
    let bounds = layout::screen_rect(screen);
    let build = |visual_h: f32| tree(cs, challenge, screen, visual_h);
    let mut frame = layout::layout(&build(0.0), bounds);
    // Two passes: the first finds how wide the visual's slot is (the panel's
    // width doesn't depend on its content), the second reserves its height
    // at that width.
    if let Some(slot) = frame.rect(ChallengeId::Visual) {
        frame = layout::layout(&build(visuals::extent(challenge, slot.w).h), bounds);
    }
    ChallengeLayout { frame }
}

fn tree(cs: &ChallengeState, challenge: &Challenge, screen: (f32, f32), visual_h: f32) -> Node<ChallengeId> {
    let sh = screen.1;
    // Short windows (640x480) get tighter spacing so a wrapped word problem,
    // the visual and feedback all fit above the buttons.
    let compact = sh < 600.0;
    let (gap, pad_top): (f32, f32) = if compact { (8.0, 18.0) } else { (14.0, 28.0) };
    let q_text = sanitize_math_text(&cs.question.display);

    let panel = if cs.phase == Phase::Teaching {
        let header = if cs.told_me { "Here's how it works!" } else { "Let's figure it out!" };
        col()
            .gap(gap.min(10.0))
            .child(text(header, 28, Fit::shrink(18)).id(ChallengeId::Header).center_text().fixed())
            .child(text(q_text, 34, Fit::shrink_then_wrap(20, 6)).id(ChallengeId::Question).center_text())
            .child(visual(visual_h))
            .child(text(format!("= {}", challenge.correct_answer), 54, Fit::shrink(28)).id(ChallengeId::Answer).center_text())
            .maybe(cs.feedback.as_ref().map(|fb| {
                text(fb.display.clone(), 24, Fit::wrap_lines(14, 2)).id(ChallengeId::Feedback).center_text()
            }))
            .child(text("Press SPACE or click to continue", 22, Fit::shrink(14)).id(ChallengeId::Dismiss).center_text().fixed())
    } else {
        let feedback = (cs.phase == Phase::Feedback).then_some(cs.feedback.as_ref()).flatten();
        // While the kid is answering, the feedback line is always laid out —
        // empty until they miss — so "Hmm, not quite!" appearing can't shove
        // the answer buttons out from under their finger.
        let answering = cs.phase == Phase::Presented || cs.phase == Phase::Feedback;
        let feedback_slot = answering.then(|| {
            let msg = feedback.map(|fb| fb.display.clone()).unwrap_or_default();
            text(msg, 28, Fit::wrap_lines(16, 2)).id(ChallengeId::Feedback).center_text().reserve_lines(2)
        });
        let scaffolds = (cs.phase == Phase::Presented || cs.phase == Phase::Feedback).then(|| {
            row()
                .gap(12.0)
                .justify(Justify::Center)
                // The row gives up height on a short screen; the buttons stretch to it.
                .align(Align::Stretch)
                .h(46.0)
                .min_h(38.0)
                // Show me — available until the visual is displayed.
                .maybe((!cs.hint_used).then(|| scaffold(ChallengeId::ShowMe, ChallengeId::ShowMeLabel, "Show me")))
                .child(scaffold(ChallengeId::TellMe, ChallengeId::TellMeLabel, "Tell me"))
        });
        let complete = cs.phase == Phase::Complete;
        let praise = (complete && cs.correct == Some(true)).then(|| {
            let praise = PRAISES[(cs.correct_answer.unsigned_abs() as usize) % PRAISES.len()];
            row()
                .id(ChallengeId::Celebrate)
                .h(100.0)
                .min_h(30.0)
                .justify(Justify::Center)
                .child(text(praise, 44, Fit::shrink(24)).id(ChallengeId::Praise).center_text())
        });
        col()
            .gap(gap)
            .child(text(q_text, 42, Fit::shrink_then_wrap(22, 6)).id(ChallengeId::Question).center_text())
            .maybe(cs.hint_used.then(|| visual(visual_h)))
            .maybe(feedback_slot)
            .child(
                row()
                    .gap(20.0)
                    .justify(Justify::Center)
                    .align(Align::Stretch)
                    .h(88.0)
                    .min_h(64.0)
                    .children(challenge.choices.iter().enumerate().map(|(i, c)| answer_button(i, &c.text))),
            )
            .maybe(scaffolds)
            .maybe(praise)
            // Dismiss hint (for both correct and post-teaching).
            .maybe(complete.then(|| text("Press SPACE to continue", 22, Fit::shrink(14)).id(ChallengeId::Dismiss).center_text().fixed()))
    };

    // PANEL_W wide, or the whole screen if that's narrower.
    let panel = panel.id(ChallengeId::Panel).w_pct(1.0).max_w(PANEL_W).pad_edges(24.0, pad_top, 24.0, 20.0);
    layout::centered_on_screen(panel, MARGIN)
}

// ─── DRAWING ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0); // #141430
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0); // #FFD54F
const ORANGE: Color = Color::new(1.0, 0.541, 0.396, 1.0); // #FF8A65
const BLUE_BTN: Color = Color::new(0.129, 0.588, 0.953, 1.0); // #2196F3
const GREEN_BTN: Color = Color::new(0.298, 0.686, 0.314, 1.0); // #4CAF50
const DIM_BTN: Color = Color::new(0.216, 0.278, 0.310, 1.0); // #37474F
const SCAFFOLD_BG: Color = Color::new(0.329, 0.431, 0.478, 1.0); // #546E7A
const SCAFFOLD_DIM: Color = Color::new(0.271, 0.353, 0.392, 1.0); // #455A64
const SCAFFOLD_TXT: Color = Color::new(0.690, 0.745, 0.773, 1.0); // #B0BEC5
const SCAFFOLD_TXT_DIM: Color = Color::new(0.565, 0.643, 0.682, 1.0); // #90A4AE
const PRAISE_COLOR: Color = Color::new(1.0, 0.835, 0.310, 1.0); // #FFD54F
const GREEN_ANS: Color = Color::new(0.412, 0.941, 0.682, 1.0); // #69F0AE
const HINT_GRAY: Color = Color::new(0.471, 0.565, 0.604, 1.0); // #78909C

/// Paint the overlay from its frame.
pub fn draw(layout: &ChallengeLayout, cs: &ChallengeState, challenge: &Challenge, time: f32) {
    let f = &layout.frame;
    paint::dim(f.bounds, 0.5);
    let teaching = cs.phase == Phase::Teaching;
    let solved = cs.phase == Phase::Complete && cs.correct == Some(true);
    for el in f.elements() {
        let Some(id) = el.id else { continue };
        let r = el.rect;
        if let Kind::Text(t) = &el.kind {
            match id {
                ChallengeId::Header => paint::text(t, ORANGE),
                ChallengeId::Feedback if teaching => paint::text(t, GOLD),
                ChallengeId::Feedback => paint::text(t, ORANGE),
                ChallengeId::ChoiceKey(_) => paint::text(t, Color::new(1.0, 1.0, 1.0, 0.4)),
                ChallengeId::ShowMeLabel => paint::text(t, SCAFFOLD_TXT),
                ChallengeId::TellMeLabel => paint::text(t, SCAFFOLD_TXT_DIM),
                ChallengeId::Praise => paint::text(t, PRAISE_COLOR),
                ChallengeId::Answer => paint::text(t, GREEN_ANS),
                ChallengeId::Dismiss if teaching => paint::text(t, HINT_GRAY),
                ChallengeId::Dismiss => {
                    if paint::blink(4.0) {
                        paint::text(t, HINT_GRAY)
                    }
                }
                _ => paint::text(t, WHITE),
            }
            continue;
        }
        match id {
            ChallengeId::Panel => {
                paint::round_rect(r, 16.0, DARK_BG);
                paint::outline(r, 4.0, if teaching { ORANGE } else { GOLD });
            }
            ChallengeId::Visual => visuals::draw(challenge, r),
            ChallengeId::Choice(i) => {
                let correct = challenge.choices.get(i).is_some_and(|c| c.correct);
                let color = if solved { if correct { GREEN_BTN } else { DIM_BTN } } else { BLUE_BTN };
                paint::round_rect(r, 12.0, color);
                paint::outline(r, 2.0, Color::new(1.0, 1.0, 1.0, 0.3));
            }
            ChallengeId::ShowMe => paint::round_rect(r, 6.0, SCAFFOLD_BG),
            ChallengeId::TellMe => paint::round_rect(r, 6.0, SCAFFOLD_DIM),
            ChallengeId::Celebrate => {
                let praise = f.rect(ChallengeId::Praise).unwrap_or(r);
                draw_star_burst(r, praise.center(), time);
            }
            _ => {}
        }
    }
}

/// Stars circling the praise, kept inside the celebration box.
fn draw_star_burst(area: UiRect, (cx, cy): (f32, f32), time: f32) {
    let c = paint::canvas(area);
    let room = ((area.h / 2.0).min(area.w / 2.0) - 8.0).max(0.0);
    let num_stars = 8;
    for i in 0..num_stars {
        let angle = (i as f32 / num_stars as f32) * std::f32::consts::TAU + time * 2.0;
        let dist = (30.0 + (time * 3.0).sin().abs() * 20.0).min(room);
        let sx = cx + angle.cos() * dist;
        let sy = cy + angle.sin() * dist;
        let size = 4.0 + ((time * 5.0 + i as f32).sin().abs()) * 3.0;
        let alpha = 0.5 + ((time * 4.0 + i as f32 * 0.7).sin().abs()) * 0.5;
        let color = Color::new(1.0, 0.835, 0.310, alpha);
        // A simple 4-point star.
        c.line(sx - size, sy, sx + size, sy, 2.0, color);
        c.line(sx, sy - size, sx, sy + size, 2.0, color);
    }
}

// ─── INPUT HANDLING ─────────────────────────────────────

pub fn handle_key(cs: &ChallengeState, challenge: &Challenge, input: &FrameInput) -> Option<ChallengeAction> {
    // In complete or teaching phase, Space/Enter dismisses
    if cs.phase == Phase::Complete || cs.phase == Phase::Teaching {
        if input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) {
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
        if input.pressed(*key) {
            if let Some(choice) = challenge.choices.get(i) {
                let answer: i32 = choice.text.parse().unwrap_or(0);
                return Some(ChallengeAction::AnswerSubmitted { answer });
            }
        }
    }

    None
}

pub fn handle_click(
    mx: f32,
    my: f32,
    cs: &ChallengeState,
    challenge: &Challenge,
    layout: &ChallengeLayout,
) -> Option<ChallengeAction> {
    // Teaching/Complete: click anywhere to dismiss
    if cs.phase == Phase::Teaching {
        return Some(ChallengeAction::TeachingComplete);
    }
    if cs.phase == Phase::Complete {
        return None; // Signal caller to dismiss
    }
    match layout.frame.hit_at(mx, my)? {
        ChallengeId::ShowMe => Some(ChallengeAction::ShowMe),
        ChallengeId::TellMe => Some(ChallengeAction::TellMe),
        ChallengeId::Choice(i) => {
            let answer: i32 = challenge.choices.get(i)?.text.parse().unwrap_or(0);
            Some(ChallengeAction::AnswerSubmitted { answer })
        }
        _ => None,
    }
}
