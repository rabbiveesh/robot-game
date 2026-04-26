//! KenKen puzzle UI.
//!
//! Single entrypoint surface for the game-side glue:
//!   - `layout(session, screen)` — pure, hit-testable rectangles
//!   - `draw_kenken(session, layout, time, selected)` — render
//!   - `handle_click(mx, my, session, layout, selected)` → KenKenInput
//!   - `handle_key(session, input, selected)` → KenKenInput
//!
//! `KenKenInput` is the only type the game.rs glue cares about — it's either
//! a domain `KenKenAction` to dispatch, or a UI-only selection change.

use macroquad::prelude::*;
use robot_buddy_domain::logic::kenken::{
    KenKenAction, KenKenPhase, KenKenSession, ValidationResult,
};

use crate::input::FrameInput;

// ─── Layout types ───────────────────────────────────────

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
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

pub struct PickerBound {
    pub rect: UiRect,
    pub value: u8,
}

pub struct KenKenLayout {
    pub panel: UiRect,
    pub cells: Vec<Vec<UiRect>>,
    pub pickers: Vec<PickerBound>,
    pub hint_btn: UiRect,
    pub clear_btn: UiRect,
}

/// Outcome of a UI input event. Either a domain action to dispatch, or a
/// purely visual selection update. Keeps the domain reducer untainted by UI
/// concerns like "which cell is highlighted".
pub enum KenKenInput {
    Action(KenKenAction),
    SelectCell(u8, u8),
    Deselect,
}

// ─── Layout (pure) ──────────────────────────────────────

pub fn layout(session: &KenKenSession, screen: (f32, f32)) -> KenKenLayout {
    let (sw, sh) = screen;
    let panel_w = (sw - 40.0).min(720.0);
    let panel_h = (sh - 40.0).min(640.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;

    let n = session.puzzle.grid_size as usize;
    let grid_max = panel_w.min(panel_h - 220.0).min(440.0);
    let cell_size = (grid_max / n as f32).floor();
    let grid_size_px = cell_size * n as f32;
    let grid_x = panel_x + (panel_w - grid_size_px) / 2.0;
    let grid_y = panel_y + 60.0;

    let mut cells: Vec<Vec<UiRect>> = Vec::with_capacity(n);
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

    let pickers_y = grid_y + grid_size_px + 24.0;
    let picker_size: f32 = 60.0;
    let picker_gap: f32 = 12.0;
    let total_picker_w = picker_size * n as f32 + picker_gap * (n.saturating_sub(1)) as f32;
    let picker_start_x = panel_x + (panel_w - total_picker_w) / 2.0;
    let mut pickers: Vec<PickerBound> = Vec::with_capacity(n);
    for i in 0..n {
        pickers.push(PickerBound {
            rect: UiRect {
                x: picker_start_x + i as f32 * (picker_size + picker_gap),
                y: pickers_y,
                w: picker_size,
                h: picker_size,
            },
            value: (i + 1) as u8,
        });
    }

    let btn_w = 110.0;
    let btn_h = 40.0;
    let btn_y = pickers_y + picker_size + 16.0;
    let btn_gap = 16.0;
    let btn_start_x = panel_x + (panel_w - (btn_w * 2.0 + btn_gap)) / 2.0;
    let hint_btn = UiRect { x: btn_start_x, y: btn_y, w: btn_w, h: btn_h };
    let clear_btn = UiRect { x: btn_start_x + btn_w + btn_gap, y: btn_y, w: btn_w, h: btn_h };

    KenKenLayout {
        panel: UiRect { x: panel_x, y: panel_y, w: panel_w, h: panel_h },
        cells,
        pickers,
        hint_btn,
        clear_btn,
    }
}

// ─── Input handling ─────────────────────────────────────

pub fn handle_click(
    mx: f32,
    my: f32,
    session: &KenKenSession,
    layout: &KenKenLayout,
    selected: Option<(u8, u8)>,
) -> Option<KenKenInput> {
    if session.phase == KenKenPhase::Complete {
        return None;
    }

    if layout.hint_btn.contains(mx, my) {
        return Some(KenKenInput::Action(KenKenAction::RequestHint));
    }
    if layout.clear_btn.contains(mx, my) {
        if let Some((r, c)) = selected {
            return Some(KenKenInput::Action(KenKenAction::CellCleared { row: r, col: c }));
        }
        return None;
    }
    for picker in &layout.pickers {
        if picker.rect.contains(mx, my) {
            if let Some((r, c)) = selected {
                return Some(KenKenInput::Action(KenKenAction::CellPlaced {
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
                let is_given = session
                    .puzzle
                    .cages
                    .iter()
                    .any(|cg| cg.cells.len() == 1 && cg.cells[0] == (r as u8, c as u8));
                if is_given {
                    return Some(KenKenInput::Deselect);
                }
                return Some(KenKenInput::SelectCell(r as u8, c as u8));
            }
        }
    }
    Some(KenKenInput::Deselect)
}

pub fn handle_key(
    session: &KenKenSession,
    input: &FrameInput,
    selected: Option<(u8, u8)>,
) -> Option<KenKenInput> {
    if session.phase == KenKenPhase::Complete {
        return None;
    }
    let n = session.puzzle.grid_size;
    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
    for (i, key) in keys.iter().take(n as usize).enumerate() {
        if input.pressed(*key) {
            if let Some((r, c)) = selected {
                return Some(KenKenInput::Action(KenKenAction::CellPlaced {
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
const BLUE_BTN: Color = Color::new(0.129, 0.588, 0.953, 1.0);
const SCAFFOLD_BG: Color = Color::new(0.329, 0.431, 0.478, 1.0);
const HINT_GRAY: Color = Color::new(0.471, 0.565, 0.604, 1.0);
const GIVEN_TEXT: Color = Color::new(0.20, 0.20, 0.20, 1.0);
const USER_TEXT: Color = Color::new(0.06, 0.30, 0.55, 1.0);
const VIOLATION_TINT: Color = Color::new(1.0, 0.4, 0.4, 0.45);

const CAGE_TINTS: &[Color] = &[
    Color::new(0.95, 0.93, 0.85, 1.0),
    Color::new(0.85, 0.93, 0.95, 1.0),
    Color::new(0.92, 0.85, 0.93, 1.0),
    Color::new(0.85, 0.95, 0.87, 1.0),
    Color::new(0.95, 0.87, 0.85, 1.0),
    Color::new(0.85, 0.88, 0.95, 1.0),
    Color::new(0.93, 0.95, 0.85, 1.0),
    Color::new(0.88, 0.85, 0.95, 1.0),
];

fn cage_color(idx: usize) -> Color {
    CAGE_TINTS[idx % CAGE_TINTS.len()]
}

pub fn draw_kenken(
    session: &KenKenSession,
    layout: &KenKenLayout,
    _time: f32,
    selected: Option<(u8, u8)>,
) {
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));

    let p = layout.panel;
    draw_rectangle(p.x, p.y, p.w, p.h, DARK_BG);
    draw_rectangle_lines(p.x, p.y, p.w, p.h, 4.0, GOLD);

    let header = if session.phase == KenKenPhase::Complete {
        "PUZZLE SOLVED!"
    } else {
        "KenKen"
    };
    let header_color = if session.phase == KenKenPhase::Complete { WIN_GREEN } else { GOLD };
    let hw = measure_text(header, None, 30, 1.0).width;
    draw_text(header, p.x + p.w / 2.0 - hw / 2.0, p.y + 38.0, 30.0, header_color);

    let n = session.puzzle.grid_size as usize;

    let mut cell_cage: Vec<Vec<usize>> = vec![vec![0; n]; n];
    for (idx, cage) in session.puzzle.cages.iter().enumerate() {
        for &(r, c) in &cage.cells {
            cell_cage[r as usize][c as usize] = idx;
        }
    }

    let violation_cells = violation_cells(session, selected);

    for r in 0..n {
        for c in 0..n {
            let rect = layout.cells[r][c];
            let cage_idx = cell_cage[r][c];
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, cage_color(cage_idx));

            if violation_cells.iter().any(|&(vr, vc)| vr == r as u8 && vc == c as u8) {
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, VIOLATION_TINT);
            }

            let cage = &session.puzzle.cages[cage_idx];
            let first_cell = cage.cells.iter().min().unwrap();
            if (r as u8, c as u8) == *first_cell {
                draw_text(
                    &cage.display_label,
                    rect.x + 4.0,
                    rect.y + 16.0,
                    16.0,
                    Color::new(0.0, 0.0, 0.0, 0.75),
                );
            }

            if let Some(v) = session.grid[r][c] {
                let is_given = cage.cells.len() == 1;
                let color = if is_given { GIVEN_TEXT } else { USER_TEXT };
                let text = format!("{}", v);
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
        }
    }

    // Cage borders — thick line where adjacent cells are in different cages.
    for r in 0..n {
        for c in 0..n {
            let rect = layout.cells[r][c];
            let my_cage = cell_cage[r][c];
            if c + 1 < n && cell_cage[r][c + 1] != my_cage {
                draw_line(rect.x + rect.w, rect.y, rect.x + rect.w, rect.y + rect.h, 3.0, BLACK);
            }
            if r + 1 < n && cell_cage[r + 1][c] != my_cage {
                draw_line(rect.x, rect.y + rect.h, rect.x + rect.w, rect.y + rect.h, 3.0, BLACK);
            }
        }
    }

    // Outer + thin grid lines
    let g0 = layout.cells[0][0];
    let g_last = layout.cells[n - 1][n - 1];
    let grid_w = g_last.x + g_last.w - g0.x;
    let grid_h = g_last.y + g_last.h - g0.y;
    draw_rectangle_lines(g0.x, g0.y, grid_w, grid_h, 4.0, BLACK);
    for r in 0..n {
        for c in 0..n {
            let rect = layout.cells[r][c];
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, Color::new(0.0, 0.0, 0.0, 0.3));
        }
    }

    if let Some((sr, sc)) = selected {
        let rect = layout.cells[sr as usize][sc as usize];
        draw_rectangle_lines(rect.x + 2.0, rect.y + 2.0, rect.w - 4.0, rect.h - 4.0, 4.0, GOLD);
    }

    for picker in &layout.pickers {
        let r = picker.rect;
        draw_rectangle(r.x, r.y, r.w, r.h, BLUE_BTN);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.4));
        let label = format!("{}", picker.value);
        let size = 32u16;
        let tw = measure_text(&label, None, size, 1.0).width;
        draw_text(&label, r.x + r.w / 2.0 - tw / 2.0, r.y + r.h / 2.0 + size as f32 * 0.35, size as f32, WHITE);
    }

    draw_button(layout.hint_btn, "Hint");
    draw_button(layout.clear_btn, "Clear");

    if session.phase == KenKenPhase::Complete {
        let dismiss = "Press SPACE to continue";
        let dw = measure_text(dismiss, None, 22, 1.0).width;
        let blink = (get_time() * 4.0).sin() > 0.0;
        if blink {
            draw_text(dismiss, p.x + p.w / 2.0 - dw / 2.0, p.y + p.h - 16.0, 22.0, HINT_GRAY);
        }
    }
}

fn draw_button(rect: UiRect, label: &str) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, SCAFFOLD_BG);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, Color::new(1.0, 1.0, 1.0, 0.3));
    let size = 22u16;
    let tw = measure_text(label, None, size, 1.0).width;
    draw_text(
        label,
        rect.x + rect.w / 2.0 - tw / 2.0,
        rect.y + rect.h / 2.0 + 8.0,
        size as f32,
        WHITE,
    );
}

fn violation_cells(session: &KenKenSession, selected: Option<(u8, u8)>) -> Vec<(u8, u8)> {
    let mut out = Vec::new();
    let placed = match selected {
        Some(p) => p,
        None => return out,
    };
    match session.last_violation {
        Some(ValidationResult::RowConflict { col }) => {
            out.push(placed);
            out.push((placed.0, col));
        }
        Some(ValidationResult::ColConflict { row }) => {
            out.push(placed);
            out.push((row, placed.1));
        }
        Some(ValidationResult::CageWrong) => {
            out.push(placed);
        }
        _ => {}
    }
    out
}
