//! Quest runner UI. Pure layout + render. Opt-in (feature-flagged) while the
//! quest path is being playtested.
//!
//! Self-contained: a quest plays as a sequence of narrative beats and inline
//! multiple-choice math moments, so it never has to hand control to the
//! challenge state and come back. The game builds a `QuestView` from the domain
//! `QuestSession` (+ the generated puzzle choices) each frame; this module lays
//! it out (see `ui::layout`), draws it, and reports taps — both from the same
//! `Frame`.

use crate::prelude::*;

use crate::input::FrameInput;
use crate::ui::layout::{self, button, col, paint, row, spacer, text, Align, Fit, Frame, Justify, Kind, Node};
use robot_buddy_domain::economy::shop::Currency;
pub use crate::ui::layout::UiRect;

/// What the current quest step looks like to the player. Built by the game from
/// the domain step (+ generated puzzle choices for a MathPuzzle).
pub enum QuestView<'a> {
    Narrative { speaker: &'a str, lines: &'a [String] },
    Travel { label: String },
    Puzzle { prompt: &'a str, choices: &'a [i32] },
    /// A branching decision — the player taps one of the labelled options.
    Choice { prompt: &'a str, options: &'a [String] },
    Reward { dum_dums: u32 },
}

impl QuestView<'_> {
    /// Narrative / travel / reward beats (and a degenerate option-less Choice,
    /// so it can never soft-lock) advance with a Continue button.
    pub fn has_continue(&self) -> bool {
        match self {
            QuestView::Puzzle { .. } => false,
            QuestView::Choice { options, .. } => options.is_empty(),
            _ => true,
        }
    }

    fn body(&self) -> String {
        match self {
            QuestView::Narrative { speaker, lines } => format!("{speaker}: {}", lines.join(" ")),
            QuestView::Travel { label } => label.clone(),
            QuestView::Puzzle { prompt, .. } | QuestView::Choice { prompt, .. } => prompt.to_string(),
            QuestView::Reward { dum_dums } => format!("You earned {}!", Currency::DumDums.count(*dum_dums)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestId {
    Panel,
    Title,
    Body,
    Message,
    Answer(usize),
    AnswerLabel(usize),
    Option(usize),
    OptionLabel(usize),
    Continue,
    ContinueLabel,
}

pub struct QuestLayout {
    pub frame: Frame<QuestId>,
}

impl QuestLayout {
    pub fn continue_btn(&self) -> Option<UiRect> {
        self.frame.rect(QuestId::Continue)
    }
    pub fn option(&self, index: usize) -> Option<UiRect> {
        self.frame.rect(QuestId::Option(index))
    }
    pub fn answer(&self, index: usize) -> Option<UiRect> {
        self.frame.rect(QuestId::Answer(index))
    }
}

pub enum QuestClick {
    Continue,
    Answer(i32),
    Choose(usize),
}

const PANEL_W: f32 = 720.0;
const PANEL_H: f32 = 420.0;
const MARGIN: f32 = 20.0;

fn actions(view: &QuestView) -> Node<QuestId> {
    match view {
        QuestView::Puzzle { choices, .. } => row().gap(18.0).justify(Justify::Center).min_h(60.0).children(
            choices.iter().enumerate().map(|(i, v)| {
                button(QuestId::Answer(i), QuestId::AnswerLabel(i), v.to_string(), 34, Fit::shrink(18))
                    .size(84.0, 84.0)
                    .min_w(48.0)
            }),
        ),
        QuestView::Choice { options, .. } if !options.is_empty() => {
            // Stacked full-width buttons.
            col().gap(12.0).children(options.iter().enumerate().map(|(i, label)| {
                button(QuestId::Option(i), QuestId::OptionLabel(i), format!("{}. {}", i + 1, label), 22, Fit::shrink_then_wrap(14, 2))
                    .h(46.0)
                    .min_h(36.0)
            }))
        }
        _ => button(QuestId::Continue, QuestId::ContinueLabel, "Continue", 22, Fit::shrink(16))
            .size(180.0, 44.0)
            .fixed()
            .align_self(Align::Center),
    }
}

/// Lay out the current beat. The panel grows with a long body (up to the
/// screen), and the body shrinks its font before anything is clipped.
pub fn layout(view: &QuestView, title: &str, message: Option<&str>, screen: (f32, f32)) -> QuestLayout {
    let panel = col()
        .id(QuestId::Panel)
        .w(PANEL_W)
        .min_w(0.0)
        .min_h(PANEL_H.min(screen.1 - 2.0 * MARGIN))
        .pad_edges(28.0, 14.0, 28.0, 20.0)
        .gap(12.0)
        .child(text(title, 26, Fit::shrink(16)).id(QuestId::Title).center_text().fixed())
        .child(text(view.body(), 24, Fit::wrap(14)).id(QuestId::Body))
        .child(spacer())
        // Reserved even when empty, so the buttons don't jump on a retry.
        .child(text(message.unwrap_or(""), 22, Fit::wrap_lines(12, 2)).id(QuestId::Message).center_text().h(30.0).fixed())
        .child(actions(view));
    let bounds = layout::screen_rect(screen);
    QuestLayout { frame: layout::layout(&layout::centered_on_screen(panel, MARGIN), bounds) }
}

pub fn handle_click(mx: f32, my: f32, layout: &QuestLayout, view: &QuestView) -> Option<QuestClick> {
    match layout.frame.hit_at(mx, my)? {
        QuestId::Continue => Some(QuestClick::Continue),
        QuestId::Answer(i) => match view {
            QuestView::Puzzle { choices, .. } => choices.get(i).map(|&v| QuestClick::Answer(v)),
            _ => None,
        },
        QuestId::Option(i) => Some(QuestClick::Choose(i)),
        _ => None,
    }
}

pub fn handle_key(input: &FrameInput, view: &QuestView) -> Option<QuestClick> {
    if view.has_continue() && (input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter)) {
        return Some(QuestClick::Continue);
    }
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
    let pressed = keys.iter().position(|k| input.pressed(*k))?;
    match view {
        QuestView::Puzzle { choices, .. } => choices.get(pressed).map(|&v| QuestClick::Answer(v)),
        QuestView::Choice { options, .. } if pressed < options.len() => Some(QuestClick::Choose(pressed)),
        _ => None,
    }
}

// ─── Drawing ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const TILE_BG: Color = Color::new(0.129, 0.588, 0.953, 1.0);

pub fn draw(layout: &QuestLayout) {
    let f = &layout.frame;
    paint::dim(f.bounds, 0.5);
    for el in f.elements() {
        let Some(id) = el.id else { continue };
        match (&el.kind, id) {
            (Kind::Text(t), QuestId::Title | QuestId::Message) => paint::text(t, GOLD),
            (Kind::Text(t), _) => paint::text(t, WHITE),
            (_, QuestId::Panel) => paint::boxed(el.rect, DARK_BG, 4.0, GOLD),
            (_, QuestId::Answer(_) | QuestId::Option(_) | QuestId::Continue) => {
                paint::boxed(el.rect, TILE_BG, 2.0, Color::new(1.0, 1.0, 1.0, 0.4))
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A Choice step must render one button per option and report the TAPPED
    // index — not always 0 (the bug this fixes).
    #[test]
    fn choice_options_map_to_their_own_index() {
        let opts = vec!["Confront".to_string(), "Set a trap".to_string()];
        let view = QuestView::Choice { prompt: "How?", options: &opts };
        let layout = layout(&view, "A quest", None, (960.0, 720.0));
        assert!(layout.option(0).is_some() && layout.option(1).is_some(), "one button per option");
        assert!(layout.option(2).is_none());
        assert!(layout.continue_btn().is_none(), "a Choice has no Continue button");

        // Tapping the second option's centre yields Choose(1), not Choose(0).
        let (x, y) = layout.option(1).unwrap().center();
        assert!(matches!(handle_click(x, y, &layout, &view), Some(QuestClick::Choose(1))));
        let (x, y) = layout.option(0).unwrap().center();
        assert!(matches!(handle_click(x, y, &layout, &view), Some(QuestClick::Choose(0))));
    }
}
