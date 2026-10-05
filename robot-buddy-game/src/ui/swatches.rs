//! The outfit-colour swatch grid, shared by every panel that picks a Color
//! Change colour: Bolt's counter (for the kid) and the Give-Swag panel (for a
//! buddy). One grid, so the kid picks a colour the same way for anybody.
//!
//! Layout-only nodes plus a painter; the panel supplies its own ids.

use crate::prelude::*;
use crate::ui::layout::{col, paint, region, row, Align, Justify, Node, UiRect};

/// Swatches per row.
const PER_ROW: usize = 4;

/// `count` tappable swatches in rows of four, `id(i)` naming swatch `i`.
/// Rows shrink toward 56px on a short screen and the swatches stretch to them;
/// `min_h(0)` lets the grid squeeze its rows (CSS won't shrink a column below
/// its rows' preferred heights otherwise).
pub fn grid<Id: Copy>(count: usize, id: impl Fn(usize) -> Id) -> Node<Id> {
    col().gap(18.0).min_h(0.0).children((0..count).step_by(PER_ROW).map(|start| {
        let end = (start + PER_ROW).min(count);
        row().gap(18.0).justify(Justify::Center).align(Align::Stretch).h(90.0).min_h(56.0).children(
            (start..end).map(|i| region(90.0, 0.0).auto_h().id(id(i)).hit().min_w(40.0)),
        )
    }))
}

const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);

/// Paint one swatch; the colour being worn right now gets a thick gold frame.
pub fn paint_swatch(r: UiRect, color: Color, worn: bool) {
    paint::fill(r, color);
    if worn {
        paint::outline(r.expand(3.0), 6.0, GOLD);
    } else {
        paint::outline(r, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
    }
}
