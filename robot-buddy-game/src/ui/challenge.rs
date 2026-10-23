//! The challenge overlay: question, answer buttons, Show me / Tell me, the
//! celebration, and the teaching walkthrough.
//!
//! [`ChallengeView`] is what the game shows; [`ChallengePanel`] turns it into
//! Bevy UI. Taps come back as [`ChallengeId`]s, so the buttons you see are
//! exactly the buttons you can tap — a long wrapped word problem pushes both
//! down together.

use std::sync::Arc;

use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::{
    default, AlignItems, ChildSpawner, Children, JustifyContent, Node, PositionType, Spawn, SpawnRelated,
    UiRect as Edges,
};
use bevy::text::Justify;
use bevy::ui::{percent, px, BackgroundColor, BorderColor, BorderRadius};

use crate::prelude::*;
use robot_buddy_domain::challenge::challenge_state::{ChallengeAction, ChallengeState};
use robot_buddy_domain::learning::challenge_generator::Challenge;
use robot_buddy_domain::types::Phase;

use super::kit::{self, backdrop, button, col, fitted, text_centered, Art, Blink, HeightFor, Panel};
use super::views::UiViews;
use super::{visuals, ArtId, UiAction};
use crate::input::FrameInput;
pub use crate::ui::rect::UiRect;

/// The challenge's buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeId {
    Choice(usize),
    ShowMe,
    TellMe,
}

const PANEL_W: f32 = 760.0;
const MARGIN: f32 = 20.0;
const PRAISES: [&str; 6] = ["AMAZING!", "WOW!", "GENIUS!", "SO SMART!", "INCREDIBLE!", "YOU GOT IT!"];

/// Replace Unicode math symbols with ASCII equivalents (a holdover from
/// before the bundled font; the font has −, × and ÷ now).
fn sanitize_math_text(text: &str) -> String {
    text.replace('\u{2212}', "-") // minus sign → hyphen-minus
        .replace('\u{00d7}', "x") // multiplication sign → letter x
        .replace('\u{00f7}', "/") // division sign → slash
}

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

/// Everything the overlay shows, read off the challenge state.
#[derive(Clone)]
pub struct ChallengeView {
    pub phase: Phase,
    pub told_me: bool,
    pub hint_used: bool,
    pub correct: Option<bool>,
    pub question: String,
    pub feedback: Option<String>,
    /// Answer labels, and which one is right.
    pub choices: Vec<(String, bool)>,
    pub correct_answer: i32,
    /// A short window gets tighter spacing.
    pub compact: bool,
    /// For the visual (Show me / teaching), which is drawn as art.
    pub challenge: Arc<Challenge>,
}

impl PartialEq for ChallengeView {
    /// The visual is a function of the question, so it isn't compared.
    fn eq(&self, o: &Self) -> bool {
        (self.phase, self.told_me, self.hint_used, self.correct, &self.question, &self.feedback)
            == (o.phase, o.told_me, o.hint_used, o.correct, &o.question, &o.feedback)
            && (&self.choices, self.correct_answer, self.compact) == (&o.choices, o.correct_answer, o.compact)
    }
}

impl ChallengeView {
    pub fn new(cs: &ChallengeState, challenge: &Challenge, screen: (f32, f32)) -> Self {
        ChallengeView {
            phase: cs.phase,
            told_me: cs.told_me,
            hint_used: cs.hint_used,
            correct: cs.correct,
            question: sanitize_math_text(&cs.question.display),
            feedback: cs.feedback.as_ref().map(|f| f.display.clone()),
            choices: challenge.choices.iter().map(|c| (c.text.clone(), c.correct)).collect(),
            correct_answer: challenge.correct_answer,
            compact: screen.1 < 600.0,
            challenge: Arc::new(challenge.clone()),
        }
    }

    fn teaching(&self) -> bool {
        self.phase == Phase::Teaching
    }

    /// What the kid can tap right now.
    pub fn buttons(&self) -> Vec<ChallengeId> {
        if self.teaching() || self.phase == Phase::Complete {
            return vec![];
        }
        let mut b: Vec<_> = (0..self.choices.len()).map(ChallengeId::Choice).collect();
        if !self.hint_used {
            b.push(ChallengeId::ShowMe);
        }
        b.push(ChallengeId::TellMe);
        b
    }
}

/// The overlay as a Bevy UI panel.
pub struct ChallengePanel;

impl Panel for ChallengePanel {
    type View = ChallengeView;
    const Z: i32 = 30;
    fn view(views: &UiViews) -> Option<&ChallengeView> {
        views.challenge.as_ref()
    }
    fn bundle(v: &ChallengeView) -> impl bevy::prelude::Bundle {
        let v = v.clone();
        (backdrop(0.5, MARGIN), Children::spawn(SpawnWith(move |p: &mut ChildSpawner| spawn_panel(p, &v))))
    }
}

fn spawn_panel(p: &mut ChildSpawner, v: &ChallengeView) {
    // Short windows (640x480) get tighter spacing so a wrapped word problem,
    // the visual and feedback all fit above the buttons.
    let (gap, pad_top): (f32, f32) = if v.compact { (8.0, 18.0) } else { (14.0, 28.0) };
    let teaching = v.teaching();
    let panel = Node {
        // PANEL_W wide, or the whole screen if that's narrower. min_height 0:
        // the panel may be shorter than its children's preferred heights
        // (their own minimums then apply) — the CSS "min-height: 0" rule.
        width: percent(100),
        max_width: px(PANEL_W),
        max_height: percent(100),
        min_height: px(0),
        padding: Edges::new(px(24), px(24), px(pad_top), px(20)),
        row_gap: px(if teaching { gap.min(10.0) } else { gap }),
        border: Edges::all(px(4)),
        border_radius: BorderRadius::all(px(16)),
        ..col()
    };
    let outline = if teaching { ORANGE } else { GOLD };
    p.spawn((panel, BackgroundColor(kit::color(DARK_BG)), BorderColor::all(kit::color(outline))))
        .with_children(|p| if teaching { spawn_teaching(p, v) } else { spawn_answering(p, v) });
}

/// A wrapper that never gives up height to its neighbours.
fn fixed() -> Node {
    Node { flex_shrink: 0.0, ..col() }
}

/// The CRA visual's slot: the panel's full inner width, as tall as the
/// visual needs at that width (the same `visuals::plan` that draws it).
fn visual(p: &mut ChildSpawner, v: &ChallengeView) {
    let challenge = v.challenge.clone();
    p.spawn((Node { flex_shrink: 0.0, ..default() }, Art(ArtId::ChallengeVisual), HeightFor::new(move |w| visuals::extent(&challenge, w).h)));
}

fn spawn_teaching(p: &mut ChildSpawner, v: &ChallengeView) {
    let header = if v.told_me { "Here's how it works!" } else { "Let's figure it out!" };
    p.spawn(fixed()).with_child(fitted(header, 28.0, 18.0, false, ORANGE, Justify::Center));
    p.spawn(kit::fitted_squeezed(v.question.clone(), 34.0, 16.0, WHITE, Justify::Center));
    visual(p, v);
    p.spawn(fitted(format!("= {}", v.correct_answer), 54.0, 28.0, false, GREEN_ANS, Justify::Center));
    if let Some(fb) = &v.feedback {
        p.spawn(text_centered(fb.clone(), 24.0, GOLD));
    }
    p.spawn(fixed()).with_child(fitted("Press SPACE or click to continue", 22.0, 14.0, false, HINT_GRAY, Justify::Center));
}

fn spawn_answering(p: &mut ChildSpawner, v: &ChallengeView) {
    let answering = v.phase == Phase::Presented || v.phase == Phase::Feedback;
    let complete = v.phase == Phase::Complete;
    let solved = complete && v.correct == Some(true);

    p.spawn(kit::fitted_squeezed(v.question.clone(), 42.0, 16.0, WHITE, Justify::Center));
    if v.hint_used {
        visual(p, v);
    }
    // While the kid is answering, the feedback line is always there — empty
    // until they miss — so "Hmm, not quite!" appearing can't shove the answer
    // buttons out from under their finger.
    if answering {
        let msg = if v.phase == Phase::Feedback { v.feedback.clone().unwrap_or_default() } else { String::new() };
        p.spawn((Node { height: px(66), flex_shrink: 0.0, ..col() }, Children::spawn(Spawn(kit::filled(msg, 28.0, 14.0, ORANGE, Justify::Center)))));
    }
    // The answers: the row gives up height on a short screen (to 64) and the
    // buttons stretch to it.
    let choices = Node {
        column_gap: px(20),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Stretch,
        height: px(88),
        min_height: px(64),
        ..default()
    };
    p.spawn(choices).with_children(|p| {
        for (i, (label, correct)) in v.choices.iter().enumerate() {
            let fill = if solved { if *correct { GREEN_BTN } else { DIM_BTN } } else { BLUE_BTN };
            let node = Node {
                width: px(200),
                min_width: px(64),
                padding: Edges::axes(px(10), px(4)),
                border_radius: BorderRadius::all(px(12)),
                ..col()
            };
            p.spawn(button(UiAction::Challenge(ChallengeId::Choice(i)), node, fill, Color::new(1.0, 1.0, 1.0, 0.3)))
                .with_children(|p| {
                    // The answer, centred; the key hint (1, 2, 3) tucked in
                    // the top-left corner, out of its way.
                    p.spawn(Node { flex_grow: 1.0, ..kit::centered() })
                        .with_child(fitted(label.clone(), 40.0, 18.0, false, WHITE, Justify::Center));
                    p.spawn(Node { position_type: bevy::ui::PositionType::Absolute, left: px(8), top: px(2), ..default() })
                        .with_child(fitted(format!("{}", i + 1), 20.0, 12.0, false, Color::new(1.0, 1.0, 1.0, 0.4), Justify::Left));
                });
        }
    });
    if answering {
        let row = Node {
            column_gap: px(12),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            height: px(46),
            min_height: px(38),
            ..default()
        };
        p.spawn(row).with_children(|p| {
            // Show me — available until the visual is displayed.
            if !v.hint_used {
                scaffold(p, ChallengeId::ShowMe, "Show me", SCAFFOLD_BG, SCAFFOLD_TXT);
            }
            scaffold(p, ChallengeId::TellMe, "Tell me", SCAFFOLD_DIM, SCAFFOLD_TXT_DIM);
        });
    }
    if solved {
        let praise = PRAISES[(v.correct_answer.unsigned_abs() as usize) % PRAISES.len()];
        let row = Node { height: px(100), min_height: px(30), ..kit::centered() };
        p.spawn(row).with_children(|p| {
            // Stars circle the praise, drawn behind it.
            p.spawn((
                Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() },
                Art(ArtId::ChallengeStars),
            ));
            p.spawn(fitted(praise, 44.0, 24.0, false, PRAISE_COLOR, Justify::Center));
        });
    }
    if complete {
        p.spawn((fixed(), Blink(4.0)))
            .with_child(fitted("Press SPACE to continue", 22.0, 14.0, false, HINT_GRAY, Justify::Center));
    }
}

/// A labelled scaffold button ("Show me" / "Tell me"): preferred 150 wide,
/// squeezes (never clips) on a narrow phone.
fn scaffold(p: &mut ChildSpawner, id: ChallengeId, label: &str, fill: Color, label_color: Color) {
    let node = Node {
        width: px(150),
        min_width: px(96),
        flex_shrink: 1.0,
        border: Edges::ZERO,
        border_radius: BorderRadius::all(px(6)),
        ..kit::centered()
    };
    p.spawn(button(UiAction::Challenge(id), node, fill, fill))
        .with_child(fitted(label, 22.0, 14.0, false, label_color, Justify::Center));
}

// ─── Art ────────────────────────────────────────────────

/// Paint art slot `id` (`w`×`h`, local coordinates).
pub fn draw_art(id: ArtId, challenge: &Challenge, w: f32, h: f32, time: f32) {
    match id {
        ArtId::ChallengeVisual => visuals::draw(challenge, UiRect::new(0.0, 0.0, w, h)),
        ArtId::ChallengeStars => draw_star_burst(UiRect::new(0.0, 0.0, w, h), (w / 2.0, h / 2.0), time),
        _ => {}
    }
}

/// Stars circling the praise, kept inside the celebration box.
fn draw_star_burst(area: UiRect, (cx, cy): (f32, f32), time: f32) {
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
        draw_line(sx - size, sy, sx + size, sy, 2.0, color);
        draw_line(sx, sy - size, sx, sy + size, 2.0, color);
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

/// A click this frame: on `press` (a challenge button), or elsewhere.
pub fn handle_click(press: Option<UiAction>, cs: &ChallengeState, challenge: &Challenge) -> Option<ChallengeAction> {
    // Teaching/Complete: click anywhere to dismiss
    if cs.phase == Phase::Teaching {
        return Some(ChallengeAction::TeachingComplete);
    }
    if cs.phase == Phase::Complete {
        return None; // Signal caller to dismiss
    }
    let Some(UiAction::Challenge(id)) = press else { return None };
    match id {
        ChallengeId::ShowMe => Some(ChallengeAction::ShowMe),
        ChallengeId::TellMe => Some(ChallengeAction::TellMe),
        ChallengeId::Choice(i) => {
            let answer: i32 = challenge.choices.get(i)?.text.parse().unwrap_or(0);
            Some(ChallengeAction::AnswerSubmitted { answer })
        }
    }
}
