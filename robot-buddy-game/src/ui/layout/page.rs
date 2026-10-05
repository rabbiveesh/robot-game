//! Paging policy for lists, kept ABOVE the engine.
//!
//! Kids can't scroll, and flexbox (ours or taffy's) only knows how to clip. So
//! when a list's rows won't all fit even after shrinking, we lay the panel out
//! again with fewer rows plus a "more" control, until nothing is clipped. The
//! policy only reads `Frame::clipped`, so it works unchanged on any engine.

use std::fmt::Debug;

use super::frame::Frame;
use super::node::Node;
use super::rect::UiRect;

/// Which slice of a list is on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// First row shown.
    pub start: usize,
    /// Rows shown.
    pub len: usize,
    /// Rows in the whole list.
    pub total: usize,
    /// Zero-based page number (after clamping).
    pub index: usize,
    /// Rows per full page.
    pub per: usize,
}

impl Page {
    pub fn all(total: usize) -> Self {
        Page { start: 0, len: total, total, index: 0, per: total }
    }
    /// More than one page exists — the panel should show page controls.
    pub fn paged(&self) -> bool {
        self.len < self.total
    }
    pub fn has_prev(&self) -> bool {
        self.start > 0
    }
    pub fn has_next(&self) -> bool {
        self.start + self.len < self.total
    }
    pub fn rows(&self) -> std::ops::Range<usize> {
        self.start..self.start + self.len
    }
    /// Number of pages at this page size.
    pub fn count(&self) -> usize {
        if self.per == 0 { 1 } else { self.total.div_ceil(self.per) }
    }
}

/// Lay out `build(page)` showing page number `want` (clamped to the last
/// page), with the most rows per page at which *every* page lays out without
/// clipping — so page boundaries don't move as the kid pages through.
/// `build` must add page controls when `page.paged()`.
pub fn paged<Id: Copy + PartialEq + Debug>(
    total: usize,
    want: usize,
    bounds: UiRect,
    build: impl Fn(Page) -> Node<Id>,
) -> (Frame<Id>, Page) {
    if total == 0 {
        let page = Page::all(0);
        return (super::layout(&build(page), bounds), page);
    }
    let window = |per: usize, index: usize| {
        let start = index * per;
        Page { start, len: per.min(total - start), total, index, per }
    };
    let fits = |page: Page| {
        let frame = super::layout(&build(page), bounds);
        frame.clipped().is_empty().then_some(frame)
    };
    for per in (1..=total).rev() {
        let pages = total.div_ceil(per);
        let index = want.min(pages - 1);
        let Some(frame) = fits(window(per, index)) else { continue };
        if (0..pages).filter(|&i| i != index).all(|i| fits(window(per, i)).is_some()) {
            return (frame, window(per, index));
        }
    }
    // Even one row per page clips: show it anyway and let the sanity sweep
    // report what was lost.
    let page = window(1, want.min(total - 1));
    (super::layout(&build(page), bounds), page)
}
