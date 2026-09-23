//! Central text rendering backed by a bundled font.
//!
//! macroquad's built-in font is `ProggyClean`, a pixel font that lacks the
//! math operators we show (−, ×, ÷), the star (★), and every emoji. We bundle
//! a glyph set derived from **GNU Unifont** (BMP + upper planes, subset to the
//! ranges we render) so no glyph ever falls back to tofu — including emoji that
//! arrive in generated dialogue.
//!
//! Unifont is a 16px bitmap-style pixel font, so it sits naturally next to the
//! retro tile art. macroquad rasterizes glyphs to a monochrome alpha mask
//! (via fontdue), so emoji render as single-color silhouettes tinted by the
//! draw color — color emoji aren't possible in this engine.
//!
//! Every UI module draws through the `draw_text`/`measure_text` re-exports here
//! (which shadow macroquad's prelude versions) so they all share this font.
//! Until [`init`] runs they transparently fall back to macroquad's default
//! font, which keeps the headless tests (no GL context) working unchanged.

use macroquad::prelude::{load_ttf_font_from_bytes, Color, Font, TextDimensions, TextParams};
use macroquad::text as mq_text;
use std::cell::RefCell;

/// GNU Unifont, merged BMP + upper planes and subset to the ranges we draw.
/// Built from the system `unifont.otf` + `unifont_upper.otf` (see
/// `assets/README.md`). glyf outlines so fontdue can rasterize it directly.
pub const FONT_BYTES: &[u8] = include_bytes!("../assets/unifont-subset.ttf");

thread_local! {
    static FONT: RefCell<Option<Font>> = const { RefCell::new(None) };
}

/// Load the bundled font into the GL texture atlas, then check that what
/// macroquad renders measures the same as what UI layout measured headlessly
/// ([`crate::ui::layout::metrics::renderer_drift`]). Call once after the
/// macroquad context exists, since building the atlas needs the graphics
/// context.
///
/// `Err` means layout and rendering disagree — the font didn't load (so
/// macroquad draws its default font into boxes sized for Unifont), or a
/// fractional `high_dpi` scale makes macroquad round glyph sizes up — and
/// text will overflow the boxes laid out for it. The caller decides how loud
/// to be; [`init_or_die`] is the game's choice.
pub fn init() -> Result<(), String> {
    let font = load_ttf_font_from_bytes(FONT_BYTES).map_err(|err| format!("bundled font failed to load: {err}"))?;
    FONT.with(|cell| *cell.borrow_mut() = Some(font));
    match crate::ui::layout::metrics::renderer_drift() {
        None => Ok(()),
        Some(drift) => Err(format!("UI layout metrics disagree with the renderer: {drift}")),
    }
}

/// [`init`], failing loudly: a debug build panics (layout would be wrong and
/// every screenshot would lie); a release build logs an error and carries on
/// with slightly mis-sized text rather than showing a kid a dead screen.
pub fn init_or_die() {
    if let Err(e) = init() {
        if cfg!(debug_assertions) {
            panic!("{e}");
        }
        macroquad::logging::error!("{e}");
    }
}

fn with_font<R>(f: impl FnOnce(Option<&Font>) -> R) -> R {
    FONT.with(|cell| f(cell.borrow().as_ref()))
}

/// Drop-in replacement for `macroquad::text::draw_text` that renders with the
/// bundled font. Same signature, so call sites are unchanged.
pub fn draw_text(text: &str, x: f32, y: f32, font_size: f32, color: Color) -> TextDimensions {
    with_font(|font| {
        mq_text::draw_text_ex(
            text,
            x,
            y,
            TextParams {
                font,
                font_size: font_size as u16,
                color,
                ..Default::default()
            },
        )
    })
}

/// Drop-in replacement for `macroquad::text::measure_text`. The `_font`
/// argument mirrors macroquad's signature (call sites pass `None`); the bundled
/// font is always used so measurements match what `draw_text` renders.
pub fn measure_text(
    text: &str,
    _font: Option<&Font>,
    font_size: u16,
    font_scale: f32,
) -> TextDimensions {
    with_font(|font| mq_text::measure_text(text, font, font_size, font_scale))
}
