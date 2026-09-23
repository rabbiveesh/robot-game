//! The CRA teaching visuals (dots, number bonds, base-ten blocks, groups).
//!
//! Every visual is built as a display list by ONE function, [`plan`], in local
//! coordinates. Its bounding box is the space it needs ([`extent`]), and
//! [`draw`] paints exactly that list inside the layout region the challenge
//! panel reserved. Measuring and drawing are the same computation, so the
//! reserved space can't disagree with what's painted — and when the natural
//! size is wider than the room, the whole visual is squeezed until it fits.
//!
//! Pure except [`draw`] (render-only, paints through `paint::Canvas`).

use crate::prelude::*;
use crate::ui::layout::{paint, FontMetrics, TextMetrics, UiRect};
use robot_buddy_domain::learning::challenge_generator::Challenge;

const BLUE_A: Color = Color::new(0.259, 0.647, 0.961, 1.0); // #42A5F5
const YELLOW_B: Color = Color::new(1.0, 0.835, 0.310, 1.0); // #FFD54F
const RED_TAKE: Color = Color::new(0.937, 0.263, 0.212, 1.0); // #EF5350
const RED_FAINT: Color = Color::new(0.957, 0.263, 0.212, 0.4);
const LABEL_GRAY: Color = Color::new(0.878, 0.878, 0.878, 1.0); // #E0E0E0
const HINT_GRAY: Color = Color::new(0.667, 0.667, 0.667, 1.0); // #AAA
const GROUP_LINE: Color = Color::new(0.329, 0.431, 0.478, 1.0); // #546E7A

/// Grouped visuals (× and ÷ as groups of dots) never grow past this, even on
/// a wide screen.
const GROUPS_MAX_W: f32 = 500.0;
/// Squeezed labels never go below this size.
const MIN_LABEL: u16 = 9;

/// One thing the visual paints, in local coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    Circle { x: f32, y: f32, r: f32, color: Color },
    Ring { x: f32, y: f32, r: f32, thickness: f32, color: Color },
    Rect { r: UiRect, color: Color },
    /// Outline drawn inside `r` (macroquad's `draw_rectangle_lines`).
    RectLines { r: UiRect, thickness: f32, color: Color },
    Line { x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color },
    Text { text: String, x: f32, baseline: f32, size: u16, color: Color },
}

/// A visual's display list and the box it covers.
#[derive(Debug, Clone, PartialEq)]
pub struct Visual {
    pub prims: Vec<Prim>,
    /// Bounding box of every prim (local coordinates).
    pub bbox: UiRect,
}

/// Space a visual needs, so a panel can reserve it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualExtent {
    pub w: f32,
    pub h: f32,
}

/// Collects prims and their bounding box. `s` scales every length and font.
struct Plot {
    prims: Vec<Prim>,
    s: f32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

impl Plot {
    fn new(s: f32) -> Self {
        Plot { prims: Vec::new(), s, x0: f32::INFINITY, y0: f32::INFINITY, x1: f32::NEG_INFINITY, y1: f32::NEG_INFINITY }
    }
    fn cover(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.x0 = self.x0.min(x0);
        self.y0 = self.y0.min(y0);
        self.x1 = self.x1.max(x1);
        self.y1 = self.y1.max(y1);
    }
    /// A length at this plot's scale.
    fn l(&self, v: f32) -> f32 {
        v * self.s
    }
    /// A font size at this plot's scale.
    fn fs(&self, size: u16) -> u16 {
        ((size as f32 * self.s).floor() as u16).clamp(MIN_LABEL.min(size), size)
    }
    fn circle(&mut self, x: f32, y: f32, r: f32, color: Color) {
        self.cover(x - r, y - r, x + r, y + r);
        self.prims.push(Prim::Circle { x, y, r, color });
    }
    fn ring(&mut self, x: f32, y: f32, r: f32, thickness: f32, color: Color) {
        let o = r + thickness / 2.0;
        self.cover(x - o, y - o, x + o, y + o);
        self.prims.push(Prim::Ring { x, y, r, thickness, color });
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.cover(x, y, x + w, y + h);
        self.prims.push(Prim::Rect { r: UiRect::new(x, y, w, h), color });
    }
    fn rect_lines(&mut self, x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color) {
        self.cover(x, y, x + w, y + h);
        self.prims.push(Prim::RectLines { r: UiRect::new(x, y, w, h), thickness, color });
    }
    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        let t = thickness / 2.0;
        self.cover(x1.min(x2) - t, y1.min(y2) - t, x1.max(x2) + t, y1.max(y2) + t);
        self.prims.push(Prim::Line { x1, y1, x2, y2, thickness, color });
    }
    /// Text with its left edge at `x`. Returns its width.
    fn text(&mut self, text: &str, x: f32, baseline: f32, size: u16, color: Color) -> f32 {
        let m = FontMetrics::bundled();
        let w = m.width(text, size);
        self.cover(x, baseline - m.ascent(size), x + w, baseline + m.descent(size));
        self.prims.push(Prim::Text { text: text.to_string(), x, baseline, size, color });
        w
    }
    /// Text centered on `cx`.
    fn text_c(&mut self, text: &str, cx: f32, baseline: f32, size: u16, color: Color) {
        let w = FontMetrics::bundled().width(text, size);
        self.text(text, cx - w / 2.0, baseline, size, color);
    }
    fn finish(self) -> Visual {
        let bbox = if self.prims.is_empty() {
            UiRect::default()
        } else {
            UiRect::new(self.x0, self.y0, self.x1 - self.x0, self.y1 - self.y0)
        };
        Visual { prims: self.prims, bbox }
    }
}

fn grouped(challenge: &Challenge) -> bool {
    let op = challenge.numbers.op.as_str();
    challenge.numbers.format != "bond"
        && (matches!(op, "\u{00f7}" | "/") || (challenge.sampled_band < 5 && matches!(op, "\u{00d7}" | "*")))
}

/// Build `challenge`'s visual at scale `s` (1 = natural size).
fn build(challenge: &Challenge, s: f32) -> Visual {
    let mut p = Plot::new(s);
    let a = challenge.numbers.a;
    let b = challenge.numbers.b;
    let op = challenge.numbers.op.as_str();
    if challenge.numbers.format == "bond" {
        // Number bond / missing addend: "What + b = total?"
        let total = challenge.numbers.bond_total.unwrap_or(a);
        bond(&mut p, total, b, challenge.correct_answer);
    } else if challenge.sampled_band >= 5 {
        base10_blocks(&mut p, a, b, op, challenge.correct_answer);
    } else {
        dots(&mut p, a, b, op);
    }
    p.finish()
}

/// Pure: `challenge`'s visual, squeezed (lengths and label sizes together)
/// until it is no wider than `max_w`. Whatever this returns is exactly what
/// [`draw`] paints.
pub fn plan(challenge: &Challenge, max_w: f32) -> Visual {
    let room = if grouped(challenge) { max_w.min(GROUPS_MAX_W) } else { max_w };
    let mut s = 1.0;
    let mut v = build(challenge, s);
    // Labels shrink in whole font sizes, so width isn't exactly linear in `s`:
    // iterate. Converges in one or two steps in practice.
    for _ in 0..8 {
        if v.bbox.w <= room || v.bbox.w <= 0.0 {
            break;
        }
        s *= (room / v.bbox.w) * 0.995;
        v = build(challenge, s);
    }
    v
}

/// Pure: the space [`draw`] needs for `challenge` inside `max_w`. It is the
/// true box of what gets painted — if even the squeezed visual can't fit,
/// `w > max_w` and the layout sweep says so.
pub fn extent(challenge: &Challenge, max_w: f32) -> VisualExtent {
    let v = plan(challenge, max_w);
    // Whole pixels: layout rounds box edges, and a fractional reservation
    // could round down below what's drawn.
    VisualExtent { w: v.bbox.w.ceil(), h: v.bbox.h.ceil() }
}

/// Paint `challenge`'s visual centered in `rect` (the region the layout
/// reserved from [`extent`]). Render-only.
pub fn draw(challenge: &Challenge, rect: UiRect) {
    let v = plan(challenge, rect.w);
    let dx = rect.x + (rect.w - v.bbox.w) / 2.0 - v.bbox.x;
    let dy = rect.y + (rect.h - v.bbox.h) / 2.0 - v.bbox.y;
    let c = paint::canvas(rect);
    for prim in &v.prims {
        match prim {
            Prim::Circle { x, y, r, color } => c.circle(x + dx, y + dy, *r, *color),
            Prim::Ring { x, y, r, thickness, color } => c.circle_lines(x + dx, y + dy, *r, *thickness, *color),
            Prim::Rect { r, color } => c.rect(UiRect::new(r.x + dx, r.y + dy, r.w, r.h), *color),
            Prim::RectLines { r, thickness, color } => {
                c.rect_lines(UiRect::new(r.x + dx, r.y + dy, r.w, r.h), *thickness, *color)
            }
            Prim::Line { x1, y1, x2, y2, thickness, color } => {
                c.line(x1 + dx, y1 + dy, x2 + dx, y2 + dy, *thickness, *color)
            }
            Prim::Text { text, x, baseline, size, color } => c.text(text, x + dx, baseline + dy, *size, *color),
        }
    }
}

// ─── NUMBER BOND (part-part-whole) ─────────────────────
//
// Renders a part-whole diagram for missing-addend problems ("? + b = total"):
//   [whole: total dots]  ← all dots together
//          ╱    ╲
//   [known]    [missing]
// The unknown part is drawn with empty circles + "?" so the child can see
// how many are still needed.

const BOND_DOT_R: f32 = 7.0;
const BOND_DOT_GAP: f32 = 5.0;
const BOND_BOX_PAD: f32 = 8.0;
const BOND_LABEL_SIZE: u16 = 18;

fn bond_box_width(p: &Plot, count: i32) -> f32 {
    let n = count.max(1) as f32;
    p.l(n * (BOND_DOT_R * 2.0 + BOND_DOT_GAP) - BOND_DOT_GAP + BOND_BOX_PAD * 2.0)
}

fn bond_box_h(p: &Plot) -> f32 {
    p.l(BOND_DOT_R * 2.0 + BOND_BOX_PAD * 2.0)
}

fn bond_box(p: &mut Plot, x: f32, y: f32, w: f32, color: Color) {
    // Soft rounded container: center rects + corner circles, then an outline.
    let h = bond_box_h(p);
    let r = p.l(8.0).min(w / 2.0).min(h / 2.0);
    let body = Color::new(color.r, color.g, color.b, 0.18);
    p.rect(x + r, y, w - 2.0 * r, h, body);
    p.rect(x, y + r, w, h - 2.0 * r, body);
    for (ccx, ccy) in [(x + r, y + r), (x + w - r, y + r), (x + r, y + h - r), (x + w - r, y + h - r)] {
        p.circle(ccx, ccy, r, body);
    }
    p.rect_lines(x, y, w, h, 2.0, color);
}

fn bond_dot_x(p: &Plot, x: f32, i: i32) -> f32 {
    x + p.l(BOND_BOX_PAD + BOND_DOT_R + i as f32 * (BOND_DOT_R * 2.0 + BOND_DOT_GAP))
}

fn bond(p: &mut Plot, total: i32, known: i32, missing: i32) {
    // Render count is bounded so things don't run off the panel.
    let t_count = total.min(20);
    let k_count = known.min(20);
    let m_count = missing.min(20);
    let unknown_color = LABEL_GRAY;
    let label = p.fs(BOND_LABEL_SIZE);
    let h = bond_box_h(p);
    let dot_r = p.l(BOND_DOT_R);

    // Row 1: the whole
    let whole_w = bond_box_width(p, t_count);
    let whole_x = -whole_w / 2.0;
    let whole_y = 0.0;

    // Row 2: known + missing, side by side
    let known_w = bond_box_width(p, k_count);
    let miss_w = bond_box_width(p, m_count);
    let parts_total = known_w + p.l(32.0) + miss_w;
    let known_x = -parts_total / 2.0;
    let miss_x = known_x + known_w + p.l(32.0);
    let parts_y = whole_y + h + p.l(44.0);

    // Connector lines (whole → each part)
    let line_color = Color::new(0.690, 0.745, 0.773, 1.0);
    p.line(0.0, whole_y + h, known_x + known_w / 2.0, parts_y, 2.0, line_color);
    p.line(0.0, whole_y + h, miss_x + miss_w / 2.0, parts_y, 2.0, line_color);

    // Whole
    bond_box(p, whole_x, whole_y, whole_w, BLUE_A);
    for i in 0..t_count {
        let x = bond_dot_x(p, whole_x, i);
        p.circle(x, whole_y + h / 2.0, dot_r, BLUE_A);
    }
    p.text_c(&total.to_string(), 0.0, whole_y - 6.0, label, LABEL_GRAY);

    // Known part
    bond_box(p, known_x, parts_y, known_w, YELLOW_B);
    for i in 0..k_count {
        let x = bond_dot_x(p, known_x, i);
        p.circle(x, parts_y + h / 2.0, dot_r, YELLOW_B);
    }
    p.text_c(&known.to_string(), known_x + known_w / 2.0, parts_y + h + p.l(20.0), label, YELLOW_B);

    // Missing part: hollow circles with a "?" over the group.
    bond_box(p, miss_x, parts_y, miss_w, unknown_color);
    for i in 0..m_count {
        let x = bond_dot_x(p, miss_x, i);
        p.ring(x, parts_y + h / 2.0, dot_r, 2.0, unknown_color);
    }
    let q = p.fs(22);
    p.text_c("?", miss_x + miss_w / 2.0, parts_y + h / 2.0 + p.l(8.0), q, unknown_color);
    p.text_c("?", miss_x + miss_w / 2.0, parts_y + h + p.l(20.0), label, unknown_color);
}

// ─── DOT VISUAL (bands 1-4) ────────────────────────────

fn dots(p: &mut Plot, a: i32, b: i32, op: &str) {
    let dot_r = p.l(5.0);
    let gap = p.l(4.0);
    let step = dot_r * 2.0 + gap;
    let label = p.fs(16);

    match op {
        "+" => {
            let total = a + b;
            let per_row = total.min(10);
            let start_x = -(per_row as f32 * step) / 2.0;
            for idx in 0..total.max(0) {
                let (row, col) = (idx / 10, idx % 10);
                let color = if idx < a { BLUE_A } else { YELLOW_B };
                p.circle(start_x + col as f32 * step + dot_r, row as f32 * (step + gap) + dot_r, dot_r, color);
            }
            let label_y = ((total - 1).max(0) / 10 + 1) as f32 * (step + gap) + p.l(12.0);
            p.text_c(&a.to_string(), -p.l(40.0), label_y, label, BLUE_A);
            p.text_c("+", 0.0, label_y, label, HINT_GRAY);
            p.text_c(&b.to_string(), p.l(40.0), label_y, label, YELLOW_B);
        }
        "-" | "\u{2212}" => {
            let per_row = a.min(10);
            let start_x = -(per_row as f32 * step) / 2.0;
            let x_arm = p.l(3.0);
            for i in 0..a.max(0) {
                let (row, col) = (i / 10, i % 10);
                let dx = start_x + col as f32 * step + dot_r;
                let dy = row as f32 * (step + gap) + dot_r;
                if i >= a - b {
                    // "Taken away" dots, crossed out.
                    p.circle(dx, dy, dot_r, RED_FAINT);
                    p.line(dx - x_arm, dy - x_arm, dx + x_arm, dy + x_arm, 2.0, RED_TAKE);
                    p.line(dx + x_arm, dy - x_arm, dx - x_arm, dy + x_arm, 2.0, RED_TAKE);
                } else {
                    p.circle(dx, dy, dot_r, BLUE_A);
                }
            }
            let label_y = ((a - 1).max(0) / 10 + 1) as f32 * (step + gap) + p.l(12.0);
            p.text_c(&format!("{} - {} = count the blue ones!", a, b), 0.0, label_y, label, BLUE_A);
        }
        "\u{00d7}" | "*" => {
            // a groups of b dots
            let groups = a.min(8);
            let per_group = b.min(10);
            let group_gap = p.l(30.0);
            let group_w = per_group as f32 * step + group_gap;
            let total_w = groups as f32 * group_w - group_gap;
            let start_x = -total_w / 2.0;
            p.text_c(&format!("{} groups of {}", a, b), 0.0, -p.l(8.0), p.fs(14), HINT_GRAY);
            for g in 0..groups {
                let gx = start_x + g as f32 * group_w;
                let color = if g % 2 == 0 { BLUE_A } else { YELLOW_B };
                for d in 0..per_group {
                    p.circle(gx + d as f32 * step + dot_r, p.l(10.0) + dot_r, dot_r, color);
                }
            }
        }
        "\u{00f7}" | "/" => split_groups(p, a, b, (a / b.max(1)).min(12), 4.0),
        _ => {}
    }
}

/// "a split into b groups": boxed groups of `per_group` dots with a count
/// under each. Shared by the dot and block visuals (they differ in dot gap).
fn split_groups(p: &mut Plot, a: i32, b: i32, per_group: i32, dot_gap: f32) {
    let groups = b.min(8);
    let dot_r = p.l(5.0);
    let step = dot_r * 2.0 + p.l(dot_gap);
    let group_w = per_group as f32 * step + p.l(10.0);
    let total_w = groups as f32 * group_w;
    let start_x = -total_w / 2.0;
    p.text_c(&format!("{} split into {} groups", a, b), 0.0, -p.l(8.0), p.fs(14), HINT_GRAY);
    let count_size = p.fs(11).max(MIN_LABEL);
    for g in 0..groups {
        let gx = start_x + g as f32 * group_w;
        let inner_w = group_w - p.l(4.0);
        p.rect_lines(gx, p.l(2.0), inner_w, dot_r * 2.0 + p.l(8.0), 1.0, GROUP_LINE);
        let color = if g % 2 == 0 { BLUE_A } else { YELLOW_B };
        for d in 0..per_group {
            p.circle(gx + p.l(4.0) + d as f32 * step + dot_r, p.l(6.0) + dot_r, dot_r, color);
        }
        p.text_c(&per_group.to_string(), gx + inner_w / 2.0, dot_r * 2.0 + p.l(22.0), count_size, HINT_GRAY);
    }
}

// ─── BASE-10 BLOCKS (bands 5+) ─────────────────────────

const ROD_W: f32 = 10.0;
const ROD_H: f32 = 44.0;
const FIVE_H: f32 = 22.0;
const CUBE: f32 = 10.0;
const BLOCK_GAP: f32 = 3.0;
/// Rods drawn per number (bigger tens still count in the label).
const MAX_RODS: i32 = 15;

struct BlockColors {
    rod: Color,
    cube: Color,
    five: Color,
}

const COLORS_A: BlockColors = BlockColors {
    rod: Color::new(0.259, 0.647, 0.961, 1.0),  // #42A5F5
    cube: Color::new(0.392, 0.710, 0.965, 1.0), // #64B5F6
    five: Color::new(0.400, 0.733, 0.416, 1.0), // #66BB6A
};

const COLORS_B: BlockColors = BlockColors {
    rod: Color::new(1.0, 0.835, 0.310, 1.0),    // #FFD54F
    cube: Color::new(1.0, 0.878, 0.510, 1.0),   // #FFE082
    five: Color::new(0.506, 0.780, 0.518, 1.0), // #81C784
};

const COLORS_RED: BlockColors = BlockColors {
    rod: Color::new(0.937, 0.263, 0.212, 1.0),  // #EF5350
    cube: Color::new(0.937, 0.604, 0.604, 1.0), // #EF9A9A
    five: Color::new(0.898, 0.451, 0.451, 1.0), // #E57373
};

/// Width of one number's blocks.
fn num_w(p: &Plot, num: i32) -> f32 {
    let tens = (num / 10).min(MAX_RODS);
    let ones = num % 10;
    let rods_w = tens as f32 * (ROD_W + BLOCK_GAP);
    let ones_w = (ones / 5) as f32 * (ROD_W + BLOCK_GAP) + (ones % 5) as f32 * (CUBE + BLOCK_GAP);
    p.l(rods_w.max(ones_w).max(20.0))
}

fn num_h(p: &Plot, num: i32) -> f32 {
    let tens = num / 10;
    let ones = num % 10;
    let ones_h = if ones >= 5 { FIVE_H } else if ones > 0 { CUBE } else { 0.0 };
    p.l(if tens > 0 { ROD_H + 5.0 + ones_h } else { ones_h.max(CUBE) })
}

fn num_blocks(p: &mut Plot, x: f32, num: i32, colors: &BlockColors) {
    let tens = num / 10;
    let ones = num % 10;
    let outline = Color::new(0.0, 0.0, 0.0, 0.3);
    let (rod_w, rod_h, five_h, cube, gap) = (p.l(ROD_W), p.l(ROD_H), p.l(FIVE_H), p.l(CUBE), p.l(BLOCK_GAP));

    let w = num_w(p, num);
    let size = p.fs(16);
    p.text_c(&num.to_string(), x + w / 2.0, -p.l(6.0), size, LABEL_GRAY);

    // Tens rods
    for i in 0..tens.min(MAX_RODS) {
        let rx = x + i as f32 * (rod_w + gap);
        p.rect(rx, 0.0, rod_w, rod_h, colors.rod);
        p.rect_lines(rx, 0.0, rod_w, rod_h, 1.0, outline);
    }

    // Ones row: 5-bars, then remainder cubes.
    let ones_y = if tens > 0 { rod_h + p.l(5.0) } else { 0.0 };
    let mut ox = x;
    for _ in 0..ones / 5 {
        p.rect(ox, ones_y, rod_w, five_h, colors.five);
        p.rect_lines(ox, ones_y, rod_w, five_h, 1.0, outline);
        ox += rod_w + gap;
    }
    let cube_y = if ones >= 5 { ones_y + (five_h - cube) / 2.0 } else { ones_y };
    for _ in 0..ones % 5 {
        p.rect(ox, cube_y, cube, cube, colors.cube);
        p.rect_lines(ox, cube_y, cube, cube, 1.0, outline);
        ox += cube + gap;
    }
}

fn base10_blocks(p: &mut Plot, a: i32, b: i32, op: &str, answer: i32) {
    match op {
        "+" | "-" | "\u{2212}" => {
            let (wa, wb) = (num_w(p, a), num_w(p, b));
            let op_gap = p.l(40.0);
            let start_x = -(wa + op_gap + wb) / 2.0;
            num_blocks(p, start_x, a, &COLORS_A);
            let sym_x = start_x + wa + op_gap / 2.0;
            let sym_y = num_h(p, a).max(num_h(p, b)) / 2.0 + p.l(4.0);
            if op == "+" {
                let size = p.fs(28);
                p.text_c("+", sym_x, sym_y, size, WHITE);
                num_blocks(p, start_x + wa + op_gap, b, &COLORS_B);
            } else {
                // Minus as a line: crisper than the glyph at this size.
                let half = p.l(8.0);
                p.line(sym_x - half, sym_y - p.l(4.0), sym_x + half, sym_y - p.l(4.0), 3.0, WHITE);
                num_blocks(p, start_x + wa + op_gap, b, &COLORS_RED);
            }
        }
        "\u{00d7}" | "*" => {
            // Array: rows × cols dots
            let rows = a.min(b).min(12);
            let cols = a.max(b).min(12);
            let dot_r = p.l(5.0);
            let step = dot_r * 2.0 + p.l(4.0);
            let start_x = -(cols as f32 * step) / 2.0;
            p.text_c(&format!("{} rows of {}", rows, cols), 0.0, -p.l(8.0), p.fs(14), HINT_GRAY);
            for r in 0..rows {
                let color = if r % 2 == 0 { BLUE_A } else { YELLOW_B };
                for c in 0..cols {
                    p.circle(start_x + c as f32 * step + dot_r, p.l(5.0) + r as f32 * step + dot_r, dot_r, color);
                }
            }
        }
        "\u{00f7}" | "/" => split_groups(p, a, b, answer.min(12), 3.0),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::rand::rngs::SmallRng;
    use ::rand::SeedableRng;
    use robot_buddy_domain::learning::challenge_generator::{generate_challenge, ChallengeProfile};
    use robot_buddy_domain::learning::operation_stats::OperationStats;

    fn challenges(per_band: usize) -> Vec<Challenge> {
        let mut rng = SmallRng::seed_from_u64(7);
        (1..=10u8)
            .flat_map(|band| {
                let profile = ChallengeProfile { math_band: band, spread_width: 0.0, operation_stats: OperationStats::new() };
                (0..per_band).map(|_| generate_challenge(&profile, &mut rng)).collect::<Vec<_>>()
            })
            .collect()
    }

    /// The phone bug: a band-4 bond needed ~415px and reported the slot's
    /// width instead. Now every visual fits every width a panel can offer,
    /// and the extent is the box of what's actually drawn.
    #[test]
    fn every_visual_fits_the_width_it_was_given() {
        for c in challenges(40) {
            for max_w in [180.0, 272.0, 344.0, 432.0, 712.0] {
                let v = plan(&c, max_w);
                assert!(
                    v.bbox.w <= max_w + 0.5,
                    "{} {} {} (band {}, {}) is {}px wide in a {max_w}px slot",
                    c.numbers.a, c.numbers.op, c.numbers.b, c.sampled_band, c.numbers.format, v.bbox.w
                );
                assert_eq!(extent(&c, max_w), VisualExtent { w: v.bbox.w.ceil(), h: v.bbox.h.ceil() });
            }
        }
    }

    #[test]
    fn a_big_bond_squeezes_instead_of_lying() {
        let mut c = challenges(1).remove(0);
        c.numbers.format = "bond".into();
        c.numbers.bond_total = Some(20);
        c.numbers.b = 10;
        c.correct_answer = 10;
        let natural = plan(&c, 10_000.0).bbox.w;
        assert!(natural > 400.0, "a 10+10 bond is wide ({natural})");
        let squeezed = plan(&c, 272.0);
        assert!(squeezed.bbox.w <= 272.0 && squeezed.bbox.w > 200.0, "{}", squeezed.bbox.w);
    }

    #[test]
    fn a_visual_that_fits_is_not_squeezed() {
        for c in challenges(5) {
            let wide = plan(&c, 10_000.0);
            if wide.bbox.w <= 300.0 && !grouped(&c) {
                assert_eq!(plan(&c, 300.0), wide);
            }
        }
    }
}
