//! "Give Swag" picker. Pure layout + render; who-wears-what lives in
//! `robot_buddy_domain::economy::wardrobe`.
//!
//! Two views. The list: the pieces the kid is wearing, as big tappable rows,
//! with the buddy they'd go to named at the top. Tap one and it's theirs — no
//! math gate here, the arithmetic already happened at Bolt's counter. And the
//! colour picker: hand over Color Change (or ask a buddy wearing it for a "New
//! colour?") and the rows give way to swatches — "What colour for Tali?".
//!
//! Layout is declarative (see `ui::layout`): the `Frame` in [`SwagLayout`] is
//! what both [`draw`] and [`handle_click`] read.

use std::collections::BTreeSet;

use crate::prelude::*;
use robot_buddy_domain::economy::shop::ShopItem;

use crate::input::FrameInput;
use crate::ui::layout::{self, button, col, paint, region, row, spacer, text, Fit, Frame, Kind, Node, Page};
pub use crate::ui::layout::UiRect;
use crate::ui::swatches;

/// Picking the recipient's Color Change colour: the palette, and the swatch
/// they're wearing now.
pub struct ColorPick<'a> {
    pub colors: &'a [(&'static str, Color)],
    pub current: usize,
}

/// Everything the picker shows.
pub struct SwagModel<'a> {
    pub recipient: &'a str,
    /// The pieces the kid can hand over (what they're wearing).
    pub items: &'a [ShopItem],
    /// Ids the recipient already wears: shown, but reading as unavailable.
    pub taken: &'a BTreeSet<String>,
    pub message: Option<&'a str>,
    pub page: usize,
    /// `Some` while picking the recipient's outfit colour instead of listing.
    pub picking: Option<ColorPick<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwagId {
    Panel,
    Title,
    Subtitle,
    /// Where the game draws the recipient in their current outfit.
    Preview,
    Item(usize),
    ItemLabel(usize),
    ItemNote(usize),
    Empty,
    Swatch(usize),
    Message,
    PrevPage,
    PrevLabel,
    NextPage,
    NextLabel,
    Done,
    DoneLabel,
}

pub struct SwagLayout {
    pub frame: Frame<SwagId>,
    pub page: Page,
}

impl SwagLayout {
    pub fn item(&self, index: usize) -> Option<UiRect> {
        self.frame.rect(SwagId::Item(index))
    }
    /// `None` if a tiny window clipped it (Esc still closes).
    pub fn done(&self) -> Option<UiRect> {
        self.frame.rect(SwagId::Done)
    }
    pub fn swatch(&self, index: usize) -> Option<UiRect> {
        self.frame.rect(SwagId::Swatch(index))
    }
    /// Top-left to draw the recipient sprite at.
    /// `None` if a tiny window clipped it — skip the sprite, don't panic.
    pub fn preview(&self) -> Option<(f32, f32)> {
        let r = self.frame.rect(SwagId::Preview)?;
        Some((r.x + 6.0, r.y + 10.0))
    }
}

const PANEL_W: f32 = 560.0;
const PREVIEW: (f32, f32) = (60.0, 64.0);

fn build(m: &SwagModel, page: Page) -> Node<SwagId> {
    let header = row()
        .fixed()
        // Balances the preview so the title stays centered — the first thing
        // to give up width on a narrow phone.
        .child(region(PREVIEW.0, 0.0).min_w(0.0).shrink(1.0))
        .child(text(title(m), 30, Fit::shrink_then_wrap(16, 2)).id(SwagId::Title).center_text().grow(1.0))
        .child(region(PREVIEW.0, PREVIEW.1).id(SwagId::Preview).fixed());

    let rows = col().gap(10.0).grow(1.0).min_h(0.0).children(page.rows().map(|i| {
        let item = &m.items[i];
        row()
            .id(SwagId::Item(i))
            .hit()
            .h(54.0)
            .min_h(40.0)
            .pad_xy(16.0, 4.0)
            .gap(12.0)
            .child(text(format!("[{}] {}", i + 1, item.name), 24, Fit::shrink_then_wrap(14, 2)).id(SwagId::ItemLabel(i)).grow(1.0))
            .maybe(m.taken.contains(&item.id).then(|| {
                text(format!("{} has one", m.recipient), 20, Fit::shrink(12)).id(SwagId::ItemNote(i))
            }))
    }));

    let small_btn = |id, label_id, label: &str| button(id, label_id, label, 22, Fit::shrink(14))
        // Preferred 100 wide, but a narrow phone squeezes Back / More / Done
        // side by side rather than clipping one off.
        .size(100.0, 38.0).min_w(72.0).shrink(1.0);
    let footer = col()
        .fixed()
        .gap(4.0)
        .child(text(m.message.unwrap_or(""), 24, Fit::wrap_lines(14, 2)).id(SwagId::Message).center_text().h(40.0).fixed())
        .child(
            row()
                .gap(12.0)
                .maybe(page.has_prev().then(|| small_btn(SwagId::PrevPage, SwagId::PrevLabel, "< Back")))
                .maybe(page.has_next().then(|| small_btn(SwagId::NextPage, SwagId::NextLabel, "More >")))
                .child(spacer())
                .child(small_btn(SwagId::Done, SwagId::DoneLabel, "Done")),
        );

    if let Some(pick) = &m.picking {
        return layout::centered_on_screen(
            col()
                .id(SwagId::Panel)
                .w_pct(1.0)
                .max_w(PANEL_W)
                .min_h(0.0)
                .pad_edges(32.0, 8.0, 16.0, 14.0)
                .gap(12.0)
                .child(header)
                .child(swatches::grid(pick.colors.len(), SwagId::Swatch))
                .child(footer),
            20.0,
        );
    }

    layout::centered_on_screen(
        col()
            .id(SwagId::Panel)
            // As tall as the wardrobe needs, up to the screen; paging only
            // kicks in once even the whole screen can't hold it.
            .w_pct(1.0)
            .max_w(PANEL_W)
            .min_h(0.0)
            .pad_edges(32.0, 8.0, 16.0, 14.0)
            .gap(12.0)
            .child(header)
            .child(text("Tap something you're wearing to give it away.", 22, Fit::wrap_lines(14, 2)).id(SwagId::Subtitle).fixed())
            .maybe(m.items.is_empty().then(|| {
                text("You're not wearing any swag right now!", 24, Fit::wrap_lines(14, 2)).id(SwagId::Empty).fixed()
            }))
            .child(rows)
            .child(footer),
        20.0,
    )
}

/// The panel's heading: whose outfit it is, and what's being chosen.
fn title(m: &SwagModel) -> String {
    match m.picking {
        Some(_) => format!("What colour for {}?", m.recipient),
        None => format!("Dress up {}!", m.recipient),
    }
}

pub fn layout(m: &SwagModel, screen: (f32, f32)) -> SwagLayout {
    let bounds = layout::screen_rect(screen);
    if m.picking.is_some() {
        let page = Page::all(0);
        return SwagLayout { frame: layout::layout(&build(m, page), bounds), page };
    }
    let (frame, page) = layout::paged(m.items.len(), m.page, bounds, |p| build(m, p));
    SwagLayout { frame, page }
}

pub enum SwagInput {
    Give(usize),
    /// Swatch `i` of the palette, while picking a colour.
    PickColor(usize),
    Page(usize),
    Close,
}

pub fn handle_click(mx: f32, my: f32, layout: &SwagLayout) -> Option<SwagInput> {
    match layout.frame.hit_at(mx, my)? {
        SwagId::Done => Some(SwagInput::Close),
        SwagId::Item(i) => Some(SwagInput::Give(i)),
        SwagId::Swatch(i) => Some(SwagInput::PickColor(i)),
        SwagId::PrevPage => Some(SwagInput::Page(layout.page.index.saturating_sub(1))),
        SwagId::NextPage => Some(SwagInput::Page(layout.page.index + 1)),
        _ => None,
    }
}

pub fn handle_key(input: &FrameInput, m: &SwagModel) -> Option<SwagInput> {
    if input.pressed(KeyCode::Escape) {
        return Some(SwagInput::Close);
    }
    let keys = [
        KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4,
        KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8,
    ];
    // Number keys pick a swatch while choosing a colour, a row otherwise.
    let (count, pick): (usize, fn(usize) -> SwagInput) = match &m.picking {
        Some(p) => (p.colors.len(), SwagInput::PickColor),
        None => (m.items.len(), SwagInput::Give),
    };
    keys.iter().take(count).enumerate()
        .find(|(_, key)| input.pressed(**key))
        .map(|(i, _)| pick(i))
}

// ─── Drawing ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const ROW_BG: Color = Color::new(0.16, 0.18, 0.28, 1.0);
const FADED: Color = Color::new(1.0, 1.0, 1.0, 0.45);

/// Draw the picker from its frame (the game draws the recipient sprite into
/// `layout.preview()` afterwards).
pub fn draw(m: &SwagModel, layout: &SwagLayout) {
    let f = &layout.frame;
    paint::dim(f.bounds, 0.5);
    for el in f.elements() {
        let Some(id) = el.id else { continue };
        if let Kind::Text(t) = &el.kind {
            let color = match id {
                SwagId::Title | SwagId::Empty | SwagId::Message => GOLD,
                SwagId::ItemNote(_) => FADED,
                SwagId::ItemLabel(i) if m.taken.contains(&m.items[i].id) => FADED,
                _ => WHITE,
            };
            paint::text(t, color);
            continue;
        }
        match id {
            SwagId::Panel => paint::boxed(el.rect, DARK_BG, 4.0, GOLD),
            SwagId::Item(_) => paint::boxed(el.rect, ROW_BG, 2.0, Color::new(1.0, 1.0, 1.0, 0.25)),
            SwagId::Swatch(i) => {
                if let Some(p) = &m.picking {
                    swatches::paint_swatch(el.rect, p.colors[i].1, i == p.current);
                }
            }
            SwagId::Done | SwagId::PrevPage | SwagId::NextPage => {
                paint::boxed(el.rect, Color::new(0.329, 0.431, 0.478, 1.0), 2.0, Color::new(1.0, 1.0, 1.0, 0.3))
            }
            _ => {}
        }
    }
}
