//! Mini-Sudoku puzzle UI.
//!
//! Single entrypoint surface (mirrors `ui::kenken`):
//!   - `layout(session, screen)` — pure, hit-testable rectangles
//!   - `draw_sudoku(session, layout, selected)` — render
//!   - `handle_click(mx, my, session, layout, selected)` → SudokuInput
//!   - `handle_key(session, input, selected)` → SudokuInput
//!
//! 4×4 boards render in "picture mode" — each symbol is a colored shape, so a
//! pre-reader can play. 6×6 boards use digits.

use crate::prelude::*;
use robot_buddy_domain::logic::sudoku::{
    SudokuAction, SudokuPhase, SudokuSession, SudokuValidation,
};

use crate::input::FrameInput;

#[derive(Debug, Clone, Copy)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UiRect {
    pub fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + self.h
    }
}

pub struct PickerBound {
    pub rect: UiRect,
    pub value: u8,
}

pub struct SudokuLayout {
    pub panel: UiRect,
    pub cells: Vec<Vec<UiRect>>,
    pub pickers: Vec<PickerBound>,
    pub clear_btn: UiRect,
}

pub enum SudokuInput {
    Action(SudokuAction),
    SelectCell(u8, u8),
    Deselect,
}

// ─── Layout (pure) ──────────────────────────────────────

pub fn layout(session: &SudokuSession, screen: (f32, f32)) -> SudokuLayout {
    let (sw, sh) = screen;
    let panel_w = (sw - 40.0).min(720.0);
    let panel_h = (sh - 40.0).min(680.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;

    let n = session.puzzle.grid_size as usize;
    let grid_max = panel_w.min(panel_h - 200.0).min(440.0);
    let cell_size = (grid_max / n as f32).floor();
    let grid_px = cell_size * n as f32;
    let grid_x = panel_x + (panel_w - grid_px) / 2.0;
    let grid_y = panel_y + 56.0;

    let mut cells = Vec::with_capacity(n);
    for r in 0..n {
        let mut row = Vec::with_capacity(n);
        for c in 0..n {
            row.push(UiRect {
                x: grid_x + c as f32 * cell_size,
                y: grid_y + r as f32 * cell_size,
                w: cell_size,
                h: cell_size,
            });
        }
        cells.push(row);
    }

    let pickers_y = grid_y + grid_px + 22.0;
    let picker_size = (((panel_w - 32.0) / n as f32) - 10.0).min(60.0).max(36.0);
    let picker_gap = 10.0;
    let total_w = picker_size * n as f32 + picker_gap * (n as f32 - 1.0);
    let start_x = panel_x + (panel_w - total_w) / 2.0;
    let mut pickers = Vec::with_capacity(n);
    for i in 0..n {
        pickers.push(PickerBound {
            rect: UiRect {
                x: start_x + i as f32 * (picker_size + picker_gap),
                y: pickers_y,
                w: picker_size,
                h: picker_size,
            },
            value: (i + 1) as u8,
        });
    }

    let btn_w = 130.0;
    let btn_h = 40.0;
    let clear_btn = UiRect {
        x: panel_x + (panel_w - btn_w) / 2.0,
        y: pickers_y + picker_size + 16.0,
        w: btn_w,
        h: btn_h,
    };

    SudokuLayout {
        panel: UiRect { x: panel_x, y: panel_y, w: panel_w, h: panel_h },
        cells,
        pickers,
        clear_btn,
    }
}

// ─── Input ──────────────────────────────────────────────

pub fn handle_click(
    mx: f32,
    my: f32,
    session: &SudokuSession,
    layout: &SudokuLayout,
    selected: Option<(u8, u8)>,
) -> Option<SudokuInput> {
    if session.phase == SudokuPhase::Complete {
        return None;
    }
    if layout.clear_btn.contains(mx, my) {
        if let Some((r, c)) = selected {
            return Some(SudokuInput::Action(SudokuAction::CellCleared { row: r, col: c }));
        }
        return None;
    }
    for picker in &layout.pickers {
        if picker.rect.contains(mx, my) {
            if let Some((r, c)) = selected {
                return Some(SudokuInput::Action(SudokuAction::CellPlaced {
                    row: r,
                    col: c,
                    value: picker.value,
                }));
            }
            return None;
        }
    }
    for (r, row) in layout.cells.iter().enumerate() {
        for (c, rect) in row.iter().enumerate() {
            if rect.contains(mx, my) {
                let is_given = session.puzzle.givens[r][c].is_some();
                if is_given {
                    return Some(SudokuInput::Deselect);
                }
                return Some(SudokuInput::SelectCell(r as u8, c as u8));
            }
        }
    }
    Some(SudokuInput::Deselect)
}

pub fn handle_key(
    session: &SudokuSession,
    input: &FrameInput,
    selected: Option<(u8, u8)>,
) -> Option<SudokuInput> {
    if session.phase == SudokuPhase::Complete {
        return None;
    }
    let n = session.puzzle.grid_size as usize;
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5, KeyCode::Key6];
    for (i, key) in keys.iter().take(n).enumerate() {
        if input.pressed(*key) {
            if let Some((r, c)) = selected {
                return Some(SudokuInput::Action(SudokuAction::CellPlaced {
                    row: r,
                    col: c,
                    value: (i + 1) as u8,
                }));
            }
        }
    }
    None
}

// ─── Drawing ────────────────────────────────────────────

const DARK_BG: Color = Color::new(0.078, 0.078, 0.180, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const WIN_GREEN: Color = Color::new(0.412, 0.941, 0.682, 1.0);
const CELL_BG: Color = Color::new(0.93, 0.94, 0.97, 1.0);
const GIVEN_BG: Color = Color::new(0.82, 0.85, 0.92, 1.0);
const PICKER_BG: Color = Color::new(0.129, 0.588, 0.953, 1.0);
const CLEAR_BG: Color = Color::new(0.329, 0.431, 0.478, 1.0);
const VIOLATION_TINT: Color = Color::new(1.0, 0.25, 0.25, 0.5);

/// Distinct hue per symbol — used to color the picture-mode shapes and the
/// picker tokens so a value reads at a glance.
fn token_color(value: u8) -> Color {
    match value {
        1 => Color::new(0.90, 0.30, 0.30, 1.0),
        2 => Color::new(0.30, 0.55, 0.95, 1.0),
        3 => Color::new(0.35, 0.78, 0.45, 1.0),
        4 => Color::new(0.95, 0.78, 0.25, 1.0),
        5 => Color::new(0.70, 0.45, 0.90, 1.0),
        _ => Color::new(0.40, 0.80, 0.80, 1.0),
    }
}

/// Draw a value as a picture-mode shape (circle/square/triangle/diamond/…)
/// centered in `rect`. Pre-readers match shapes instead of digits.
fn draw_shape(rect: UiRect, value: u8) {
    let cx = rect.x + rect.w / 2.0;
    let cy = rect.y + rect.h / 2.0;
    let s = rect.w.min(rect.h) * 0.30;
    let col = token_color(value);
    match value {
        1 => draw_circle(cx, cy, s, col),
        2 => draw_rectangle(cx - s, cy - s, s * 2.0, s * 2.0, col),
        3 => draw_triangle(
            Vec2::new(cx, cy - s),
            Vec2::new(cx - s, cy + s),
            Vec2::new(cx + s, cy + s),
            col,
        ),
        4 => draw_poly(cx, cy, 4, s * 1.2, 0.0, col), // diamond (rotated square)
        5 => draw_poly(cx, cy, 5, s * 1.2, 90.0, col),
        _ => draw_poly(cx, cy, 6, s * 1.2, 0.0, col),
    }
}

fn draw_digit(rect: UiRect, value: u8, color: Color) {
    let text = format!("{value}");
    let size = (rect.w * 0.55) as u16;
    let tw = measure_text(&text, None, size, 1.0).width;
    draw_text(
        &text,
        rect.x + rect.w / 2.0 - tw / 2.0,
        rect.y + rect.h / 2.0 + size as f32 * 0.35,
        size as f32,
        color,
    );
}

fn draw_token(rect: UiRect, value: u8, picture: bool, given: bool) {
    if picture {
        draw_shape(rect, value);
    } else {
        let color = if given { Color::new(0.20, 0.20, 0.20, 1.0) } else { Color::new(0.06, 0.30, 0.55, 1.0) };
        draw_digit(rect, value, color);
    }
}

fn violation_cells(session: &SudokuSession, selected: Option<(u8, u8)>) -> Vec<(u8, u8)> {
    let mut out = Vec::new();
    let placed = match selected {
        Some(p) => p,
        None => return out,
    };
    match session.last_violation {
        Some(SudokuValidation::RowConflict { col }) => {
            out.push(placed);
            out.push((placed.0, col));
        }
        Some(SudokuValidation::ColConflict { row }) => {
            out.push(placed);
            out.push((row, placed.1));
        }
        Some(SudokuValidation::BoxConflict { row, col }) => {
            out.push(placed);
            out.push((row, col));
        }
        _ => {}
    }
    out
}

pub fn draw_sudoku(session: &SudokuSession, layout: &SudokuLayout, selected: Option<(u8, u8)>) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));

    let p = layout.panel;
    draw_rectangle(p.x, p.y, p.w, p.h, DARK_BG);
    draw_rectangle_lines(p.x, p.y, p.w, p.h, 4.0, GOLD);

    let solved = session.phase == SudokuPhase::Complete;
    let header = if solved { "PUZZLE SOLVED!" } else { "Mini Sudoku" };
    let header_color = if solved { WIN_GREEN } else { GOLD };
    let hw = measure_text(header, None, 28, 1.0).width;
    draw_text(header, p.x + p.w / 2.0 - hw / 2.0, p.y + 36.0, 28.0, header_color);

    let n = session.puzzle.grid_size as usize;
    let picture = session.puzzle.grid_size == 4;
    let violations = violation_cells(session, selected);

    for r in 0..n {
        for c in 0..n {
            let rect = layout.cells[r][c];
            let given = session.puzzle.givens[r][c].is_some();
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, if given { GIVEN_BG } else { CELL_BG });
            if violations.iter().any(|&(vr, vc)| vr == r as u8 && vc == c as u8) {
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, VIOLATION_TINT);
            }
            if let Some(v) = session.grid[r][c] {
                draw_token(rect, v, picture, given);
            }
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, Color::new(0.0, 0.0, 0.0, 0.3));
        }
    }

    // Thick box borders.
    let br = session.puzzle.box_rows as usize;
    let bc = session.puzzle.box_cols as usize;
    for r in 0..n {
        for c in 0..n {
            let rect = layout.cells[r][c];
            if (c + 1) % bc == 0 && c + 1 < n {
                draw_line(rect.x + rect.w, rect.y, rect.x + rect.w, rect.y + rect.h, 3.0, BLACK);
            }
            if (r + 1) % br == 0 && r + 1 < n {
                draw_line(rect.x, rect.y + rect.h, rect.x + rect.w, rect.y + rect.h, 3.0, BLACK);
            }
        }
    }
    let g0 = layout.cells[0][0];
    let g_last = layout.cells[n - 1][n - 1];
    draw_rectangle_lines(g0.x, g0.y, g_last.x + g_last.w - g0.x, g_last.y + g_last.h - g0.y, 4.0, BLACK);

    if let Some((sr, sc)) = selected {
        let rect = layout.cells[sr as usize][sc as usize];
        draw_rectangle_lines(rect.x + 2.0, rect.y + 2.0, rect.w - 4.0, rect.h - 4.0, 4.0, GOLD);
    }

    if solved {
        let dismiss = "Tap or press SPACE to continue";
        let dw = measure_text(dismiss, None, 24, 1.0).width;
        if (get_time() * 4.0).sin() > 0.0 {
            draw_text(dismiss, p.x + p.w / 2.0 - dw / 2.0, layout.clear_btn.y + 28.0, 24.0, GOLD);
        }
    } else {
        for picker in &layout.pickers {
            let rr = picker.rect;
            draw_rectangle(rr.x, rr.y, rr.w, rr.h, PICKER_BG);
            draw_rectangle_lines(rr.x, rr.y, rr.w, rr.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
            if picture {
                draw_shape(rr, picker.value);
            } else {
                draw_digit(rr, picker.value, WHITE);
            }
        }
        let cb = layout.clear_btn;
        draw_rectangle(cb.x, cb.y, cb.w, cb.h, CLEAR_BG);
        draw_rectangle_lines(cb.x, cb.y, cb.w, cb.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.3));
        let label = "Clear";
        let tw = measure_text(label, None, 22, 1.0).width;
        draw_text(label, cb.x + cb.w / 2.0 - tw / 2.0, cb.y + cb.h / 2.0 + 8.0, 22.0, WHITE);
    }
}
