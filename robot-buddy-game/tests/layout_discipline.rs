//! Migrated panels can't bypass the layout layer.
//!
//! A panel that's on `ui::layout` gets every coordinate from its `Frame` and
//! every pixel from `ui::layout::paint`. This test parses each migrated
//! panel's source and fails if it calls macroquad's raw-coordinate drawing or
//! measuring functions (or reads the screen size / clock) itself — the only
//! way to reintroduce a hand-placed `y + 76` that collides with something.
//!
//! Migrating a panel = add it to `MIGRATED`. Only `ui/layout/paint.rs` (the
//! painter) may call these functions on a migrated panel's behalf.

use std::path::Path;

/// Panels that lay out through `ui::layout`.
const MIGRATED: &[&str] = &["shop.rs", "swag.rs", "quest.rs", "challenge.rs"];

/// Raw macroquad calls a migrated panel must not make.
const FORBIDDEN: &[&str] = &[
    // text
    "draw_text", "draw_text_ex", "measure_text", "draw_multiline_text",
    // shapes
    "draw_rectangle", "draw_rectangle_lines", "draw_rectangle_ex", "draw_circle", "draw_circle_lines",
    "draw_line", "draw_triangle", "draw_triangle_lines", "draw_ellipse", "draw_ellipse_lines",
    "draw_poly", "draw_poly_lines", "draw_arc", "draw_texture", "draw_texture_ex",
    // raw screen geometry / wall clock (layout gets the screen; blink is paint::blink)
    "screen_width", "screen_height", "get_time",
];

fn forbidden_calls(src: &str) -> Vec<String> {
    use syn::visit::Visit;

    struct V(Vec<String>);
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(p) = &*call.func {
                if let Some(last) = p.path.segments.last() {
                    let name = last.ident.to_string();
                    if FORBIDDEN.contains(&name.as_str()) {
                        self.0.push(name);
                    }
                }
            }
            syn::visit::visit_expr_call(self, call);
        }
        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
            // Passing the function itself (e.g. `.map(draw_text)`) counts too.
            if let Some(last) = p.path.segments.last() {
                let name = last.ident.to_string();
                if FORBIDDEN.contains(&name.as_str()) && p.path.segments.len() == 1 {
                    self.0.push(name);
                }
            }
            syn::visit::visit_expr_path(self, p);
        }
    }

    let file = syn::parse_file(src).expect("panel source parses");
    let mut v = V(Vec::new());
    v.visit_file(&file);
    v.0.sort();
    v.0.dedup();
    v.0
}

#[test]
fn migrated_panels_paint_only_through_the_layout_painter() {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    let mut offenders = Vec::new();
    for file in MIGRATED {
        let src = std::fs::read_to_string(ui.join(file)).unwrap_or_else(|e| panic!("read {file}: {e}"));
        let calls = forbidden_calls(&src);
        if !calls.is_empty() {
            offenders.push(format!("{file}: {}", calls.join(", ")));
        }
    }
    assert!(
        offenders.is_empty(),
        "migrated panels must draw via ui::layout::paint with rects from their Frame:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn the_scanner_catches_a_raw_draw() {
    let bad = "fn f() { draw_text(\"hi\", 1.0, 2.0, 3.0, WHITE); let w = screen_width(); }";
    assert_eq!(forbidden_calls(bad), vec!["draw_text", "screen_width"]);
    let ok = "fn f() { paint::text(t, WHITE); paint::fill(r, BLACK); }";
    assert!(forbidden_calls(ok).is_empty());
}
