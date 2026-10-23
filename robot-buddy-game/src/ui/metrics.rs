//! Text measurement, the one thing a layout engine needs from the font.
//!
//! [`FontMetrics`] is used everywhere (step, render, tests). It parses the
//! bundled font (`assets/unifont-subset.ttf`, the one Bevy draws with) with
//! fontdue and sums advances, so no window is needed: `Game::step` and the
//! headless tests lay out with the same numbers `Game::render` paints with.

use std::sync::OnceLock;

pub trait TextMetrics {
    /// Advance width of `text` drawn at `size` px.
    fn width(&self, text: &str, size: u16) -> f32;
    /// Distance from the top of a line box to the baseline.
    fn ascent(&self, size: u16) -> f32;
    /// Distance from the baseline to the bottom of a line box (positive).
    fn descent(&self, size: u16) -> f32;
    /// Height of one line box.
    fn line_height(&self, size: u16) -> f32 {
        self.ascent(size) + self.descent(size)
    }
}

/// fontdue over the bundled game font. Headless and deterministic.
pub struct FontMetrics {
    font: fontdue::Font,
    /// Ascent / descent per px of font size (Unifont: 14/16 and 2/16).
    ascent_per_px: f32,
    descent_per_px: f32,
}

impl FontMetrics {
    /// macroquad-style [`crate::draw::TextDimensions`]: advance width, plus
    /// the union of the glyph boxes (height, and its top's distance above the
    /// baseline). Panels center labels on these.
    pub fn dimensions(&self, text: &str, size: u16) -> crate::draw::TextDimensions {
        let px = size as f32;
        let (mut width, mut top, mut bottom) = (0.0f32, f32::MIN, f32::MAX);
        for c in text.chars() {
            let m = self.font.metrics(c, px);
            width += m.advance_width;
            top = top.max(m.ymin as f32 + m.height as f32);
            bottom = bottom.min(m.ymin as f32);
        }
        if text.is_empty() {
            return crate::draw::TextDimensions::default();
        }
        crate::draw::TextDimensions { width, height: top - bottom, offset_y: top }
    }

    fn load() -> Self {
        let font = fontdue::Font::from_bytes(crate::text::FONT_BYTES, fontdue::FontSettings::default())
            .expect("bundled font parses");
        let lm = font.horizontal_line_metrics(100.0).expect("font has horizontal metrics");
        FontMetrics { font, ascent_per_px: lm.ascent / 100.0, descent_per_px: -lm.descent / 100.0 }
    }

    /// The shared instance (parsed once, on first use).
    pub fn bundled() -> &'static FontMetrics {
        static INSTANCE: OnceLock<FontMetrics> = OnceLock::new();
        INSTANCE.get_or_init(FontMetrics::load)
    }
}

impl TextMetrics for FontMetrics {
    fn width(&self, text: &str, size: u16) -> f32 {
        // Advance per char at the integer pixel size; Unifont has no kerning.
        let px = size as f32;
        text.chars().map(|c| self.font.metrics(c, px).advance_width).sum()
    }
    fn ascent(&self, size: u16) -> f32 {
        (self.ascent_per_px * size as f32).round()
    }
    fn descent(&self, size: u16) -> f32 {
        (self.descent_per_px * size as f32).round()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_font_measures_headlessly() {
        let m = FontMetrics::bundled();
        // Unifont is a 16px cell font: ASCII is half-width (8px at 16px).
        assert_eq!(m.width("Done", 16), 32.0);
        assert_eq!(m.width("Done", 24), 48.0);
        assert!(m.width("", 24) == 0.0);
        let lh = m.line_height(16);
        assert!((15.0..=18.0).contains(&lh), "line height {lh}");
        assert!(m.ascent(24) > m.descent(24));
    }
}
