//! The one screen-space rectangle every UI module shares.

/// Axis-aligned screen rectangle (top-left origin, y down).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Slack for float comparisons in containment / overlap checks.
pub const EPS: f32 = 0.5;

impl UiRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        UiRect { x, y, w, h }
    }

    /// Inclusive point test — the hit-test every panel uses.
    pub fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + self.h
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Shrink by per-side insets (never below zero size).
    pub fn inset(&self, left: f32, top: f32, right: f32, bottom: f32) -> UiRect {
        UiRect {
            x: self.x + left,
            y: self.y + top,
            w: (self.w - left - right).max(0.0),
            h: (self.h - top - bottom).max(0.0),
        }
    }

    /// Grow by `d` on every side (negative shrinks).
    pub fn expand(&self, d: f32) -> UiRect {
        UiRect { x: self.x - d, y: self.y - d, w: (self.w + 2.0 * d).max(0.0), h: (self.h + 2.0 * d).max(0.0) }
    }

    /// `other` lies inside `self` (within [`EPS`]).
    pub fn contains_rect(&self, other: &UiRect) -> bool {
        other.x >= self.x - EPS
            && other.y >= self.y - EPS
            && other.right() <= self.right() + EPS
            && other.bottom() <= self.bottom() + EPS
    }

    /// The two rects share more than a hairline of area.
    pub fn overlaps(&self, other: &UiRect) -> bool {
        let ix = self.right().min(other.right()) - self.x.max(other.x);
        let iy = self.bottom().min(other.bottom()) - self.y.max(other.y);
        ix > EPS && iy > EPS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_ignores_shared_edges() {
        let a = UiRect::new(0.0, 0.0, 10.0, 10.0);
        let b = UiRect::new(10.0, 0.0, 10.0, 10.0);
        assert!(!a.overlaps(&b));
        assert!(a.overlaps(&UiRect::new(5.0, 5.0, 10.0, 10.0)));
        assert!(a.contains_rect(&UiRect::new(1.0, 1.0, 9.0, 9.0)));
        assert!(!a.contains_rect(&UiRect::new(1.0, 1.0, 12.0, 9.0)));
    }
}
