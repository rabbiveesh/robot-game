//! The bundled game font.
//!
//! We bundle a glyph set derived from **GNU Unifont** (BMP + upper planes,
//! subset to the ranges we render) so the math operators (−, ×, ÷), the star
//! (★) and emoji in generated dialogue never fall back to tofu. Unifont is a
//! 16px pixel font, so it sits naturally next to the retro tile art.
//!
//! [`crate::app`] installs it as Bevy's default font, so every `Text` and
//! `Text2d` uses it without naming a font. Layout measures the same bytes
//! headlessly through `ui::metrics::FontMetrics`.

/// GNU Unifont, merged BMP + upper planes and subset to the ranges we draw.
/// Built from the system `unifont.otf` + `unifont_upper.otf` (see
/// `assets/README.md`).
pub const FONT_BYTES: &[u8] = include_bytes!("../assets/unifont-subset.ttf");
