use macroquad::prelude::*;
use robot_buddy_domain::learning::challenge_generator::Challenge;

const BLUE_A: Color = Color::new(0.259, 0.647, 0.961, 1.0);       // #42A5F5
const YELLOW_B: Color = Color::new(1.0, 0.835, 0.310, 1.0);       // #FFD54F
const RED_TAKE: Color = Color::new(0.937, 0.263, 0.212, 1.0);     // #EF5350
const RED_FAINT: Color = Color::new(0.957, 0.263, 0.212, 0.4);
const LABEL_GRAY: Color = Color::new(0.878, 0.878, 0.878, 1.0);   // #E0E0E0
const HINT_GRAY: Color = Color::new(0.667, 0.667, 0.667, 1.0);    // #AAA

/// Draw the appropriate CRA visual for a challenge.
/// Uses dots for bands 1-4, base-10 blocks for bands 5+.
pub fn draw_visual(challenge: &Challenge, cx: f32, cy: f32, _time: f32) {
    let a = challenge.numbers.a;
    let b = challenge.numbers.b;
    let op = challenge.numbers.op.as_str();
    let answer = challenge.correct_answer;
    let band = challenge.sampled_band;

    if band >= 5 {
        draw_base10_blocks(a, b, op, answer, cx, cy);
    } else {
        draw_dots(a, b, op, cx, cy);
    }
}

// ─── DOT VISUAL (bands 1-4) ────────────────────────────

fn draw_dots(a: i32, b: i32, op: &str, cx: f32, cy: f32) {
    let dot_r = 5.0;
    let gap = 4.0;
    let step = dot_r * 2.0 + gap;

    match op {
        "+" => {
            let total = a + b;
            let per_row = total.min(10);
            let start_x = cx - (per_row as f32 * step) / 2.0;

            let mut idx = 0;
            // Group A (blue)
            for _ in 0..a {
                let row = idx / 10;
                let col = idx % 10;
                let dx = start_x + col as f32 * step + dot_r;
                let dy = cy + row as f32 * (step + gap) + dot_r;
                draw_circle(dx, dy, dot_r, BLUE_A);
                idx += 1;
            }
            // Group B (yellow)
            for _ in 0..b {
                let row = idx / 10;
                let col = idx % 10;
                let dx = start_x + col as f32 * step + dot_r;
                let dy = cy + row as f32 * (step + gap) + dot_r;
                draw_circle(dx, dy, dot_r, YELLOW_B);
                idx += 1;
            }

            // Labels
            let label_y = cy + ((total - 1) / 10 + 1) as f32 * (step + gap) + 12.0;
            let a_str = format!("{}", a);
            let b_str = format!("{}", b);
            let aw = measure_text(&a_str, None, 16, 1.0).width;
            let bw = measure_text(&b_str, None, 16, 1.0).width;
            let pw = measure_text("+", None, 16, 1.0).width;
            draw_text(&a_str, cx - 40.0 - aw / 2.0, label_y, 16.0, BLUE_A);
            draw_text("+", cx - pw / 2.0, label_y, 16.0, HINT_GRAY);
            draw_text(&b_str, cx + 40.0 - bw / 2.0, label_y, 16.0, YELLOW_B);
        }
        "-" | "\u{2212}" => {
            let per_row = a.min(10);
            let start_x = cx - (per_row as f32 * step) / 2.0;

            for i in 0..a {
                let row = i / 10;
                let col = i % 10;
                let dx = start_x + col as f32 * step + dot_r;
                let dy = cy + row as f32 * (step + gap) + dot_r;

                if i >= a - b {
                    // "Taken away" dots
                    draw_circle(dx, dy, dot_r, RED_FAINT);
                    // X mark
                    draw_line(dx - 3.0, dy - 3.0, dx + 3.0, dy + 3.0, 2.0, RED_TAKE);
                    draw_line(dx + 3.0, dy - 3.0, dx - 3.0, dy + 3.0, 2.0, RED_TAKE);
                } else {
                    draw_circle(dx, dy, dot_r, BLUE_A);
                }
            }

            let label_y = cy + ((a - 1) / 10 + 1) as f32 * (step + gap) + 12.0;
            let label = format!("{} - {} = count the blue ones!", a, b);
            let lw = measure_text(&label, None, 16, 1.0).width;
            draw_text(&label, cx - lw / 2.0, label_y, 16.0, BLUE_A);
        }
        "\u{00d7}" | "*" => {
            // a groups of b dots
            let max_w = 500.0;
            let groups = a.min(8);
            let per_group = b.min(10);
            let group_gap = 30.0;
            let naive_w = groups as f32 * (per_group as f32 * step + group_gap) - group_gap;
            let scale = if naive_w > max_w { max_w / naive_w } else { 1.0 };
            let s_dot_r = dot_r * scale;
            let s_step = step * scale;
            let s_group_gap = group_gap * scale;
            let s_group_w = per_group as f32 * s_step + s_group_gap;
            let total_w = groups as f32 * s_group_w - s_group_gap;
            let start_x = cx - total_w / 2.0;

            let label = format!("{} groups of {}", a, b);
            let lw = measure_text(&label, None, 14, 1.0).width;
            draw_text(&label, cx - lw / 2.0, cy - 8.0, 14.0, HINT_GRAY);

            for g in 0..groups {
                let gx = start_x + g as f32 * s_group_w;
                let color = if g % 2 == 0 { BLUE_A } else { YELLOW_B };
                for d in 0..per_group {
                    let dx = gx + d as f32 * s_step + s_dot_r;
                    let dy = cy + 10.0 + s_dot_r;
                    draw_circle(dx, dy, s_dot_r, color);
                }
            }
        }
        "\u{00f7}" | "/" => {
            // a split into b groups of answer
            let max_w = 500.0;
            let groups = b.min(8);
            let per_group = (a / b.max(1)).min(12);
            // Scale down dot size if content would overflow
            let naive_group_w = per_group as f32 * step + 10.0;
            let naive_total = groups as f32 * naive_group_w;
            let scale = if naive_total > max_w { max_w / naive_total } else { 1.0 };
            let s_dot_r = dot_r * scale;
            let s_step = step * scale;
            let s_group_w = per_group as f32 * s_step + 10.0 * scale;
            let total_w = groups as f32 * s_group_w;
            let start_x = cx - total_w / 2.0;

            let label = format!("{} split into {} groups", a, b);
            let lw = measure_text(&label, None, 14, 1.0).width;
            draw_text(&label, cx - lw / 2.0, cy - 8.0, 14.0, HINT_GRAY);

            for g in 0..groups {
                let gx = start_x + g as f32 * s_group_w;
                // Group outline
                draw_rectangle_lines(gx, cy + 2.0, s_group_w - 4.0 * scale, s_dot_r * 2.0 + 8.0 * scale, 1.0,
                    Color::new(0.329, 0.431, 0.478, 1.0));
                let color = if g % 2 == 0 { BLUE_A } else { YELLOW_B };
                for d in 0..per_group {
                    let dx = gx + 4.0 * scale + d as f32 * s_step + s_dot_r;
                    let dy = cy + 4.0 * scale + s_dot_r + 2.0;
                    draw_circle(dx, dy, s_dot_r, color);
                }
                // Group count label
                let count_str = format!("{}", per_group);
                let font_size = (11.0 * scale).max(9.0);
                let cw = measure_text(&count_str, None, font_size as u16, 1.0).width;
                draw_text(&count_str, gx + (s_group_w - 4.0 * scale) / 2.0 - cw / 2.0,
                    cy + s_dot_r * 2.0 + 18.0 * scale + 4.0, font_size, HINT_GRAY);
            }
        }
        _ => {}
    }
}

// ─── BASE-10 BLOCKS (bands 5+) ─────────────────────────

const ROD_W: f32 = 10.0;
const ROD_H: f32 = 44.0;
const FIVE_H: f32 = 22.0;
const CUBE: f32 = 10.0;
const BLOCK_GAP: f32 = 3.0;

struct BlockColors {
    rod: Color,
    cube: Color,
    five: Color,
}

const COLORS_A: BlockColors = BlockColors {
    rod: Color::new(0.259, 0.647, 0.961, 1.0),   // #42A5F5
    cube: Color::new(0.392, 0.710, 0.965, 1.0),   // #64B5F6
    five: Color::new(0.400, 0.733, 0.416, 1.0),   // #66BB6A
};

const COLORS_B: BlockColors = BlockColors {
    rod: Color::new(1.0, 0.835, 0.310, 1.0),      // #FFD54F
    cube: Color::new(1.0, 0.878, 0.510, 1.0),     // #FFE082
    five: Color::new(0.506, 0.780, 0.518, 1.0),   // #81C784
};

const COLORS_RED: BlockColors = BlockColors {
    rod: Color::new(0.937, 0.263, 0.212, 1.0),    // #EF5350
    cube: Color::new(0.937, 0.604, 0.604, 1.0),   // #EF9A9A
    five: Color::new(0.898, 0.451, 0.451, 1.0),   // #E57373
};

fn measure_num(num: i32) -> f32 {
    let tens = num / 10;
    let ones = num % 10;
    let fives = ones / 5;
    let remainder = ones % 5;
    let rods_w = if tens > 0 { tens as f32 * (ROD_W + BLOCK_GAP) } else { 0.0 };
    let ones_w = fives as f32 * (ROD_W + BLOCK_GAP) + remainder as f32 * (CUBE + BLOCK_GAP);
    rods_w.max(ones_w).max(20.0)
}

fn content_height(num: i32) -> f32 {
    let tens = num / 10;
    let ones = num % 10;
    let fives = ones / 5;
    let ones_h = if fives > 0 { FIVE_H } else if ones > 0 { CUBE } else { 0.0 };
    if tens > 0 { ROD_H + 5.0 + ones_h } else { ones_h.max(CUBE) }
}

fn draw_num_blocks(x: f32, y: f32, num: i32, colors: &BlockColors) {
    let tens = num / 10;
    let ones = num % 10;
    let fives = ones / 5;
    let remainder = ones % 5;
    let total_w = measure_num(num);
    let outline = Color::new(0.0, 0.0, 0.0, 0.3);

    // Label
    let label = format!("{}", num);
    let lw = measure_text(&label, None, 16, 1.0).width;
    draw_text(&label, x + total_w / 2.0 - lw / 2.0, y - 6.0, 16.0, LABEL_GRAY);

    // Tens rods
    for i in 0..tens.min(15) {
        let rx = x + i as f32 * (ROD_W + BLOCK_GAP);
        draw_rectangle(rx, y, ROD_W, ROD_H, colors.rod);
        draw_rectangle_lines(rx, y, ROD_W, ROD_H, 1.0, outline);
    }

    // Ones row
    let ones_y = if tens > 0 { y + ROD_H + 5.0 } else { y };
    let mut ones_x = x;

    // 5-bars
    for _ in 0..fives {
        draw_rectangle(ones_x, ones_y, ROD_W, FIVE_H, colors.five);
        draw_rectangle_lines(ones_x, ones_y, ROD_W, FIVE_H, 1.0, outline);
        ones_x += ROD_W + BLOCK_GAP;
    }

    // Remainder cubes
    for _ in 0..remainder {
        let cube_y = if fives > 0 { ones_y + (FIVE_H - CUBE) / 2.0 } else { ones_y };
        draw_rectangle(ones_x, cube_y, CUBE, CUBE, colors.cube);
        draw_rectangle_lines(ones_x, cube_y, CUBE, CUBE, 1.0, outline);
        ones_x += CUBE + BLOCK_GAP;
    }
}

fn draw_base10_blocks(a: i32, b: i32, op: &str, answer: i32, cx: f32, cy: f32) {
    match op {
        "+" => {
            let wa = measure_num(a);
            let wb = measure_num(b);
            let op_gap = 40.0;
            let total_w = wa + op_gap + wb;
            let start_x = cx - total_w / 2.0;

            draw_num_blocks(start_x, cy, a, &COLORS_A);
            draw_op_symbol(start_x + wa + op_gap / 2.0, cy, "+", a, b);
            draw_num_blocks(start_x + wa + op_gap, cy, b, &COLORS_B);
        }
        "-" | "\u{2212}" => {
            let wa = measure_num(a);
            let wb = measure_num(b);
            let op_gap = 40.0;
            let total_w = wa + op_gap + wb;
            let start_x = cx - total_w / 2.0;

            draw_num_blocks(start_x, cy, a, &COLORS_A);
            draw_op_symbol(start_x + wa + op_gap / 2.0, cy, "\u{2212}", a, b);
            draw_num_blocks(start_x + wa + op_gap, cy, b, &COLORS_RED);
        }
        "\u{00d7}" | "*" => {
            // Array: rows × cols dots
            let rows = a.min(b).min(12) as i32;
            let cols = a.max(b).min(12) as i32;
            let dot_r = 5.0;
            let dot_gap = 4.0;
            let step = dot_r * 2.0 + dot_gap;
            let grid_w = cols as f32 * step;
            let start_x = cx - grid_w / 2.0;

            let label = format!("{} rows of {}", rows, cols);
            let lw = measure_text(&label, None, 14, 1.0).width;
            draw_text(&label, cx - lw / 2.0, cy - 8.0, 14.0, HINT_GRAY);

            for r in 0..rows {
                for c in 0..cols {
                    let color = if r % 2 == 0 { BLUE_A } else { YELLOW_B };
                    let dx = start_x + c as f32 * step + dot_r;
                    let dy = cy + 5.0 + r as f32 * step + dot_r;
                    draw_circle(dx, dy, dot_r, color);
                }
            }
        }
        "\u{00f7}" | "/" => {
            let max_w = 500.0;
            let groups = b.min(8);
            let per_group = answer.min(12);
            let dot_r = 5.0;
            let dot_gap = 3.0;
            let step = dot_r * 2.0 + dot_gap;
            let naive_group_w = per_group as f32 * step + 10.0;
            let naive_total = groups as f32 * naive_group_w;
            let scale = if naive_total > max_w { max_w / naive_total } else { 1.0 };
            let s_dot_r = dot_r * scale;
            let s_step = step * scale;
            let s_group_w = per_group as f32 * s_step + 10.0 * scale;
            let total_w = groups as f32 * s_group_w;
            let start_x = cx - total_w / 2.0;

            let label = format!("{} split into {} groups", a, b);
            let lw = measure_text(&label, None, 14, 1.0).width;
            draw_text(&label, cx - lw / 2.0, cy - 8.0, 14.0, HINT_GRAY);

            for g in 0..groups {
                let gx = start_x + g as f32 * s_group_w;
                draw_rectangle_lines(gx, cy + 2.0, s_group_w - 4.0 * scale, s_dot_r * 2.0 + 8.0 * scale, 1.0,
                    Color::new(0.329, 0.431, 0.478, 1.0));
                let color = if g % 2 == 0 { BLUE_A } else { YELLOW_B };
                for d in 0..per_group {
                    let dx = gx + 4.0 * scale + d as f32 * s_step + s_dot_r;
                    let dy = cy + 4.0 * scale + s_dot_r + 2.0;
                    draw_circle(dx, dy, s_dot_r, color);
                }
                let count_str = format!("{}", per_group);
                let font_size = (11.0 * scale).max(9.0);
                let cw = measure_text(&count_str, None, font_size as u16, 1.0).width;
                draw_text(&count_str, gx + (s_group_w - 4.0 * scale) / 2.0 - cw / 2.0,
                    cy + s_dot_r * 2.0 + 18.0 * scale + 4.0, font_size, HINT_GRAY);
            }
        }
        _ => {}
    }
}

fn draw_op_symbol(x: f32, y: f32, symbol: &str, num_a: i32, num_b: i32) {
    let h = content_height(num_a).max(content_height(num_b));
    let cy = y + h / 2.0 + 4.0;
    // Draw minus as a line since the default font may lack U+2212
    if symbol == "\u{2212}" || symbol == "-" {
        let half_w = 8.0;
        draw_line(x - half_w, cy - 4.0, x + half_w, cy - 4.0, 3.0, WHITE);
    } else {
        let sw = measure_text(symbol, None, 28, 1.0).width;
        draw_text(symbol, x - sw / 2.0, cy, 28.0, WHITE);
    }
}
