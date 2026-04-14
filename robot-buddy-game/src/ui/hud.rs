use macroquad::prelude::*;
use robot_buddy_domain::learning::learner_profile::LearnerProfile;
use robot_buddy_domain::types::Operation;

// ─── AREA NAME ──────────────────────────────────────────

struct AreaRect {
    name: &'static str,
    x1: usize, y1: usize, x2: usize, y2: usize,
}

const OVERWORLD_AREAS: &[AreaRect] = &[
    AreaRect { name: "Home",          x1: 3, y1: 3, x2: 8, y2: 9 },
    AreaRect { name: "Main Path",     x1: 9, y1: 1, x2: 18, y2: 10 },
    AreaRect { name: "Pond",          x1: 14, y1: 11, x2: 22, y2: 15 },
    AreaRect { name: "East House",    x1: 20, y1: 3, x2: 26, y2: 7 },
    AreaRect { name: "South House",   x1: 22, y1: 15, x2: 27, y2: 19 },
    AreaRect { name: "Forest Edge",   x1: 1, y1: 10, x2: 5, y2: 22 },
    AreaRect { name: "South Meadow",  x1: 6, y1: 15, x2: 14, y2: 22 },
    AreaRect { name: "Treasure Woods", x1: 22, y1: 8, x2: 28, y2: 14 },
];

pub fn get_area_name(map_id: &str, tx: usize, ty: usize) -> &'static str {
    match map_id {
        "overworld" => {
            for a in OVERWORLD_AREAS {
                if tx >= a.x1 && tx <= a.x2 && ty >= a.y1 && ty <= a.y2 {
                    return a.name;
                }
            }
            "The Wild"
        }
        "home" => "Home (Inside)",
        "lab" => "Gizmo's Lab",
        "shop" => "Dum Dum Shop",
        "dream" => "The Dream",
        "doghouse" => "D0GH0USE.exe",
        "grove" => "Hidden Grove",
        _ => "???",
    }
}

pub fn draw_area_name(map_id: &str, tx: usize, ty: usize) {
    let name = get_area_name(map_id, tx, ty);
    let tw = measure_text(name, None, 20, 1.0).width;
    let pill_w = tw + 30.0;
    let pill_h = 32.0;
    let x = 12.0;
    let y = 10.0;

    draw_rectangle(x, y, pill_w, pill_h, Color::new(0.078, 0.078, 0.157, 0.7));
    draw_text(name, x + 12.0, y + 22.0, 20.0, Color::from_rgba(144, 202, 249, 255));
}

// ─── DUM DUM COUNTER ────────────────────────────────────

pub struct DumDumHud {
    flash_timer: f32,
}

impl DumDumHud {
    pub fn new() -> Self {
        DumDumHud { flash_timer: 0.0 }
    }

    pub fn flash(&mut self) {
        self.flash_timer = 0.5;
    }

    pub fn update(&mut self, dt: f32) {
        if self.flash_timer > 0.0 {
            self.flash_timer -= dt;
        }
    }

    pub fn draw(&self, count: u32) {
        let sw = screen_width();
        let bg_w = 80.0;
        let bg_h = 44.0;
        let x = sw - bg_w - 12.0;
        let y = 10.0;

        // Scale on flash
        let scale = if self.flash_timer > 0.0 {
            1.0 + 0.3 * (self.flash_timer / 0.5)
        } else {
            1.0
        };

        let cx = x + bg_w / 2.0;
        let cy = y + bg_h / 2.0;
        let sx = cx - (bg_w * scale) / 2.0;
        let sy = cy - (bg_h * scale) / 2.0;

        draw_rectangle(sx, sy, bg_w * scale, bg_h * scale,
            Color::new(0.078, 0.078, 0.157, 0.8));

        // Lollipop icon
        let icon_x = sx + 18.0;
        let icon_y = sy + bg_h * scale / 2.0;
        // Stick
        draw_line(icon_x, icon_y + 4.0, icon_x, icon_y + 16.0, 2.0,
            Color::from_rgba(224, 224, 224, 255));
        // Candy ball
        draw_circle(icon_x, icon_y + 2.0, 7.0, Color::from_rgba(255, 82, 82, 255));
        // Swirl highlight
        draw_circle(icon_x - 2.0, icon_y, 2.0, Color::from_rgba(255, 205, 210, 200));

        // Count
        let text = format!("x{}", count);
        let tw = measure_text(&text, None, 22, 1.0).width;
        draw_text(&text, sx + bg_w * scale - tw - 10.0, sy + bg_h * scale / 2.0 + 8.0,
            22.0, Color::from_rgba(255, 82, 82, 255));
    }
}

// ─── PARENT DEBUG OVERLAY (P key) ───────────────────────

pub struct DebugOverlay {
    pub visible: bool,
}

impl DebugOverlay {
    pub fn new() -> Self {
        DebugOverlay { visible: false }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Draw the debug overlay. Returns true if the Export button was clicked.
    pub fn draw(&self, map_id: &str, tx: usize, ty: usize, dum_dums: u32, play_time: f32,
                profile: &LearnerProfile, challenges: usize, correct: usize) -> bool {
        if !self.visible { return false; }

        let sw = screen_width();
        let panel_w = 380.0;
        let panel_h = 620.0;
        let x = sw - panel_w - 10.0;
        let y = 10.0;

        // Background
        draw_rectangle(x, y, panel_w, panel_h, Color::new(0.0, 0.0, 0.0, 0.88));
        draw_rectangle_lines(x, y, panel_w, panel_h, 2.0, Color::from_rgba(0, 230, 118, 255));

        let green = Color::from_rgba(0, 230, 118, 255);
        let white = Color::from_rgba(210, 210, 210, 255);
        let gold = Color::from_rgba(255, 213, 79, 255);
        let cyan = Color::from_rgba(100, 200, 255, 255);
        let mut ly = y + 26.0;
        let lx = x + 14.0;
        let line_h = 22.0;

        draw_text("PARENT DEBUG", lx, ly, 20.0, green);
        ly += line_h + 4.0;

        draw_text(&format!("Map: {}  Tile: ({}, {})", map_id, tx, ty), lx, ly, 16.0, white);
        ly += line_h;

        draw_text(&format!("Area: {}", get_area_name(map_id, tx, ty)), lx, ly, 16.0, white);
        ly += line_h;

        draw_text(&format!("Dum Dums: {}", dum_dums), lx, ly, 16.0, gold);
        ly += line_h;

        let mins = (play_time as u64) / 60;
        let secs = (play_time as u64) % 60;
        draw_text(&format!("Play time: {}m {}s", mins, secs), lx, ly, 16.0, white);
        ly += line_h;

        // -- Adaptive learning profile --
        draw_text("-- Learning Profile --", lx, ly, 16.0, green);
        ly += line_h;

        let band_label = super::title_screen::BAND_NAMES.get((profile.math_band - 1) as usize).unwrap_or(&"???");
        draw_text(&format!("Band: {} ({})  Streak: {}", profile.math_band, band_label, profile.streak), lx, ly, 16.0, white);
        ly += line_h;

        draw_text(&format!("Pace: {:.2}  Scaffolding: {:.2}", profile.pace, profile.scaffolding), lx, ly, 16.0, white);
        ly += line_h;

        draw_text(&format!("Spread: {:.2}  Window: {}/20", profile.spread_width,
            profile.rolling_window.entries.len()), lx, ly, 16.0, white);
        ly += line_h;

        draw_text(&format!("Intake: {}  Teach after: {} wrong",
            if profile.intake_completed { "done" } else { "pending" },
            profile.wrongs_before_teach), lx, ly, 16.0, white);
        ly += line_h;

        // CRA per operation
        draw_text("CRA stages:", lx, ly, 16.0, cyan);
        ly += line_h;
        for (op, cra) in &profile.cra_stages {
            draw_text(&format!("  {:?}: {:?}", op, cra), lx, ly, 14.0, white);
            ly += 18.0;
        }
        ly += 4.0;

        // Per-operation accuracy (from rolling window)
        draw_text("-- Operation Stats --", lx, ly, 16.0, green);
        ly += line_h;
        for op in [Operation::Add, Operation::Sub, Operation::Multiply, Operation::Divide, Operation::NumberBond] {
            let stats = profile.operation_stats.get_coarse(op);
            let window_acc = profile.rolling_window.operation_accuracy(op);
            let acc_str = match window_acc {
                Some(a) => format!("{:.0}%", a * 100.0),
                None => "—".into(),
            };
            let lifetime_str = match stats.accuracy() {
                Some(a) => format!("{:.0}% ({}/{})", a * 100.0, stats.correct, stats.attempts),
                None => "—".into(),
            };
            draw_text(&format!("  {:?}: recent {} | total {}", op, acc_str, lifetime_str),
                lx, ly, 14.0, white);
            ly += 18.0;
        }
        ly += 4.0;

        // Response time + consecutive wrong
        let avg_rt = profile.rolling_window.avg_response_time();
        let rt_str = if avg_rt > 0.0 { format!("{:.0}ms", avg_rt) } else { "—".into() };
        let consec = profile.rolling_window.consecutive_wrong();
        draw_text(&format!("Avg response: {}  Wrong streak: {}", rt_str, consec),
            lx, ly, 16.0, white);
        ly += line_h + 4.0;

        let accuracy = if challenges > 0 {
            format!("{:.0}%", correct as f64 / challenges as f64 * 100.0)
        } else {
            "n/a".into()
        };
        draw_text(&format!("Session: {}/{} correct ({})", correct, challenges, accuracy),
            lx, ly, 16.0, white);
        ly += line_h + 8.0;

        // Export button
        let btn_w = 130.0;
        let btn_h = 28.0;
        let btn_x = lx;
        let btn_y = ly;
        let (mx, my) = mouse_position();
        let hover = mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
        let btn_color = if hover {
            Color::from_rgba(0, 200, 100, 255)
        } else {
            Color::from_rgba(0, 160, 80, 255)
        };
        draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
        let etw = measure_text("Export Session", None, 16, 1.0).width;
        draw_text("Export Session", btn_x + btn_w / 2.0 - etw / 2.0, btn_y + 19.0, 16.0, WHITE);

        let clicked = is_mouse_button_pressed(MouseButton::Left) && hover;
        ly += btn_h + 10.0;

        draw_text("P close  |  E export", lx, ly, 14.0, Color::from_rgba(100, 100, 120, 255));

        clicked
    }
}
