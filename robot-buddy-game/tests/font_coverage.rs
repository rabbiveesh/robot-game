//! Guards that the bundled font can actually draw what we ask it to.
//!
//! macroquad renders missing glyphs as tofu, and the bundled font is a *subset*
//! of GNU Unifont — so a new emoji or symbol added to the code (or a name the
//! player types) could silently render as a blank box. These tests fail loudly
//! instead:
//!
//!   1. Every non-ASCII character in any string/char literal across the domain
//!      and game crates (resolved, so `\u{2605}` counts as ★) has a glyph.
//!   2. Every character the name-input filter accepts has a glyph, so free-text
//!      names can never tofu.
//!
//! ASCII (U+0000–007F) is assumed covered and skipped: the font carries all
//! printable ASCII, and control chars (`\n`, `\t`) are layout, not glyphs.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use robot_buddy_game::ui::title_screen::name_char_allowed;

/// A char we expect to render. Below 0x80 is always fine (see module docs).
fn needs_glyph(ch: char) -> bool {
    ch as u32 >= 0x80
}

/// cmap lookup against the *bundled* font bytes — the same bytes the game embeds.
fn font_has_glyph(face: &ttf_parser::Face, ch: char) -> bool {
    face.glyph_index(ch).is_some()
}

/// Collect every character appearing in a string or char literal in one .rs
/// file, with escapes resolved (`"\u{2605}"` yields ★). Comments are ignored —
/// syn never surfaces them — so decorative glyphs in comments don't count.
fn literal_chars(src: &str) -> BTreeSet<char> {
    use syn::visit::Visit;

    #[derive(Default)]
    struct Collector(BTreeSet<char>);
    impl<'ast> Visit<'ast> for Collector {
        fn visit_lit_str(&mut self, lit: &'ast syn::LitStr) {
            self.0.extend(lit.value().chars());
        }
        fn visit_lit_char(&mut self, lit: &'ast syn::LitChar) {
            self.0.insert(lit.value());
        }
    }

    let file = syn::parse_file(src).expect("source file should parse as Rust");
    let mut c = Collector::default();
    c.visit_file(&file);
    c.0
}

/// Every `.rs` under `dir`, skipping `bin/` (CLI tools like the simulator print
/// to a terminal, not through the game font).
fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).expect("readable source dir") {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "bin") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn every_rendered_literal_has_a_glyph() {
    let face = ttf_parser::Face::parse(robot_buddy_game::text::FONT_BYTES, 0)
        .expect("bundled font should parse");

    // CARGO_MANIFEST_DIR is the game crate; the domain crate sits beside it and
    // produces display strings (e.g. the × ÷ − operators) the game renders.
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let domain = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../robot-buddy-domain/src");

    let mut missing: Vec<(char, PathBuf)> = Vec::new();
    for dir in [&game, &domain] {
        for file in rust_sources(dir) {
            let src = std::fs::read_to_string(&file).expect("readable source file");
            for ch in literal_chars(&src) {
                if needs_glyph(ch) && !font_has_glyph(&face, ch) {
                    missing.push((ch, file.clone()));
                }
            }
        }
    }

    assert!(
        missing.is_empty(),
        "bundled font is missing glyphs for characters used in source literals.\n\
         Add the relevant Unicode range to robot-buddy-game/assets/build-font.py \
         and regenerate, or remove the character:\n{}",
        missing
            .iter()
            .map(|(ch, f)| format!("  {:?} (U+{:04X}) in {}", ch, *ch as u32, f.display()))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

#[test]
fn every_accepted_name_char_has_a_glyph() {
    let face = ttf_parser::Face::parse(robot_buddy_game::text::FONT_BYTES, 0)
        .expect("bundled font should parse");

    // Exhaustively check the whole Unicode space: anything the name filter lets
    // through must be renderable, so a typed name can never tofu.
    let missing: Vec<char> = (0..=0x10FFFF)
        .filter_map(char::from_u32)
        .filter(|&ch| name_char_allowed(ch) && needs_glyph(ch) && !font_has_glyph(&face, ch))
        .collect();

    assert!(
        missing.is_empty(),
        "name-input filter accepts characters the bundled font can't draw: {}",
        missing
            .iter()
            .map(|ch| format!("{:?} (U+{:04X})", ch, *ch as u32))
            .collect::<Vec<_>>()
            .join(", "),
    );
}
