//! The outfit-colour swatch grid, shared by every panel that picks a Color
//! Change colour: Bolt's counter (for the kid) and the Give-Swag panel (for a
//! buddy). One grid, so the kid picks a colour the same way for anybody.

use bevy::prelude::{default, AlignItems, ChildSpawner, JustifyContent, Node, UiRect};
use bevy::ui::px;

use crate::prelude::*;
use super::kit::{button, col};
use super::UiAction;

/// Swatches per row.
const PER_ROW: usize = 4;

const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);

/// Tappable swatches in rows of four, `action(i)` naming swatch `i`; the
/// colour worn right now gets a thick gold frame. Rows shrink toward 56px on a
/// short screen and the swatches stretch to them.
pub fn grid(p: &mut ChildSpawner, colors: &[Color], current: usize, action: impl Fn(usize) -> UiAction) {
    p.spawn(Node { row_gap: px(18), min_height: px(0), ..col() }).with_children(|p| {
        for start in (0..colors.len()).step_by(PER_ROW) {
            let row = Node {
                column_gap: px(18),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Stretch,
                height: px(90),
                min_height: px(56),
                ..default()
            };
            p.spawn(row).with_children(|p| {
                for (i, &c) in colors.iter().enumerate().skip(start).take(PER_ROW) {
                    let worn = i == current;
                    let node = Node {
                        width: px(90),
                        min_width: px(40),
                        border: UiRect::all(px(if worn { 6.0 } else { 2.0 })),
                        ..default()
                    };
                    let edge = if worn { GOLD } else { Color::new(1.0, 1.0, 1.0, 0.4) };
                    p.spawn(button(action(i), node, c, edge));
                }
            });
        }
    });
}
