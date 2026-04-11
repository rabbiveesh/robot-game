use macroquad::prelude::*;

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

    pub fn draw(&self, map_id: &str, tx: usize, ty: usize, dum_dums: u32, play_time: f32) {
        if !self.visible { return; }

        let sw = screen_width();
        let panel_w = 380.0;
        let panel_h = 300.0;
        let x = sw - panel_w - 10.0;
        let y = 10.0;

        // Background
        draw_rectangle(x, y, panel_w, panel_h, Color::new(0.0, 0.0, 0.0, 0.88));
        draw_rectangle_lines(x, y, panel_w, panel_h, 2.0, Color::from_rgba(0, 230, 118, 255));

        let green = Color::from_rgba(0, 230, 118, 255);
        let white = Color::from_rgba(210, 210, 210, 255);
        let gold = Color::from_rgba(255, 213, 79, 255);
        let mut ly = y + 26.0;
        let lx = x + 14.0;
        let line_h = 24.0;

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
        ly += line_h + 8.0;

        // Placeholder for adaptive system (not wired yet)
        draw_text("-- Learner Profile --", lx, ly, 16.0, green);
        ly += line_h;
        draw_text("Band: 1 (Add <5)  Streak: 0", lx, ly, 16.0, white);
        ly += line_h;
        draw_text("Intake: pending", lx, ly, 16.0, Color::from_rgba(255, 183, 77, 255));
        ly += line_h;
        draw_text("CRA: all concrete (placeholder)", lx, ly, 16.0, white);
        ly += line_h + 8.0;

        draw_text("P to close", lx, ly, 14.0, Color::from_rgba(100, 100, 120, 255));
    }
}
