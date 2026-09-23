//! Text shaping: pick a font size and line breaks for a text leaf inside a
//! given box, per its [`Fit`] policy. Engine-independent — the in-house engine
//! uses it to measure, `Frame::resolve` uses it to place, and a taffy engine
//! would call the same functions from its measure closure. One function
//! decides both "how big is this text" and "how is it drawn", so the two can't
//! disagree.

use super::metrics::TextMetrics;
use super::node::{Fit, TextSpec};
use super::rect::EPS;

/// A text leaf resolved against a box.
#[derive(Debug, Clone, PartialEq)]
pub struct Shaped {
    pub size: u16,
    pub lines: Vec<String>,
    /// The policy couldn't fit the text and it was cut. Never set for an
    /// intended [`Fit::Ellipsis`] cut; always a layout bug otherwise.
    pub overflowed: bool,
}

/// Greedy word wrap by measured width. A single word wider than `max_w` gets a
/// line of its own (the caller's fit loop then shrinks or flags it).
pub fn wrap(text: &str, max_w: f32, size: u16, m: &dyn TextMetrics) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let trial = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if !cur.is_empty() && m.width(&trial, size) > max_w + EPS {
            lines.push(std::mem::replace(&mut cur, word.to_string()));
        } else {
            cur = trial;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Largest size in `[min, max]` at which `text` fits `width`, measured with
/// `width_of`. `min` if nothing fits. Shared by every shrink-to-fit caller.
pub fn fit_size_by(text: &str, width: f32, max: u16, min: u16, width_of: impl Fn(&str, u16) -> f32) -> u16 {
    let mut size = max;
    while size > min && width_of(text, size) > width {
        size -= 1;
    }
    size
}

/// Gap between wrapped lines at size `s` (the spec's gap, scaled with the font).
pub fn line_gap(spec: &TextSpec, s: u16) -> f32 {
    if spec.size == 0 { 0.0 } else { (spec.line_gap * s as f32 / spec.size as f32).round() }
}

/// Height of `n` lines at size `s`.
pub fn block_height(spec: &TextSpec, n: usize, s: u16, m: &dyn TextMetrics) -> f32 {
    if n == 0 {
        return 0.0;
    }
    n as f32 * m.line_height(s) + (n - 1) as f32 * line_gap(spec, s)
}

fn lines_at(spec: &TextSpec, w: f32, s: u16, min: u16, m: &dyn TextMetrics) -> Vec<String> {
    if spec.text.trim().is_empty() {
        return Vec::new();
    }
    match spec.fit {
        Fit::Shrink { .. } | Fit::Ellipsis => vec![spec.text.clone()],
        Fit::Wrap { .. } => wrap(&spec.text, w, s, m),
        Fit::ShrinkThenWrap { .. } => {
            if s > min || m.width(&spec.text, s) <= w + EPS {
                vec![spec.text.clone()]
            } else {
                wrap(&spec.text, w, s, m)
            }
        }
    }
}

fn ellipsize(line: &str, w: f32, s: u16, m: &dyn TextMetrics) -> String {
    if m.width(line, s) <= w + EPS {
        return line.to_string();
    }
    let mut cut: String = line.to_string();
    while !cut.is_empty() && m.width(&format!("{}...", cut.trim_end()), s) > w + EPS {
        cut.pop();
    }
    format!("{}...", cut.trim_end())
}

/// Resolve `spec` into a `w`×`h` box: the largest size (down to the policy's
/// minimum) whose lines all fit; failing that, cut to fit at the minimum.
pub fn shape(spec: &TextSpec, w: f32, h: f32, m: &dyn TextMetrics) -> Shaped {
    let min = spec.fit.min_size(spec.size);
    let max_lines = spec.fit.max_lines();
    for s in (min..=spec.size).rev() {
        let lines = lines_at(spec, w, s, min, m);
        if max_lines > 0 && lines.len() > max_lines {
            continue;
        }
        if lines.iter().any(|l| m.width(l, s) > w + EPS) {
            continue;
        }
        if block_height(spec, lines.len(), s, m) > h + EPS {
            continue;
        }
        return Shaped { size: s, lines, overflowed: false };
    }
    // Nothing fits: keep what does at the minimum size and cut the rest.
    let s = min;
    let mut lines = lines_at(spec, w, s, min, m);
    let total = lines.len();
    let mut keep = if max_lines > 0 { total.min(max_lines) } else { total };
    while keep > 0 && block_height(spec, keep, s, m) > h + EPS {
        keep -= 1;
    }
    lines.truncate(keep);
    let dropped = keep < total;
    let n = lines.len();
    for (i, line) in lines.iter_mut().enumerate() {
        if dropped && i + 1 == n {
            *line = ellipsize(&format!("{line}..."), w, s, m);
        } else {
            *line = ellipsize(line, w, s, m);
        }
    }
    let intended = matches!(spec.fit, Fit::Ellipsis) && keep > 0;
    Shaped { size: s, lines, overflowed: !intended }
}

/// Max-content width: the whole text on one line at the preferred size.
pub fn natural_width(spec: &TextSpec, m: &dyn TextMetrics) -> f32 {
    m.width(&spec.text, spec.size)
}

/// Min-content width: the narrowest the policy can go without cutting.
pub fn min_width(spec: &TextSpec, m: &dyn TextMetrics) -> f32 {
    let min = spec.fit.min_size(spec.size);
    match spec.fit {
        Fit::Shrink { .. } => m.width(&spec.text, min),
        Fit::Wrap { .. } | Fit::ShrinkThenWrap { .. } => spec
            .text
            .split_whitespace()
            .map(|w| m.width(w, min))
            .fold(0.0, f32::max),
        Fit::Ellipsis => m.width("...", spec.size),
    }
}

/// Height the text wants at width `w` (no height limit).
pub fn natural_height(spec: &TextSpec, w: f32, m: &dyn TextMetrics) -> f32 {
    let s = shape(spec, w, f32::INFINITY, m);
    block_height(spec, s.lines.len(), s.size, m)
}

/// The least height the text can live in at width `w` without cutting.
pub fn min_height(spec: &TextSpec, w: f32, m: &dyn TextMetrics) -> f32 {
    let min = spec.fit.min_size(spec.size);
    let lines = lines_at(spec, w, min, min, m).len();
    let lines = match spec.fit.max_lines() {
        0 => lines,
        cap => lines.min(cap),
    };
    block_height(spec, lines, min, m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::metrics::FontMetrics;
    use crate::ui::layout::node::TextAlign;

    fn spec(t: &str, size: u16, fit: Fit) -> TextSpec {
        TextSpec { text: t.into(), size, fit, align: TextAlign::Left, line_gap: 4.0 }
    }

    #[test]
    fn wrap_breaks_on_measured_width() {
        let m = FontMetrics::bundled();
        // 8px per ASCII char at 16px: 10 chars per 80px line.
        let lines = wrap("aaaa bbbb cccc", 80.0, 16, m);
        assert_eq!(lines, vec!["aaaa bbbb", "cccc"]);
    }

    #[test]
    fn shrink_fits_width_then_flags_overflow() {
        let m = FontMetrics::bundled();
        let s = shape(&spec("Hello world", 24, Fit::shrink(12)), 100.0, 100.0, m);
        assert!(m.width("Hello world", s.size) <= 100.5 && !s.overflowed);
        let s = shape(&spec("Hello world", 24, Fit::shrink(20)), 60.0, 100.0, m);
        assert!(s.overflowed, "can't fit at the floor, so it is flagged");
        assert!(m.width(&s.lines[0], s.size) <= 60.5, "and still cut to the box");
    }

    #[test]
    fn shrink_then_wrap_prefers_one_line() {
        let m = FontMetrics::bundled();
        let sp = spec("one two three four", 32, Fit::shrink_then_wrap(16, 2));
        // 18 chars: at 16px = 144 wide. Room for 150 → one line at a shrunk size.
        let s = shape(&sp, 150.0, 200.0, m);
        assert_eq!(s.lines.len(), 1);
        // Only 100 wide → wrap at the floor.
        let s = shape(&sp, 100.0, 200.0, m);
        assert_eq!((s.lines.len(), s.size), (2, 16));
    }

    #[test]
    fn ellipsis_is_an_intended_cut() {
        let m = FontMetrics::bundled();
        let s = shape(&spec("A very long label", 16, Fit::Ellipsis), 64.0, 40.0, m);
        assert!(!s.overflowed);
        assert!(s.lines[0].ends_with("..."));
    }

    #[test]
    fn fit_size_by_shrinks_to_the_floor() {
        let width_of = |t: &str, size: u16| t.len() as f32 * size as f32 * 0.5;
        assert_eq!(fit_size_by("short", 900.0, 24, 11, width_of), 24);
        assert_eq!(fit_size_by(&"x".repeat(400), 10.0, 24, 11, width_of), 11);
    }
}
