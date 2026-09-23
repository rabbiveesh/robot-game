//! Migrated panels can't bypass the layout layer.
//!
//! A panel that's on `ui::layout` gets every coordinate from its `Frame` and
//! every pixel from `ui::layout::paint`. This test parses each migrated
//! panel's source (syn) and fails if it:
//!
//! * calls macroquad's raw drawing / measuring / camera functions, reads the
//!   screen size or the clock — including through a `use … as alias`, as a
//!   function value (`.map(draw_text)`), or inside a macro's tokens;
//! * calls ANY `draw_*` / `gl_*` function that isn't the panel's own (defined
//!   in the same file) or in another migrated module / the painter — so an
//!   unmigrated raw-drawing helper can't be smuggled in;
//! * makes rects by hand (`UiRect::new`, `UiRect { .. }`, `.inset(..)`,
//!   `.expand(..)`) outside a short, commented allowlist.
//!
//! What it can't see: arithmetic on a frame's rect fields (`r.x + 6.0`) that
//! ends up in a paint call; the painter's `Canvas` warns at runtime in debug
//! builds when custom art strays outside its region.
//!
//! Migrating a panel = add it to `MIGRATED`. Only `ui/layout/paint.rs` (the
//! painter) may call these functions on a migrated panel's behalf.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Panels (and panel art) that lay out through `ui::layout`.
const MIGRATED: &[&str] = &[
    "shop.rs",
    "swag.rs",
    "swatches.rs",
    "quest.rs",
    "challenge.rs",
    "dialogue.rs",
    "settings_overlay.rs",
    "visuals.rs",
    "pearl_hop.rs",
];

/// Raw macroquad calls a migrated panel must not make (besides every
/// `draw_*` / `gl_*`, which are caught by prefix).
const FORBIDDEN: &[&str] = &[
    // text measuring (layout measures; paint draws)
    "measure_text", "get_text_center", "camera_font_scale",
    // screen geometry / camera / wall clock (layout gets the screen; blink is paint::blink)
    "screen_width", "screen_height", "screen_dpi_scale", "get_time",
    "set_camera", "set_default_camera", "push_camera_state", "pop_camera_state",
    "get_context", "clear_background",
];

fn forbidden_name(name: &str) -> bool {
    FORBIDDEN.contains(&name) || name == "draw" || name.starts_with("draw_") || name.starts_with("gl_")
}

/// Module names whose `draw_*`-style functions are allowed: the painter and
/// the migrated modules themselves (they're scanned too).
fn trusted_module(m: &str) -> bool {
    m == "paint" || MIGRATED.iter().any(|f| f.trim_end_matches(".rs") == m)
}

/// Hand-made rects that are deliberate, with why. (file, enclosing fn or
/// "*", construct).
const COORD_ALLOW: &[(&str, &str, &str)] = &[
    // The name tab straddles the dialogue box's top edge: the box is painted
    // from the tab's midline down.
    ("dialogue.rs", "draw", ".inset"),
    // The swatch being worn gets a gold frame just outside the swatch.
    ("swatches.rs", "paint_swatch", ".expand"),
    // Display-list prims in the visual's local coordinates; painted through a
    // paint::Canvas bound to the layout region, which checks they stay inside.
    ("visuals.rs", "*", "UiRect::new"),
];

#[derive(Default)]
struct Scan {
    /// Forbidden calls found (deduped, sorted).
    calls: BTreeSet<String>,
    /// Hand-made coordinates: (enclosing fn, construct).
    coords: BTreeSet<(String, String)>,
}

fn scan(src: &str) -> Scan {
    use syn::visit::Visit;

    let file = syn::parse_file(src).expect("panel source parses");

    // `use a::b::c as d` → d ↦ [a, b, c]; plain `use a::b::c` → c ↦ [a, b, c].
    fn uses(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut BTreeMap<String, Vec<String>>) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                uses(&p.tree, prefix, out);
                prefix.pop();
            }
            syn::UseTree::Name(n) => {
                let mut full = prefix.clone();
                full.push(n.ident.to_string());
                out.insert(n.ident.to_string(), full);
            }
            syn::UseTree::Rename(r) => {
                let mut full = prefix.clone();
                full.push(r.ident.to_string());
                out.insert(r.rename.to_string(), full);
            }
            syn::UseTree::Group(g) => g.items.iter().for_each(|t| uses(t, prefix, out)),
            syn::UseTree::Glob(_) => {}
        }
    }

    struct Collect {
        imports: BTreeMap<String, Vec<String>>,
        local_fns: BTreeSet<String>,
    }
    impl<'ast> Visit<'ast> for Collect {
        fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
            uses(&u.tree, &mut Vec::new(), &mut self.imports);
        }
        fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
            self.local_fns.insert(f.sig.ident.to_string());
            syn::visit::visit_item_fn(self, f);
        }
        fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
            self.local_fns.insert(f.sig.ident.to_string());
            syn::visit::visit_impl_item_fn(self, f);
        }
    }
    let mut c = Collect { imports: BTreeMap::new(), local_fns: BTreeSet::new() };
    c.visit_file(&file);

    struct V<'a> {
        c: &'a Collect,
        out: Scan,
        fn_stack: Vec<String>,
    }
    impl V<'_> {
        /// Resolve a path through the file's imports, then judge it.
        fn check_path(&mut self, segs: &[String]) {
            let mut full: Vec<String> = match self.c.imports.get(&segs[0]) {
                Some(imported) => imported.iter().cloned().chain(segs[1..].iter().cloned()).collect(),
                None => segs.to_vec(),
            };
            let name = full.pop().unwrap();
            if !forbidden_name(&name) {
                return;
            }
            let local = segs.len() == 1 && !self.c.imports.contains_key(&segs[0]) && self.c.local_fns.contains(&name);
            let trusted = full.last().is_some_and(|m| trusted_module(m) || m == "Self" || m == "self");
            if !local && !trusted {
                full.push(name);
                self.out.calls.insert(full.join("::"));
            }
        }
        fn coord(&mut self, what: &str) {
            let f = self.fn_stack.last().cloned().unwrap_or_default();
            self.out.coords.insert((f, what.to_string()));
        }
        /// Macro bodies aren't parsed by syn: walk their tokens for
        /// `name(` / `a::b::name(` call shapes and `UiRect::new` / `UiRect {`.
        fn scan_tokens(&mut self, tokens: proc_macro2::TokenStream) {
            use proc_macro2::{Delimiter, TokenTree};
            let toks: Vec<TokenTree> = tokens.into_iter().collect();
            let mut path: Vec<String> = Vec::new();
            for (i, t) in toks.iter().enumerate() {
                match t {
                    TokenTree::Ident(id) => {
                        path.push(id.to_string());
                        match toks.get(i + 1) {
                            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
                                if path.len() >= 2 && path[path.len() - 2] == "UiRect" && path[path.len() - 1] == "new" {
                                    self.coord("UiRect::new");
                                }
                                let p = path.clone();
                                self.check_path(&p);
                                path.clear();
                            }
                            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace && id == "UiRect" => {
                                self.coord("UiRect { .. }");
                                path.clear();
                            }
                            Some(TokenTree::Punct(p)) if p.as_char() == ':' => {}
                            _ => {
                                // A bare function value inside a macro, e.g. `m!(xs.map(draw_text))`.
                                let p = std::mem::take(&mut path);
                                if p.len() == 1 && forbidden_name(&p[0]) {
                                    self.check_path(&p);
                                }
                            }
                        }
                    }
                    TokenTree::Punct(p) if p.as_char() == ':' => {}
                    TokenTree::Punct(p) if p.as_char() == '.' => {
                        // Method call in a macro: `.inset(` / `.expand(`.
                        if let (Some(TokenTree::Ident(m)), Some(TokenTree::Group(g))) = (toks.get(i + 1), toks.get(i + 2)) {
                            if g.delimiter() == Delimiter::Parenthesis && (m == "inset" || m == "expand") {
                                self.coord(&format!(".{m}"));
                            }
                        }
                        path.clear();
                    }
                    TokenTree::Group(g) => {
                        path.clear();
                        self.scan_tokens(g.stream());
                    }
                    _ => path.clear(),
                }
            }
        }
    }
    fn segs(p: &syn::Path) -> Vec<String> {
        p.segments.iter().map(|s| s.ident.to_string()).collect()
    }
    fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|a| {
            a.path().is_ident("cfg") && a.parse_args::<syn::Meta>().is_ok_and(|m| m.path().is_ident("test"))
        })
    }
    impl<'ast> Visit<'ast> for V<'_> {
        fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
            // Unit tests may build rects by hand to probe the panel.
            if !is_cfg_test(&m.attrs) {
                syn::visit::visit_item_mod(self, m);
            }
        }
        fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
            self.fn_stack.push(f.sig.ident.to_string());
            syn::visit::visit_item_fn(self, f);
            self.fn_stack.pop();
        }
        fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
            self.fn_stack.push(f.sig.ident.to_string());
            syn::visit::visit_impl_item_fn(self, f);
            self.fn_stack.pop();
        }
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(p) = &*call.func {
                let s = segs(&p.path);
                if s.len() >= 2 && s[s.len() - 2] == "UiRect" && s[s.len() - 1] == "new" {
                    self.coord("UiRect::new");
                }
            }
            syn::visit::visit_expr_call(self, call);
        }
        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
            // Every path expression: the callee of a call, or a function
            // passed by value (`.map(draw_text)`).
            self.check_path(&segs(&p.path));
            syn::visit::visit_expr_path(self, p);
        }
        fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
            let name = m.method.to_string();
            if name == "inset" || name == "expand" {
                self.coord(&format!(".{name}"));
            }
            syn::visit::visit_expr_method_call(self, m);
        }
        fn visit_expr_struct(&mut self, s: &'ast syn::ExprStruct) {
            if s.path.segments.last().is_some_and(|l| l.ident == "UiRect") {
                self.coord("UiRect { .. }");
            }
            syn::visit::visit_expr_struct(self, s);
        }
        fn visit_macro(&mut self, m: &'ast syn::Macro) {
            self.scan_tokens(m.tokens.clone());
        }
    }

    let mut v = V { c: &c, out: Scan::default(), fn_stack: Vec::new() };
    v.visit_file(&file);
    v.out
}

fn coord_allowed(file: &str, f: &str, what: &str) -> bool {
    COORD_ALLOW.iter().any(|&(af, afn, aw)| af == file && (afn == "*" || afn == f) && aw == what)
}

fn read(file: &str) -> String {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    std::fs::read_to_string(ui.join(file)).unwrap_or_else(|e| panic!("read {file}: {e}"))
}

#[test]
fn migrated_panels_paint_only_through_the_layout_painter() {
    let mut offenders = Vec::new();
    for file in MIGRATED {
        let s = scan(&read(file));
        if !s.calls.is_empty() {
            offenders.push(format!("{file}: {}", s.calls.into_iter().collect::<Vec<_>>().join(", ")));
        }
    }
    assert!(
        offenders.is_empty(),
        "migrated panels must draw via ui::layout::paint with rects from their Frame:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn migrated_panels_take_rects_from_their_frame() {
    let mut offenders = Vec::new();
    let mut used = BTreeSet::new();
    for file in MIGRATED {
        for (f, what) in scan(&read(file)).coords {
            if coord_allowed(file, &f, &what) {
                used.insert(COORD_ALLOW.iter().position(|&(af, afn, aw)| af == *file && (afn == "*" || afn == f) && aw == what));
            } else {
                offenders.push(format!("{file}: {what} in fn {f}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "hand-made rects in migrated panels (lay it out as a node instead, or add a commented COORD_ALLOW entry):\n  {}",
        offenders.join("\n  ")
    );
    let stale: Vec<_> = (0..COORD_ALLOW.len()).filter(|i| !used.contains(&Some(*i))).map(|i| COORD_ALLOW[i]).collect();
    assert!(stale.is_empty(), "COORD_ALLOW entries no longer needed: {stale:?}");
}

#[test]
fn every_migrated_file_exists() {
    for file in MIGRATED {
        read(file);
    }
}

// ─── The scanner itself ─────────────────────────────────

fn calls(src: &str) -> Vec<String> {
    scan(src).calls.into_iter().collect()
}

#[test]
fn the_scanner_catches_a_raw_draw() {
    let bad = "fn f() { draw_text(\"hi\", 1.0, 2.0, 3.0, WHITE); let w = screen_width(); }";
    assert_eq!(calls(bad), vec!["draw_text", "screen_width"]);
    let ok = "fn f() { paint::text(t, WHITE); paint::fill(r, BLACK); c.circle(1.0, 2.0, 3.0, RED); }";
    assert!(calls(ok).is_empty());
}

#[test]
fn the_scanner_sees_through_aliases() {
    let src = "use macroquad::prelude::draw_rectangle as rect; use macroquad::shapes as sh;
               fn f() { rect(0.0, 0.0, 1.0, 1.0, RED); sh::draw_hexagon(0.0, 0.0, 1.0, 1.0, true, RED, RED); }";
    assert_eq!(calls(src), vec!["macroquad::prelude::draw_rectangle", "macroquad::shapes::draw_hexagon"]);
}

#[test]
fn the_scanner_looks_inside_macros() {
    let src = "fn f() { let v = vec![measure_text(\"a\", None, 1, 1.0)]; my_macro!(xs.iter().for_each(draw_text));
               log!(\"{}\", macroquad::text::draw_multiline_text_ex(t, 0.0, 0.0, None, p)); }";
    assert_eq!(calls(src), vec!["draw_text", "macroquad::text::draw_multiline_text_ex", "measure_text"]);
}

#[test]
fn the_scanner_catches_every_draw_fn_by_prefix() {
    let src = "fn f() { draw_rectangle_lines_ex(0.0, 0.0, 1.0, 1.0, 1.0, p); draw_texture(t, 0.0, 0.0, WHITE); gl_use_material(m); }";
    assert_eq!(calls(src), vec!["draw_rectangle_lines_ex", "draw_texture", "gl_use_material"]);
}

#[test]
fn the_scanner_flags_unmigrated_helpers_but_not_local_or_migrated_ones() {
    let src = "use crate::sprites::player;
               fn draw_star_burst() {}
               fn f() { draw_star_burst(); visuals::draw(c, r); super::visuals::draw(c, r); paint::text(t, WHITE);
                        crate::ui::descent::draw_stones(s); player::draw_player(x, y); descent::draw(s); }";
    assert_eq!(calls(src), vec!["crate::sprites::player::draw_player", "crate::ui::descent::draw_stones", "descent::draw"]);
}

#[test]
fn the_scanner_finds_hand_made_rects() {
    let src = "fn draw() { let a = UiRect::new(1.0, 2.0, 3.0, 4.0); let b = UiRect { x: 1.0, y: 2.0, w: 3.0, h: 4.0 };
               paint::fill(r.inset(1.0, 1.0, 1.0, 1.0), RED); dbg!(r.expand(2.0)); }
               #[cfg(test)] mod tests { fn t() { UiRect::new(0.0, 0.0, 1.0, 1.0); } }";
    let coords: Vec<String> = scan(src).coords.into_iter().map(|(f, w)| format!("{f}:{w}")).collect();
    assert_eq!(coords, vec!["draw:.expand", "draw:.inset", "draw:UiRect { .. }", "draw:UiRect::new"]);
}
