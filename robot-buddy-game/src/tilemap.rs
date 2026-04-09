use macroquad::prelude::*;

pub const TILE_SIZE: f32 = 48.0;

#[derive(Clone)]
pub struct Map {
    pub id: &'static str,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<u8>>,
    pub render_mode: RenderMode,
}

#[derive(Clone, Copy, PartialEq)]
pub enum RenderMode {
    Normal,
    Dream,
    Glitch,
}

impl Map {
    pub fn pixel_width(&self) -> f32 { self.width as f32 * TILE_SIZE }
    pub fn pixel_height(&self) -> f32 { self.height as f32 * TILE_SIZE }

    pub fn is_solid(&self, col: usize, row: usize) -> bool {
        if col >= self.width || row >= self.height { return true; }
        let id = self.tiles[row][col];
        matches!(id, 2 | 3 | 4 | 6 | 7 | 9 | 10 | 16 | 17 | 99)
    }

    pub fn overworld() -> Self {
        Map {
            id: "overworld", width: 30, height: 25, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4],
                vec![4,4,4,0,0,5,0,0,4,4,4,0,0,0,1,1,0,0,4,4,4,0,0,5,0,0,0,4,4,4],
                vec![4,0,0,0,5,0,0,0,0,4,0,0,5,0,1,1,0,5,0,4,0,0,0,0,5,0,0,0,0,4],
                vec![4,0,5,0,0,0,0,5,0,0,0,0,0,0,1,1,0,0,0,0,0,7,7,7,7,0,0,5,0,4],
                vec![4,0,0,0,0,0,0,0,0,0,5,0,0,0,1,1,0,0,0,0,0,6,9,6,6,0,0,0,0,4],
                vec![4,0,0,0,7,7,7,0,0,0,0,0,0,1,1,1,1,0,0,5,0,6,8,6,6,0,0,0,0,4],
                vec![4,0,0,0,6,9,6,0,0,0,0,0,0,1,0,0,1,0,0,0,0,0,1,0,0,0,0,0,0,4],
                vec![4,0,0,0,6,8,6,0,0,0,0,0,0,1,0,0,1,1,1,1,1,1,1,0,0,0,5,0,0,4],
                vec![4,0,5,0,0,1,0,0,0,0,0,5,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4],
                vec![4,0,0,0,0,1,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,13,0,0,0,4],
                vec![4,0,0,0,0,1,1,1,1,1,1,1,1,1,0,0,0,0,0,10,10,10,0,0,0,0,0,0,0,4],
                vec![4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,2,2,0,0,0,0,5,0,0,0,0,4],
                vec![4,0,5,0,0,11,0,0,0,5,0,0,0,0,0,2,2,2,2,2,2,0,0,0,0,0,0,5,0,4],
                vec![4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,2,2,2,2,0,0,0,0,0,0,0,0,4],
                vec![4,0,0,0,0,0,0,5,0,0,0,0,1,1,12,12,2,2,2,2,0,0,0,0,0,0,5,0,0,4],
                vec![4,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,2,2,0,0,0,0,7,7,7,0,0,0,4],
                vec![4,0,0,5,0,0,0,0,0,0,0,0,1,0,0,5,0,0,0,0,0,0,0,6,9,6,0,0,0,4],
                vec![4,0,0,0,0,0,0,0,5,0,0,0,1,0,0,0,0,0,0,0,5,0,0,6,8,6,0,0,0,4],
                vec![4,0,0,0,0,5,0,0,0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,5,0,4],
                vec![4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4],
                vec![4,0,5,0,0,0,0,0,5,0,0,0,0,5,0,0,0,5,0,0,0,0,5,0,0,0,0,5,0,4],
                vec![4,0,0,0,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,5,0,0,0,0,0,0,0,0,4],
                vec![4,0,0,0,0,0,13,0,0,0,5,0,0,0,0,0,0,0,0,0,0,0,0,0,5,0,0,0,0,4],
                vec![4,4,0,0,0,4,4,4,0,0,4,4,0,0,0,0,0,0,4,4,0,0,4,4,4,0,0,0,4,4],
                vec![4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4],
            ],
        }
    }

    pub fn home() -> Self {
        Map {
            id: "home", width: 10, height: 8, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![3,3,3,3,3,3,3,3,3,3],
                vec![3,14,14,14,14,14,14,14,14,3],
                vec![3,14,15,15,15,14,14,17,14,3],
                vec![3,14,15,16,15,14,14,17,14,3],
                vec![3,14,15,15,15,14,14,14,14,3],
                vec![3,14,14,14,14,14,14,14,14,3],
                vec![3,14,14,14,8,14,14,14,14,3],
                vec![3,3,3,3,3,3,3,3,3,3],
            ],
        }
    }

    pub fn lab() -> Self {
        Map {
            id: "lab", width: 12, height: 9, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![3,3,3,3,3,3,3,3,3,3,3,3],
                vec![3,14,14,17,17,14,14,17,17,14,14,3],
                vec![3,14,14,14,14,14,14,14,14,14,14,3],
                vec![3,14,16,14,14,14,14,14,14,16,14,3],
                vec![3,14,14,14,15,15,15,15,14,14,14,3],
                vec![3,14,14,14,15,13,13,15,14,14,14,3],
                vec![3,14,14,14,14,14,14,14,14,14,14,3],
                vec![3,14,14,14,14,8,14,14,14,14,14,3],
                vec![3,3,3,3,3,3,3,3,3,3,3,3],
            ],
        }
    }

    pub fn shop() -> Self {
        Map {
            id: "shop", width: 10, height: 8, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![3,3,3,3,3,3,3,3,3,3],
                vec![3,14,17,17,14,14,17,17,14,3],
                vec![3,14,14,14,14,14,14,14,14,3],
                vec![3,14,14,16,16,16,16,14,14,3],
                vec![3,14,14,14,14,14,14,14,14,3],
                vec![3,14,14,15,15,15,15,14,14,3],
                vec![3,14,14,14,8,14,14,14,14,3],
                vec![3,3,3,3,3,3,3,3,3,3],
            ],
        }
    }

    pub fn doghouse() -> Self {
        Map {
            id: "doghouse", width: 16, height: 12, render_mode: RenderMode::Glitch,
            tiles: vec![
                vec![99,99,99,99,99,99,99,99,99,99,99,99,99,99,99,99],
                vec![99,14,98,98,97,14,96,96,14,95,95,14,98,14,14,99],
                vec![99,98,14,14,14,97,14,14,96,14,14,95,14,14,98,99],
                vec![99,14,14,14,14,14,14,14,14,14,14,14,14,14,14,99],
                vec![99,97,14,14,16,14,14,14,14,14,14,16,14,14,97,99],
                vec![99,14,14,14,14,14,15,15,15,14,14,14,14,14,14,99],
                vec![99,96,14,14,14,14,15,13,15,14,14,14,14,14,96,99],
                vec![99,14,14,14,14,14,15,15,15,14,14,14,14,14,14,99],
                vec![99,95,14,14,14,14,14,14,14,14,14,14,14,14,95,99],
                vec![99,14,14,14,14,14,14,14,14,14,14,14,14,14,14,99],
                vec![99,14,98,14,14,97,14,8,14,96,14,14,95,14,14,99],
                vec![99,99,99,99,99,99,99,99,99,99,99,99,99,99,99,99],
            ],
        }
    }

    pub fn grove() -> Self {
        Map {
            id: "grove", width: 12, height: 10, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![4,4,4,4,4,4,4,4,4,4,4,4],
                vec![4,5,0,0,5,0,0,5,0,0,5,4],
                vec![4,0,0,5,0,0,0,0,5,0,0,4],
                vec![4,0,5,0,0,13,13,0,0,5,0,4],
                vec![4,5,0,0,0,5,5,0,0,0,5,4],
                vec![4,0,0,0,5,0,0,5,0,0,0,4],
                vec![4,0,5,0,0,0,0,0,0,5,0,4],
                vec![4,5,0,0,0,0,0,0,0,0,5,4],
                vec![4,0,0,5,0,1,0,5,0,0,0,4],
                vec![4,4,4,4,4,4,4,4,4,4,4,4],
            ],
        }
    }

    pub fn dream() -> Self {
        let mut m = Self::overworld();
        m.id = "dream";
        m.render_mode = RenderMode::Dream;
        m
    }

    pub fn by_id(id: &str) -> Self {
        match id {
            "overworld" => Self::overworld(),
            "home" => Self::home(),
            "lab" => Self::lab(),
            "shop" => Self::shop(),
            "dream" => Self::dream(),
            "doghouse" => Self::doghouse(),
            "grove" => Self::grove(),
            _ => Self::overworld(),
        }
    }
}

/// Tile color. Glitch mode uses shifting colors for high IDs.
pub fn tile_color(tile_id: u8, mode: RenderMode, time: f32) -> Color {
    if mode == RenderMode::Glitch && tile_id >= 95 {
        let shift = ((time * 3.0 + tile_id as f32 * 0.7).sin() * 127.0 + 128.0) as u8;
        return Color::from_rgba(shift, 255 - shift, shift / 2, 255);
    }

    if mode == RenderMode::Dream {
        // Dreamy palette — muted purples and blues
        return match tile_id {
            0 => Color::from_rgba(106, 90, 205, 255),  // lavender grass
            1 => Color::from_rgba(180, 170, 200, 255),  // misty path
            2 => Color::from_rgba(100, 100, 180, 255),  // deep water
            4 => Color::from_rgba(72, 61, 139, 255),    // dark purple trees
            5 => Color::from_rgba(186, 140, 220, 255),  // purple flowers
            _ => tile_color_normal(tile_id),
        };
    }

    tile_color_normal(tile_id)
}

fn tile_color_normal(tile_id: u8) -> Color {
    match tile_id {
        0 => Color::from_rgba(76, 175, 80, 255),     // grass
        1 => Color::from_rgba(189, 189, 189, 255),    // path
        2 => Color::from_rgba(66, 165, 245, 255),     // water
        3 => Color::from_rgba(121, 85, 72, 255),      // wall
        4 => Color::from_rgba(27, 94, 32, 255),       // tree
        5 => Color::from_rgba(139, 195, 74, 255),     // flower
        6 => Color::from_rgba(161, 136, 127, 255),    // house wall
        7 => Color::from_rgba(183, 28, 28, 255),      // roof
        8 => Color::from_rgba(93, 64, 55, 255),       // door
        9 => Color::from_rgba(120, 144, 156, 255),    // window
        10 => Color::from_rgba(255, 183, 77, 255),    // shop awning
        11 => Color::from_rgba(156, 39, 176, 255),    // sign
        12 => Color::from_rgba(255, 235, 59, 255),    // bridge
        13 => Color::from_rgba(255, 215, 0, 255),     // chest
        14 => Color::from_rgba(141, 110, 99, 255),    // wood floor
        15 => Color::from_rgba(188, 170, 164, 255),   // rug
        16 => Color::from_rgba(78, 52, 46, 255),      // table
        17 => Color::from_rgba(62, 39, 35, 255),      // shelf
        _ => Color::from_rgba(50, 50, 50, 255),       // unknown
    }
}

pub fn draw_map(map: &Map, cam_x: f32, cam_y: f32, view_w: f32, view_h: f32, time: f32) {
    let start_col = ((cam_x / TILE_SIZE).floor() as usize).saturating_sub(1);
    let start_row = ((cam_y / TILE_SIZE).floor() as usize).saturating_sub(1);
    let end_col = ((cam_x + view_w) / TILE_SIZE).ceil() as usize + 1;
    let end_row = ((cam_y + view_h) / TILE_SIZE).ceil() as usize + 1;

    for row in start_row..end_row.min(map.height) {
        for col in start_col..end_col.min(map.width) {
            let tile_id = map.tiles[row][col];
            let color = tile_color(tile_id, map.render_mode, time);
            let x = col as f32 * TILE_SIZE;
            let y = row as f32 * TILE_SIZE;
            draw_rectangle(x, y, TILE_SIZE, TILE_SIZE, color);
        }
    }
}
