//! Text measurement, the one thing a layout engine needs from the font.
//!
//! Two implementations of [`TextMetrics`]:
//!
//! * [`FontMetrics`] — the **default everywhere** (step, render, tests). It
//!   parses the exact bytes `crate::text` hands macroquad
//!   (`assets/unifont-subset.ttf`) with fontdue — the rasterizer macroquad
//!   itself uses — and sums advances the way `macroquad::text::measure_text`
//!   does. So widths are identical to what gets drawn, and no GL context is
//!   needed: `Game::step` (and the headless tests) lay out with the same
//!   numbers `Game::render` paints with. That is what keeps hit rects from
//!   drifting away from drawn rects.
//! * [`MacroquadMetrics`] — asks macroquad directly. Needs a live GL context,
//!   so it can only run inside `render`; debug builds use it to cross-check
//!   `FontMetrics` against the real renderer (see `paint::debug_check_width`).

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

/// fontdue over the bundled game font. Headless, deterministic, and
/// byte-for-byte the font macroquad draws with.
pub struct FontMetrics {
    font: fontdue::Font,
    /// Ascent / descent per px of font size (Unifont: 14/16 and 2/16).
    ascent_per_px: f32,
    descent_per_px: f32,
}

impl FontMetrics {
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
        // Same sum macroquad's Font::measure_text does: advance per char at
        // the integer pixel size, no kerning.
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

/// Measures through macroquad (`crate::text::measure_text`). Render-only.
pub struct MacroquadMetrics;

impl TextMetrics for MacroquadMetrics {
    fn width(&self, text: &str, size: u16) -> f32 {
        crate::text::measure_text(text, None, size, 1.0).width
    }
    fn ascent(&self, size: u16) -> f32 {
        FontMetrics::bundled().ascent(size)
    }
    fn descent(&self, size: u16) -> f32 {
        FontMetrics::bundled().descent(size)
    }
}

/// Strings and sizes [`renderer_drift`] probes: every glyph class we draw
/// (ASCII, digits, math operators, the star) across the sizes panels use.
const PROBE: &str = "Sparky has 12 Dum Dums! 7 + 5 = ? 3\u{00d7}4 8\u{00f7}2 9\u{2212}1 \u{2605}";
const PROBE_SIZES: [u16; 8] = [11, 14, 16, 18, 22, 28, 42, 54];

/// Compare [`FontMetrics`] (what layout measured, headless) with
/// [`MacroquadMetrics`] (what macroquad will render) on a probe string.
/// `None` if they agree to within half a pixel at every probed size; else a
/// description of the worst disagreement. Needs a live GL context.
///
/// They disagree when the bundled font didn't load (macroquad measures its
/// default font) or when a fractional DPI scale makes macroquad rasterize at
/// `ceil(size × dpi)` and divide back — advances then no longer match the
/// integer-size advances layout summed.
pub fn renderer_drift() -> Option<String> {
    let ours = FontMetrics::bundled();
    PROBE_SIZES
        .iter()
        .map(|&size| (size, ours.width(PROBE, size), MacroquadMetrics.width(PROBE, size)))
        .filter(|(_, a, b)| (a - b).abs() > 0.5)
        .max_by(|x, y| (x.1 - x.2).abs().total_cmp(&(y.1 - y.2).abs()))
        .map(|(size, a, b)| {
            format!(
                "a {}-char probe at {size}px is {a}px wide to layout but {b}px rendered (dpi scale {})",
                PROBE.chars().count(),
                macroquad::miniquad::window::dpi_scale()
            )
        })
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
