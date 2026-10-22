//! "Give Swag" picker, as a Bevy UI panel; who-wears-what lives in
//! `robot_buddy_domain::economy::wardrobe`.
//!
//! Two views. The list: the pieces the kid is wearing, as big tappable rows,
//! with the buddy they'd go to named at the top. Tap one and it's theirs — no
//! math gate here, the arithmetic already happened at Bolt's counter. And the
//! colour picker: hand over Color Change (or ask a buddy wearing it for a "New
//! color?") and the rows give way to swatches — "What color for Tali?".

use std::collections::BTreeSet;

use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::{default, Bundle, ChildSpawner, Children, JustifyContent, Node, SpawnRelated, UiRect};
use bevy::text::Justify;
use bevy::ui::{percent, px, BackgroundColor, BorderColor};

use crate::prelude::*;
use crate::npc::SpriteType;
use robot_buddy_domain::economy::shop::ShopItem;

use super::kit::{self, backdrop, button, col, fitted, Art, Panel};
use super::views::UiViews;
use super::{swatches, ArtId, UiAction};
use crate::input::FrameInput;

/// Picking the recipient's Color Change colour: the palette, and the swatch
/// they're wearing now.
pub struct ColorPick<'a> {
    pub colors: &'a [(&'static str, Color)],
    pub current: usize,
}

/// Everything the picker shows, borrowed from the session.
pub struct SwagModel<'a> {
    pub recipient: &'a str,
    /// The pieces the kid can hand over (what they're wearing).
    pub items: &'a [ShopItem],
    /// Ids the recipient already wears: shown, but reading as unavailable.
    pub taken: &'a BTreeSet<String>,
    pub message: Option<&'a str>,
    /// `Some` while picking the recipient's outfit colour instead of listing.
    pub picking: Option<ColorPick<'a>>,
}

/// The picker's buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwagId {
    Item(usize),
    Swatch(usize),
    Done,
}

/// The buddy as they look now (no sprite: Sparky, drawn as the robot).
#[derive(Clone, PartialEq, Debug)]
pub struct BuddyLook {
    pub sprite: Option<SpriteType>,
    pub worn: BTreeSet<String>,
    pub color: String,
}

/// One row of the wardrobe list.
#[derive(Clone, PartialEq, Debug)]
pub struct SwagRow {
    pub label: String,
    /// "Tali has one" when the recipient already wears it.
    pub note: Option<String>,
}

/// Everything the picker shows, owned.
#[derive(Clone, PartialEq, Debug)]
pub struct SwagPanelView {
    pub title: String,
    pub rows: Vec<SwagRow>,
    pub swatches: Option<(Vec<Color>, usize)>,
    pub message: Option<String>,
    pub look: BuddyLook,
}

impl SwagPanelView {
    pub fn new(m: &SwagModel, look: BuddyLook) -> Self {
        SwagPanelView {
            title: title(m),
            rows: m.items.iter().enumerate().map(|(i, item)| SwagRow {
                label: format!("[{}] {}", i + 1, item.name),
                note: m.taken.contains(&item.id).then(|| format!("{} has one", m.recipient)),
            }).collect(),
            swatches: m.picking.as_ref().map(|p| (p.colors.iter().map(|c| c.1).collect(), p.current)),
            message: m.message.map(str::to_string),
            look,
        }
    }
}

const PANEL_W: f32 = 560.0;
const PREVIEW: (f32, f32) = (60.0, 64.0);

/// The panel's heading: whose outfit it is, and what's being chosen.
fn title(m: &SwagModel) -> String {
    match m.picking {
        Some(_) => format!("What color for {}?", m.recipient),
        None => format!("Dress up {}!", m.recipient),
    }
}

pub struct SwagPanel;

impl Panel for SwagPanel {
    type View = SwagPanelView;
    const Z: i32 = 40;
    fn view(views: &UiViews) -> Option<&SwagPanelView> {
        views.swag.as_ref()
    }
    fn bundle(v: &SwagPanelView) -> impl Bundle {
        let v = v.clone();
        (backdrop(0.5, 20.0), Children::spawn(SpawnWith(move |p: &mut ChildSpawner| spawn_panel(p, &v))))
    }
}

/// As tall as the wardrobe needs, up to the screen; past that the list scrolls.
fn spawn_panel(p: &mut ChildSpawner, v: &SwagPanelView) {
    let panel = Node {
        width: percent(100),
        max_width: px(PANEL_W),
        max_height: percent(100),
        min_height: px(0),
        padding: UiRect::new(px(32), px(16), px(8), px(14)),
        row_gap: px(12),
        border: UiRect::all(px(4)),
        ..col()
    };
    p.spawn((panel, BackgroundColor(kit::color(DARK_BG)), BorderColor::all(kit::color(GOLD)))).with_children(|p| {
        p.spawn(Node { flex_shrink: 0.0, ..default() }).with_children(|p| {
            // Balances the preview so the title stays centred — the first
            // thing to give up width on a narrow phone.
            p.spawn(Node { width: px(PREVIEW.0), min_width: px(0), flex_shrink: 1.0, ..default() });
            p.spawn(Node { flex_grow: 1.0, min_width: px(0), ..col() })
                .with_child(fitted(v.title.clone(), 30.0, 16.0, true, GOLD, Justify::Center));
            p.spawn((Node { width: px(PREVIEW.0), height: px(PREVIEW.1), flex_shrink: 0.0, ..default() }, Art(ArtId::SwagPreview)));
        });
        match &v.swatches {
            Some((colors, current)) => swatches::grid(p, colors, *current, |i| UiAction::Swag(SwagId::Swatch(i))),
            None => {
                p.spawn(Node { flex_shrink: 0.0, ..col() })
                    .with_child(kit::text("Tap something you're wearing to give it away.", 22.0, WHITE));
                if v.rows.is_empty() {
                    p.spawn(Node { flex_shrink: 0.0, ..col() })
                        .with_child(kit::text("You're not wearing any swag right now!", 24.0, GOLD));
                }
                p.spawn(kit::scroll_list("swag", 10.0)).with_children(|p| {
                    for (i, row) in v.rows.iter().enumerate() {
                        let node = Node {
                            height: px(54),
                            min_height: px(40),
                            flex_shrink: 0.0,
                            padding: UiRect::axes(px(16), px(4)),
                            column_gap: px(12),
                            align_items: bevy::prelude::AlignItems::Center,
                            ..default()
                        };
                        let ink = if row.note.is_some() { FADED } else { WHITE };
                        p.spawn(button(UiAction::Swag(SwagId::Item(i)), node, ROW_BG, Color::new(1.0, 1.0, 1.0, 0.25)))
                            .with_children(|p| {
                                p.spawn(Node { flex_grow: 1.0, min_width: px(0), ..col() })
                                    .with_child(fitted(row.label.clone(), 24.0, 14.0, true, ink, Justify::Left));
                                if let Some(note) = &row.note {
                                    p.spawn(fitted(note.clone(), 20.0, 12.0, false, FADED, Justify::Left));
                                }
                            });
                    }
                });
            }
        }
        p.spawn(Node { flex_shrink: 0.0, row_gap: px(4), ..col() }).with_children(|p| {
            p.spawn(Node { height: px(40), flex_shrink: 0.0, ..col() })
                .with_child(kit::filled(v.message.clone().unwrap_or_default(), 24.0, 12.0, GOLD, Justify::Center));
            p.spawn(Node { justify_content: JustifyContent::End, ..default() }).with_children(|p| {
                let node = Node { width: px(100), height: px(38), min_width: px(72), flex_shrink: 1.0, ..kit::centered() };
                p.spawn(button(UiAction::Swag(SwagId::Done), node, Color::new(0.329, 0.431, 0.478, 1.0), Color::new(1.0, 1.0, 1.0, 0.3)))
                    .with_child(fitted("Done", 22.0, 14.0, false, WHITE, Justify::Center));
            });
        });
    });
}

/// Paint the buddy in their current outfit into the preview slot.
pub fn draw_art(v: &SwagPanelView) {
    use crate::sprites::{self, Dir};
    let (px, py) = (6.0, 10.0);
    let time = get_time() as f32;
    let look = &v.look;
    match look.sprite {
        Some(sprite) => {
            sprite.draw_sprite(px, py, Dir::Down, time, false);
            sprites::swag::draw_swag(px, py, Dir::Down, 0.0, &look.worn, &look.color, sprite.swag_fit());
        }
        None => {
            sprites::robot::draw_robot(px, py, Dir::Down, 0, time);
            sprites::swag::draw_swag(px, py, Dir::Down, 0.0, &look.worn, &look.color, sprites::swag::SwagFit::ROBOT);
        }
    }
}

pub enum SwagInput {
    Give(usize),
    /// Swatch `i` of the palette, while picking a colour.
    PickColor(usize),
    Close,
}

pub fn handle_click(input: &FrameInput) -> Option<SwagInput> {
    let Some(UiAction::Swag(id)) = input.ui_press else { return None };
    Some(match id {
        SwagId::Done => SwagInput::Close,
        SwagId::Item(i) => SwagInput::Give(i),
        SwagId::Swatch(i) => SwagInput::PickColor(i),
    })
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

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const ROW_BG: Color = Color::new(0.16, 0.18, 0.28, 1.0);
const FADED: Color = Color::new(1.0, 1.0, 1.0, 0.45);
