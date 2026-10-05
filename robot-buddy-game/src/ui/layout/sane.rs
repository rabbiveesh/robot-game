//! The one generic layout test: is this frame sane?
//!
//! Engine-agnostic — it only reads a resolved [`Frame`], so the same sweep
//! guards the in-house engine today and a taffy engine tomorrow.

use std::fmt::Debug;

use super::frame::{Frame, Kind};
use super::rect::UiRect;

/// Everything wrong with `frame` inside `bounds`, or `Ok`. Checks:
/// 1. every element lies inside `bounds` and inside its parent element;
/// 2. every text line lies inside its text box;
/// 3. no two elements overlap unless one is nested in the other (a label in
///    its button, a row in its list);
/// 4. nothing was clipped out (a clipped row is an unreachable button — the
///    panel's paging policy should have prevented it);
/// 5. no text overflowed its fit policy.
pub fn check_sane<Id: Copy + PartialEq + Debug>(frame: &Frame<Id>, bounds: UiRect) -> Result<(), Vec<String>> {
    let els = frame.elements();
    let mut issues = Vec::new();
    let name = |i: usize| match els[i].id {
        Some(id) => format!("{id:?}"),
        None => match &els[i].kind {
            Kind::Text(t) => format!("text {:?}", t.lines.first().map(|l| l.text.as_str()).unwrap_or("")),
            _ => format!("element #{i}"),
        },
    };

    for (i, e) in els.iter().enumerate() {
        if e.rect.w < 0.0 || e.rect.h < 0.0 {
            issues.push(format!("{} has negative size {:?}", name(i), e.rect));
        }
        if !bounds.contains_rect(&e.rect) {
            issues.push(format!("{} {:?} escapes bounds {:?}", name(i), e.rect, bounds));
        }
        if let Some(p) = e.parent {
            if !els[p].rect.contains_rect(&e.rect) {
                issues.push(format!("{} {:?} escapes its parent {} {:?}", name(i), e.rect, name(p), els[p].rect));
            }
        }
        if let Kind::Text(t) = &e.kind {
            for l in &t.lines {
                if !e.rect.contains_rect(&l.rect) {
                    issues.push(format!("{} line {:?} {:?} escapes its box {:?}", name(i), l.text, l.rect, e.rect));
                }
            }
            if t.overflowed {
                let shown: Vec<&str> = t.lines.iter().map(|l| l.text.as_str()).collect();
                issues.push(format!("{} overflowed its fit policy (shown as {shown:?} in {:?})", name(i), e.rect));
            }
        }
    }

    for a in 0..els.len() {
        for b in (a + 1)..els.len() {
            if frame.is_ancestor_or_self(a, b) || frame.is_ancestor_or_self(b, a) {
                continue;
            }
            // Empty text takes no ink; its (possibly reserved) box can't collide.
            let ink = |i: usize| !matches!(&els[i].kind, Kind::Text(t) if t.lines.is_empty());
            if ink(a) && ink(b) && els[a].rect.overlaps(&els[b].rect) {
                issues.push(format!(
                    "{} {:?} overlaps {} {:?}",
                    name(a), els[a].rect, name(b), els[b].rect
                ));
            }
        }
    }

    for c in frame.clipped() {
        issues.push(format!("{} was clipped out (didn't fit)", c.what()));
    }

    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

/// Panic with every issue [`check_sane`] finds.
#[track_caller]
pub fn assert_sane<Id: Copy + PartialEq + Debug>(frame: &Frame<Id>, bounds: UiRect) {
    if let Err(issues) = check_sane(frame, bounds) {
        panic!("layout is not sane:\n  - {}", issues.join("\n  - "));
    }
}
