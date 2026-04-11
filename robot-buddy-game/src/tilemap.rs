use macroquad::prelude::*;

use crate::sprites::Dir;

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Tile {
    Grass = 0,
    Path = 1,
    Water = 2,
    Wall = 3,
    Tree = 4,
    Flower = 5,
    HouseWall = 6,
    Roof = 7,
    Door = 8,
    Window = 9,
    Fence = 10,
    Sign = 11,
    Bridge = 12,
    Chest = 13,
    WoodFloor = 14,
    Rug = 15,
    Table = 16,
    Bookshelf = 17,
    // Glitch-only tiles (doghouse)
    Glitch95 = 95,
    Glitch96 = 96,
    Glitch97 = 97,
    Glitch98 = 98,
    GlitchWall = 99,
}

pub const TILE_SIZE: f32 = 48.0;

// ─── PORTALS ────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Portal {
    pub from_map: &'static str,
    pub from_x: usize,
    pub from_y: usize,
    pub to_map: &'static str,
    pub to_x: usize,
    pub to_y: usize,
    pub dir: Dir,
    pub secret: bool,
}

/// All portals in the game. Checked after each player move.
pub fn all_portals() -> &'static [Portal] {
    &[
        // Home: overworld door → home interior, home door → overworld
        Portal { from_map: "overworld", from_x: 5, from_y: 7, to_map: "home", to_x: 4, to_y: 5, dir: Dir::Up, secret: false },
        Portal { from_map: "home", from_x: 4, from_y: 6, to_map: "overworld", to_x: 5, to_y: 8, dir: Dir::Down, secret: false },
        // Lab: overworld east house → lab interior
        Portal { from_map: "overworld", from_x: 22, from_y: 5, to_map: "lab", to_x: 5, to_y: 6, dir: Dir::Up, secret: false },
        Portal { from_map: "lab", from_x: 5, from_y: 7, to_map: "overworld", to_x: 22, to_y: 6, dir: Dir::Down, secret: false },
        // Shop: overworld south house → shop interior
        Portal { from_map: "overworld", from_x: 24, from_y: 17, to_map: "shop", to_x: 4, to_y: 5, dir: Dir::Up, secret: false },
        Portal { from_map: "shop", from_x: 4, from_y: 6, to_map: "overworld", to_x: 24, to_y: 18, dir: Dir::Down, secret: false },
        // SECRET: Dream world — water tile past bridge
        Portal { from_map: "overworld", from_x: 16, from_y: 14, to_map: "dream", to_x: 14, to_y: 13, dir: Dir::Down, secret: true },
        Portal { from_map: "dream", from_x: 16, from_y: 14, to_map: "overworld", to_x: 13, to_y: 14, dir: Dir::Left, secret: false },
        // Dream-mode mirrors of overworld portals (same doors work in dream)
        Portal { from_map: "dream", from_x: 5, from_y: 7, to_map: "home", to_x: 4, to_y: 5, dir: Dir::Up, secret: false },
        Portal { from_map: "dream", from_x: 22, from_y: 5, to_map: "lab", to_x: 5, to_y: 6, dir: Dir::Up, secret: false },
        Portal { from_map: "dream", from_x: 24, from_y: 17, to_map: "shop", to_x: 4, to_y: 5, dir: Dir::Up, secret: false },
        // SECRET: Doghouse land — roof tile behind home
        Portal { from_map: "overworld", from_x: 5, from_y: 5, to_map: "doghouse", to_x: 7, to_y: 1, dir: Dir::Down, secret: true },
        Portal { from_map: "doghouse", from_x: 7, from_y: 10, to_map: "overworld", to_x: 5, to_y: 4, dir: Dir::Down, secret: false },
        // SECRET: Hidden grove — tree at top border
        Portal { from_map: "overworld", from_x: 15, from_y: 0, to_map: "grove", to_x: 5, to_y: 8, dir: Dir::Up, secret: true },
        Portal { from_map: "grove", from_x: 5, from_y: 8, to_map: "overworld", to_x: 15, to_y: 1, dir: Dir::Down, secret: false },
    ]
}

/// Check if the player is standing on a portal.
pub fn check_portal(map_id: &str, col: usize, row: usize) -> Option<&'static Portal> {
    all_portals().iter().find(|p| p.from_map == map_id && p.from_x == col && p.from_y == row)
}

/// Secret walkable tiles — normally solid tiles that portals make walkable.
const SECRET_WALKABLE: &[(&str, usize, usize)] = &[
    ("overworld", 16, 14), // water tile → dream portal
    ("overworld", 5, 5),   // roof tile → doghouse portal
    ("overworld", 15, 0),  // tree tile → grove portal
    ("dream", 16, 14),     // water tile → dream exit portal
];

fn is_secret_walkable(map_id: &str, col: usize, row: usize) -> bool {
    SECRET_WALKABLE.iter().any(|(m, x, y)| *m == map_id && *x == col && *y == row)
}

// ─── MAP ────────────────────────────────────────────────

#[derive(Clone)]
pub struct Map {
    pub id: &'static str,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<Tile>>,
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
        if is_secret_walkable(self.id, col, row) { return false; }
        let tile = self.tiles[row][col];
        matches!(tile, Tile::Water | Tile::Wall | Tile::Tree | Tile::HouseWall | Tile::Roof | Tile::Window | Tile::Fence | Tile::Sign | Tile::Chest | Tile::Table | Tile::Bookshelf | Tile::GlitchWall)
    }

    #[allow(non_snake_case)]
    pub fn overworld() -> Self {
        use Tile::*;
        let (Gr, Pa, Wa, Tr, Fl) = (Grass, Path, Water, Tree, Flower);
        let (HW, Rf, Dr, Wi, Fc, Sg) = (HouseWall, Roof, Door, Window, Fence, Sign);
        let (Br, Ch) = (Bridge, Chest);
        Map {
            id: "overworld", width: 30, height: 25, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr],
                vec![Tr,Tr,Tr,Gr,Gr,Fl,Gr,Gr,Tr,Tr,Tr,Gr,Gr,Gr,Pa,Pa,Gr,Gr,Tr,Tr,Tr,Gr,Gr,Fl,Gr,Gr,Gr,Tr,Tr,Tr],
                vec![Tr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Tr,Gr,Gr,Fl,Gr,Pa,Pa,Gr,Fl,Gr,Tr,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Fl,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Gr,Gr,Pa,Pa,Gr,Gr,Gr,Gr,Gr,Rf,Rf,Rf,Rf,Gr,Gr,Fl,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Pa,Pa,Gr,Gr,Gr,Gr,Gr,HW,Wi,HW,HW,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Rf,Rf,Rf,Gr,Gr,Gr,Gr,Gr,Gr,Pa,Pa,Pa,Pa,Gr,Gr,Fl,Gr,HW,Dr,HW,HW,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,HW,Wi,HW,Gr,Gr,Gr,Gr,Gr,Gr,Pa,Gr,Gr,Pa,Gr,Gr,Gr,Gr,Gr,Pa,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,HW,Dr,HW,Gr,Gr,Gr,Gr,Gr,Gr,Pa,Gr,Gr,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Gr,Gr,Gr,Fl,Gr,Gr,Tr],
                vec![Tr,Gr,Fl,Gr,Gr,Pa,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Pa,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Pa,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Pa,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Ch,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Gr,Gr,Gr,Gr,Gr,Fc,Fc,Fc,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Wa,Wa,Wa,Wa,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Fl,Gr,Gr,Sg,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Gr,Wa,Wa,Wa,Wa,Wa,Wa,Gr,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Wa,Wa,Wa,Wa,Wa,Wa,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Pa,Pa,Br,Br,Wa,Wa,Wa,Wa,Gr,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Pa,Gr,Gr,Gr,Gr,Wa,Wa,Gr,Gr,Gr,Gr,Rf,Rf,Rf,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Pa,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Gr,Gr,Gr,HW,Wi,HW,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Pa,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Gr,HW,Dr,HW,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Gr,Gr,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Pa,Gr,Gr,Fl,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Fl,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Fl,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Ch,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Tr,Gr,Gr,Gr,Tr,Tr,Tr,Gr,Gr,Tr,Tr,Gr,Gr,Gr,Gr,Gr,Gr,Tr,Tr,Gr,Gr,Tr,Tr,Tr,Gr,Gr,Gr,Tr,Tr],
                vec![Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr],
            ],
        }
    }

    #[allow(non_snake_case)]
    pub fn home() -> Self {
        use Tile::*;
        let (Wl, WF, Rg, Tb, Bs, Dr) = (Wall, WoodFloor, Rug, Table, Bookshelf, Door);
        Map {
            id: "home", width: 10, height: 8, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,Rg,Rg,Rg,WF,WF,Bs,WF,Wl],
                vec![Wl,WF,Rg,Tb,Rg,WF,WF,Bs,WF,Wl],
                vec![Wl,WF,Rg,Rg,Rg,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,Dr,WF,WF,WF,WF,Wl],
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
            ],
        }
    }

    #[allow(non_snake_case)]
    pub fn lab() -> Self {
        use Tile::*;
        let (Wl, WF, Rg, Tb, Bs, Dr, Ch) = (Wall, WoodFloor, Rug, Table, Bookshelf, Door, Chest);
        Map {
            id: "lab", width: 12, height: 9, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
                vec![Wl,WF,WF,Bs,Bs,WF,WF,Bs,Bs,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,Tb,WF,WF,WF,WF,WF,WF,Tb,WF,Wl],
                vec![Wl,WF,WF,WF,Rg,Rg,Rg,Rg,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,Rg,Ch,Ch,Rg,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,Dr,WF,WF,WF,WF,WF,Wl],
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
            ],
        }
    }

    #[allow(non_snake_case)]
    pub fn shop() -> Self {
        use Tile::*;
        let (Wl, WF, Rg, Tb, Bs, Dr) = (Wall, WoodFloor, Rug, Table, Bookshelf, Door);
        Map {
            id: "shop", width: 10, height: 8, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
                vec![Wl,WF,Bs,Bs,WF,WF,Bs,Bs,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,Tb,Tb,Tb,Tb,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,Rg,Rg,Rg,Rg,WF,WF,Wl],
                vec![Wl,WF,WF,WF,Dr,WF,WF,WF,WF,Wl],
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
            ],
        }
    }

    #[allow(non_snake_case)]
    pub fn doghouse() -> Self {
        use Tile::*;
        let (GW, WF, Rg, Tb, Ch, Dr) = (GlitchWall, WoodFloor, Rug, Table, Chest, Door);
        let (G5, G6, G7, G8) = (Glitch95, Glitch96, Glitch97, Glitch98);
        Map {
            id: "doghouse", width: 16, height: 12, render_mode: RenderMode::Glitch,
            tiles: vec![
                vec![GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW],
                vec![GW,WF,G8,G8,G7,WF,G6,G6,WF,G5,G5,WF,G8,WF,WF,GW],
                vec![GW,G8,WF,WF,WF,G7,WF,WF,G6,WF,WF,G5,WF,WF,G8,GW],
                vec![GW,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,GW],
                vec![GW,G7,WF,WF,Tb,WF,WF,WF,WF,WF,WF,Tb,WF,WF,G7,GW],
                vec![GW,WF,WF,WF,WF,WF,Rg,Rg,Rg,WF,WF,WF,WF,WF,WF,GW],
                vec![GW,G6,WF,WF,WF,WF,Rg,Ch,Rg,WF,WF,WF,WF,WF,G6,GW],
                vec![GW,WF,WF,WF,WF,WF,Rg,Rg,Rg,WF,WF,WF,WF,WF,WF,GW],
                vec![GW,G5,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,G5,GW],
                vec![GW,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,GW],
                vec![GW,WF,G8,WF,WF,G7,WF,Dr,WF,G6,WF,WF,G5,WF,WF,GW],
                vec![GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW,GW],
            ],
        }
    }

    #[allow(non_snake_case)]
    pub fn grove() -> Self {
        use Tile::*;
        let (Gr, Pa, Tr, Fl, Ch) = (Grass, Path, Tree, Flower, Chest);
        Map {
            id: "grove", width: 12, height: 10, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr],
                vec![Tr,Fl,Gr,Gr,Fl,Gr,Gr,Fl,Gr,Gr,Fl,Tr],
                vec![Tr,Gr,Gr,Fl,Gr,Gr,Gr,Gr,Fl,Gr,Gr,Tr],
                vec![Tr,Gr,Fl,Gr,Gr,Ch,Ch,Gr,Gr,Fl,Gr,Tr],
                vec![Tr,Fl,Gr,Gr,Gr,Fl,Fl,Gr,Gr,Gr,Fl,Tr],
                vec![Tr,Gr,Gr,Gr,Fl,Gr,Gr,Fl,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Fl,Gr,Gr,Gr,Gr,Gr,Gr,Fl,Gr,Tr],
                vec![Tr,Fl,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Fl,Tr],
                vec![Tr,Gr,Gr,Fl,Gr,Pa,Gr,Fl,Gr,Gr,Gr,Tr],
                vec![Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr],
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
pub fn tile_color(tile: Tile, mode: RenderMode, time: f32) -> Color {
    if mode == RenderMode::Glitch && matches!(tile, Tile::Glitch95 | Tile::Glitch96 | Tile::Glitch97 | Tile::Glitch98 | Tile::GlitchWall) {
        let shift = ((time * 3.0 + (tile as u8) as f32 * 0.7).sin() * 127.0 + 128.0) as u8;
        return Color::from_rgba(shift, 255 - shift, shift / 2, 255);
    }

    if mode == RenderMode::Dream {
        // Dreamy palette — muted purples and blues
        return match tile {
            Tile::Grass    => Color::from_rgba(106, 90, 205, 255),  // lavender grass
            Tile::Path     => Color::from_rgba(180, 160, 200, 255), // misty path
            Tile::Water    => Color::from_rgba(50, 50, 140, 255),   // deep dream water (dark indigo)
            Tile::Tree     => Color::from_rgba(106, 90, 205, 255),  // trees (dream grass base)
            Tile::Flower   => Color::from_rgba(106, 90, 205, 255),  // flowers (dream grass base)
            _ => tile_color_normal(tile),
        };
    }

    tile_color_normal(tile)
}

fn tile_color_normal(tile: Tile) -> Color {
    match tile {
        Tile::Grass     => Color::from_rgba(76, 175, 80, 255),     // grass
        Tile::Path      => Color::from_rgba(222, 184, 135, 255),   // path (sandy)
        Tile::Water     => Color::from_rgba(66, 165, 245, 255),    // water
        Tile::Wall      => Color::from_rgba(121, 85, 72, 255),     // wall
        Tile::Tree      => Color::from_rgba(76, 175, 80, 255),     // tree (grass base)
        Tile::Flower    => Color::from_rgba(76, 175, 80, 255),     // flower (grass base)
        Tile::HouseWall => Color::from_rgba(255, 204, 128, 255),   // house wall (warm cream)
        Tile::Roof      => Color::from_rgba(211, 47, 47, 255),     // roof
        Tile::Door      => Color::from_rgba(255, 204, 128, 255),   // door (base = house wall)
        Tile::Window    => Color::from_rgba(255, 204, 128, 255),   // window (base = house wall)
        Tile::Fence     => Color::from_rgba(76, 175, 80, 255),     // fence (grass base)
        Tile::Sign      => Color::from_rgba(76, 175, 80, 255),     // sign (grass base)
        Tile::Bridge    => Color::from_rgba(66, 165, 245, 255),    // bridge (water base)
        Tile::Chest     => Color::from_rgba(76, 175, 80, 255),     // chest (grass base)
        Tile::WoodFloor => Color::from_rgba(161, 136, 127, 255),   // wood floor
        Tile::Rug       => Color::from_rgba(161, 136, 127, 255),   // rug (floor base)
        Tile::Table     => Color::from_rgba(78, 52, 46, 255),      // table
        Tile::Bookshelf => Color::from_rgba(62, 39, 35, 255),      // shelf
        _               => Color::from_rgba(50, 50, 50, 255),      // unknown / glitch
    }
}

pub fn draw_map(map: &Map, cam_x: f32, cam_y: f32, view_w: f32, view_h: f32, time: f32) {
    let start_col = ((cam_x / TILE_SIZE).floor() as usize).saturating_sub(1);
    let start_row = ((cam_y / TILE_SIZE).floor() as usize).saturating_sub(1);
    let end_col = ((cam_x + view_w) / TILE_SIZE).ceil() as usize + 1;
    let end_row = ((cam_y + view_h) / TILE_SIZE).ceil() as usize + 1;

    for row in start_row..end_row.min(map.height) {
        for col in start_col..end_col.min(map.width) {
            let tile = map.tiles[row][col];
            let color = tile_color(tile, map.render_mode, time);
            let x = col as f32 * TILE_SIZE;
            let y = row as f32 * TILE_SIZE;
            draw_rectangle(x, y, TILE_SIZE, TILE_SIZE, color);
            draw_tile_detail(tile, x, y, time, map.render_mode);
        }
    }

    // Dream sparkle overlay
    if map.render_mode == RenderMode::Dream {
        draw_dream_sparkles(cam_x, cam_y, view_w, view_h, time);
    }

    // Glitch scanlines + screen tear
    if map.render_mode == RenderMode::Glitch {
        draw_glitch_overlay(cam_x, cam_y, view_w, view_h, time);
    }
}

/// Stable pseudo-random for per-tile variation (same as old JS seededRandom)
fn seeded_random(x: f32, y: f32, seed: i32) -> f32 {
    let mut h = (x as i32).wrapping_mul(374761393)
        .wrapping_add((y as i32).wrapping_mul(668265263))
        .wrapping_add(seed.wrapping_mul(1274126177));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h & 0x7fffffff) as f32 / 0x7fffffff as f32
}

fn draw_tile_detail(tile: Tile, x: f32, y: f32, time: f32, mode: RenderMode) {
    // Skip details for glitch tiles
    if mode == RenderMode::Glitch && matches!(tile, Tile::Glitch95 | Tile::Glitch96 | Tile::Glitch97 | Tile::Glitch98 | Tile::GlitchWall) { return; }

    match tile {
        Tile::Grass     => draw_grass_detail(x, y),
        Tile::Path      => draw_path_detail(x, y),
        Tile::Water     => draw_water_detail(x, y, time, mode),
        Tile::Wall      => draw_wall_detail(x, y),
        Tile::Tree      => draw_tree_detail(x, y, time, mode),
        Tile::Flower    => draw_flower_detail(x, y, time),
        Tile::HouseWall => draw_house_wall_detail(x, y),
        Tile::Roof      => draw_roof_detail(x, y),
        Tile::Door      => draw_door_detail(x, y),
        Tile::Window    => draw_window_detail(x, y),
        Tile::Fence     => draw_fence_detail(x, y),
        Tile::Sign      => draw_sign_detail(x, y),
        Tile::Bridge    => draw_bridge_detail(x, y),
        Tile::Chest     => draw_chest_detail(x, y, time),
        Tile::WoodFloor => draw_floor_detail(x, y),
        Tile::Rug       => draw_rug_detail(x, y),
        Tile::Table     => draw_table_detail(x, y),
        Tile::Bookshelf => draw_bookshelf_detail(x, y),
        _               => {}
    }
}

fn draw_grass_detail(x: f32, y: f32) {
    let darker = Color::from_rgba(67, 160, 71, 255);
    for i in 0..4 {
        let rx = seeded_random(x, y, i * 3) * (TILE_SIZE - 6.0) + 3.0;
        let ry = seeded_random(x, y, i * 3 + 1) * (TILE_SIZE - 6.0) + 3.0;
        draw_rectangle(x + rx, y + ry, 3.0, 3.0, darker);
    }
}

fn draw_path_detail(x: f32, y: f32) {
    // Subtle pebbles on sandy path
    let pebble = Color::from_rgba(200, 169, 110, 255);
    for i in 0..3 {
        let rx = seeded_random(x, y, i * 7) * (TILE_SIZE - 8.0) + 4.0;
        let ry = seeded_random(x, y, i * 7 + 1) * (TILE_SIZE - 8.0) + 4.0;
        draw_circle(x + rx, y + ry, 2.0, pebble);
    }
}

fn draw_water_detail(x: f32, y: f32, time: f32, mode: RenderMode) {
    // Animated wave lines
    let wave_color = if mode == RenderMode::Dream {
        Color::from_rgba(140, 140, 220, 180)
    } else {
        Color::from_rgba(100, 181, 246, 200)
    };
    for row in 0..3 {
        let base_y = y + 10.0 + row as f32 * 14.0;
        let mut prev_x = x;
        let mut prev_y = base_y + ((x) * 0.1 + time * 2.0 + row as f32).sin() * 3.0;
        let mut px = 4.0;
        while px <= TILE_SIZE {
            let wave = ((x + px) * 0.1 + time * 2.0 + row as f32).sin() * 3.0;
            let cur_x = x + px;
            let cur_y = base_y + wave;
            draw_line(prev_x, prev_y, cur_x, cur_y, 1.5, wave_color);
            prev_x = cur_x;
            prev_y = cur_y;
            px += 4.0;
        }
    }
}

fn draw_wall_detail(x: f32, y: f32) {
    // Brick pattern
    let mortar = Color::from_rgba(109, 76, 65, 255);
    for row in 0..3 {
        let by = y + row as f32 * 16.0;
        draw_rectangle_lines(x, by, TILE_SIZE, 16.0, 1.0, mortar);
        let offset = if row % 2 == 0 { 0.0 } else { TILE_SIZE / 2.0 };
        draw_line(x + TILE_SIZE / 2.0 + offset, by, x + TILE_SIZE / 2.0 + offset, by + 16.0, 1.0, mortar);
    }
}

fn draw_tree_detail(x: f32, y: f32, time: f32, mode: RenderMode) {
    let sway = (time * 1.5 + x * 0.3).sin() * 1.5;
    // Trunk
    draw_rectangle(x + 19.0, y + 28.0, 10.0, 18.0, Color::from_rgba(109, 76, 65, 255));
    // Canopy
    let (c1, c2) = if mode == RenderMode::Dream {
        (Color::from_rgba(90, 75, 160, 255), Color::from_rgba(100, 85, 170, 255))
    } else {
        (Color::from_rgba(46, 125, 50, 255), Color::from_rgba(56, 142, 60, 255))
    };
    draw_circle(x + 24.0 + sway, y + 20.0, 16.0, c1);
    draw_circle(x + 18.0 + sway, y + 24.0, 11.0, c2);
    draw_circle(x + 30.0 + sway, y + 24.0, 11.0, c2);
}

fn draw_flower_detail(x: f32, y: f32, time: f32) {
    // Grass tufts underneath
    draw_grass_detail(x, y);
    let colors = [
        Color::from_rgba(255, 107, 107, 255),
        Color::from_rgba(255, 217, 61, 255),
        Color::from_rgba(224, 64, 251, 255),
    ];
    let stem = Color::from_rgba(56, 142, 60, 255);
    let center = Color::from_rgba(255, 249, 196, 255);
    for i in 0..3 {
        let fx = x + 8.0 + seeded_random(x, y, i * 5) * 28.0;
        let fy = y + 8.0 + seeded_random(x, y, i * 5 + 1) * 28.0;
        let sway = (time * 2.0 + i as f32 * 2.0).sin() * 1.5;
        // Stem
        draw_line(fx, fy + 6.0, fx + sway, fy - 2.0, 2.0, stem);
        // Petals
        draw_circle(fx + sway, fy - 4.0, 4.0, colors[i as usize % 3]);
        // Center
        draw_circle(fx + sway, fy - 4.0, 1.5, center);
    }
}

fn draw_house_wall_detail(x: f32, y: f32) {
    // Orange border
    draw_rectangle_lines(x + 1.0, y + 1.0, TILE_SIZE - 2.0, TILE_SIZE - 2.0, 2.0,
        Color::from_rgba(239, 108, 0, 255));
}

fn draw_roof_detail(x: f32, y: f32) {
    // Shingle lines
    let shingle = Color::from_rgba(183, 28, 28, 255);
    for i in 0..3 {
        let ly = y + 12.0 + i as f32 * 14.0;
        draw_line(x, ly, x + TILE_SIZE, ly, 1.0, shingle);
    }
}

fn draw_door_detail(x: f32, y: f32) {
    // House wall base (already drawn as tile color = door brown, so draw house wall underneath)
    draw_rectangle(x, y, TILE_SIZE, TILE_SIZE, Color::from_rgba(255, 204, 128, 255));
    draw_rectangle_lines(x + 1.0, y + 1.0, TILE_SIZE - 2.0, TILE_SIZE - 2.0, 2.0,
        Color::from_rgba(239, 108, 0, 255));
    // Door
    draw_rectangle(x + 14.0, y + 10.0, 20.0, 38.0, Color::from_rgba(93, 64, 55, 255));
    draw_rectangle(x + 16.0, y + 12.0, 16.0, 34.0, Color::from_rgba(141, 110, 99, 255));
    // Doorknob
    draw_circle(x + 28.0, y + 30.0, 3.0, Color::from_rgba(255, 213, 79, 255));
}

fn draw_window_detail(x: f32, y: f32) {
    // House wall border already drawn. Add window.
    draw_rectangle_lines(x + 1.0, y + 1.0, TILE_SIZE - 2.0, TILE_SIZE - 2.0, 2.0,
        Color::from_rgba(239, 108, 0, 255));
    // Window pane
    draw_rectangle(x + 12.0, y + 12.0, 24.0, 20.0, Color::from_rgba(129, 212, 250, 255));
    draw_rectangle_lines(x + 12.0, y + 12.0, 24.0, 20.0, 2.0, Color::from_rgba(239, 108, 0, 255));
    // Crossbar
    draw_line(x + 24.0, y + 12.0, x + 24.0, y + 32.0, 2.0, Color::from_rgba(239, 108, 0, 255));
    draw_line(x + 12.0, y + 22.0, x + 36.0, y + 22.0, 2.0, Color::from_rgba(239, 108, 0, 255));
}

fn draw_fence_detail(x: f32, y: f32) {
    // Grass underneath
    draw_grass_detail(x, y);
    let post = Color::from_rgba(161, 136, 127, 255);
    let dark = Color::from_rgba(141, 110, 99, 255);
    // Posts
    draw_rectangle(x + 4.0, y + 12.0, 6.0, 30.0, post);
    draw_rectangle(x + 38.0, y + 12.0, 6.0, 30.0, post);
    // Rails
    draw_rectangle(x + 2.0, y + 16.0, 44.0, 5.0, post);
    draw_rectangle(x + 2.0, y + 30.0, 44.0, 5.0, post);
    // Pointed tops
    draw_triangle(vec2(x + 4.0, y + 12.0), vec2(x + 7.0, y + 6.0), vec2(x + 10.0, y + 12.0), dark);
    draw_triangle(vec2(x + 38.0, y + 12.0), vec2(x + 41.0, y + 6.0), vec2(x + 44.0, y + 12.0), dark);
}

fn draw_sign_detail(x: f32, y: f32) {
    // Grass base underneath
    draw_grass_detail(x, y);
    // Post
    draw_rectangle(x + 21.0, y + 22.0, 6.0, 24.0, Color::from_rgba(141, 110, 99, 255));
    // Sign board
    draw_rectangle(x + 8.0, y + 8.0, 32.0, 18.0, Color::from_rgba(255, 204, 128, 255));
    draw_rectangle_lines(x + 8.0, y + 8.0, 32.0, 18.0, 2.0, Color::from_rgba(109, 76, 65, 255));
    // "!" on sign
    draw_text("!", x + 21.0, y + 23.0, 16.0, Color::from_rgba(211, 47, 47, 255));
}

fn draw_bridge_detail(x: f32, y: f32) {
    // Wooden planks over water
    draw_rectangle(x + 4.0, y, 40.0, TILE_SIZE, Color::from_rgba(161, 136, 127, 255));
    // Plank lines
    let plank = Color::from_rgba(141, 110, 99, 255);
    for i in 0..4 {
        let ly = y + i as f32 * 12.0 + 12.0;
        draw_line(x + 4.0, ly, x + 44.0, ly, 1.0, plank);
    }
    // Rails
    let rail = Color::from_rgba(109, 76, 65, 255);
    draw_rectangle(x + 2.0, y, 4.0, TILE_SIZE, rail);
    draw_rectangle(x + 42.0, y, 4.0, TILE_SIZE, rail);
}

fn draw_chest_detail(x: f32, y: f32, time: f32) {
    // Grass base
    draw_grass_detail(x, y);
    // Chest body
    draw_rectangle(x + 10.0, y + 20.0, 28.0, 20.0, Color::from_rgba(141, 110, 99, 255));
    // Chest lid
    draw_rectangle(x + 8.0, y + 14.0, 32.0, 12.0, Color::from_rgba(161, 136, 127, 255));
    // Metal band
    draw_rectangle(x + 10.0, y + 18.0, 28.0, 3.0, Color::from_rgba(255, 213, 79, 255));
    // Lock
    draw_circle(x + 24.0, y + 28.0, 4.0, Color::from_rgba(255, 213, 79, 255));
    // Sparkle
    let sparkle = (time * 3.0).sin() * 0.5 + 0.5;
    draw_circle(x + 32.0, y + 12.0, 3.0, Color::new(1.0, 0.922, 0.231, sparkle));
}

fn draw_floor_detail(x: f32, y: f32) {
    // Wood plank lines
    let plank = Color::from_rgba(141, 110, 99, 255);
    for i in 0..3 {
        let ly = y + i as f32 * 16.0 + 8.0;
        draw_line(x, ly, x + TILE_SIZE, ly, 1.0, plank);
    }
    // Vertical seam
    let seam = if seeded_random(x, y, 99) < 0.5 { 20.0 } else { 28.0 };
    draw_line(x + seam, y, x + seam, y + TILE_SIZE, 1.0, plank);
}

fn draw_rug_detail(x: f32, y: f32) {
    // Floor underneath
    draw_floor_detail(x, y);
    // Rug
    draw_rectangle(x + 2.0, y + 2.0, TILE_SIZE - 4.0, TILE_SIZE - 4.0,
        Color::from_rgba(198, 40, 40, 255));
    // Gold border pattern
    draw_rectangle_lines(x + 6.0, y + 6.0, TILE_SIZE - 12.0, TILE_SIZE - 12.0, 2.0,
        Color::from_rgba(255, 213, 79, 255));
    // Center diamond
    let gold = Color::from_rgba(255, 213, 79, 255);
    let cx = x + TILE_SIZE / 2.0;
    let cy = y + TILE_SIZE / 2.0;
    draw_triangle(vec2(cx, y + 12.0), vec2(x + TILE_SIZE - 12.0, cy), vec2(cx, y + TILE_SIZE - 12.0), gold);
    draw_triangle(vec2(cx, y + 12.0), vec2(x + 12.0, cy), vec2(cx, y + TILE_SIZE - 12.0), gold);
}

fn draw_table_detail(x: f32, y: f32) {
    // Floor underneath
    draw_floor_detail(x, y);
    // Table top
    draw_rectangle(x + 4.0, y + 8.0, TILE_SIZE - 8.0, TILE_SIZE - 16.0,
        Color::from_rgba(109, 76, 65, 255));
    draw_rectangle_lines(x + 4.0, y + 8.0, TILE_SIZE - 8.0, TILE_SIZE - 16.0, 2.0,
        Color::from_rgba(93, 64, 55, 255));
    // Items on table
    draw_rectangle(x + 14.0, y + 14.0, 10.0, 8.0, Color::from_rgba(129, 212, 250, 255));
    draw_rectangle(x + 26.0, y + 16.0, 8.0, 6.0, Color::from_rgba(224, 224, 224, 255));
}

fn draw_bookshelf_detail(x: f32, y: f32) {
    // Shelf frame
    draw_rectangle(x + 2.0, y + 2.0, TILE_SIZE - 4.0, TILE_SIZE - 4.0,
        Color::from_rgba(141, 110, 99, 255));
    // Shelves
    let shelf = Color::from_rgba(109, 76, 65, 255);
    draw_rectangle(x + 2.0, y + 20.0, TILE_SIZE - 4.0, 3.0, shelf);
    draw_rectangle(x + 2.0, y + 38.0, TILE_SIZE - 4.0, 3.0, shelf);
    // Books (top shelf)
    let book_colors = [
        Color::from_rgba(244, 67, 54, 255),
        Color::from_rgba(33, 150, 243, 255),
        Color::from_rgba(76, 175, 80, 255),
        Color::from_rgba(255, 152, 0, 255),
        Color::from_rgba(156, 39, 176, 255),
    ];
    for i in 0..5 {
        let bw = 5.0 + seeded_random(x, y, i * 2) * 3.0;
        let bx = x + 5.0 + i as f32 * 8.0;
        draw_rectangle(bx, y + 5.0, bw, 15.0, book_colors[i as usize % 5]);
    }
    // Books (bottom shelf)
    for i in 0..4 {
        let bw = 6.0 + seeded_random(x, y, i * 3 + 10) * 3.0;
        let bx = x + 6.0 + i as f32 * 9.0;
        draw_rectangle(bx, y + 24.0, bw, 13.0, book_colors[(i as usize + 3) % 5]);
    }
}

fn draw_dream_sparkles(cam_x: f32, cam_y: f32, view_w: f32, view_h: f32, time: f32) {
    for i in 0..30 {
        let seed = i as f32 * 137.5; // golden angle spread
        let sx = cam_x + ((seed * 7.3 + time * 15.0).sin() * 0.5 + 0.5) * view_w;
        let sy = cam_y + ((seed * 13.1 + time * 10.0).cos() * 0.5 + 0.5) * view_h;
        let alpha = ((time * 2.5 + seed).sin() * 0.5 + 0.5) as f32;
        let size = 2.0 + ((time * 3.0 + seed * 0.5).sin().abs()) * 2.0;
        if alpha > 0.2 {
            let color = Color::new(1.0, 1.0, 0.9, alpha * 0.7);
            draw_circle(sx, sy, size, color);
        }
    }
}

fn draw_glitch_overlay(cam_x: f32, cam_y: f32, view_w: f32, view_h: f32, time: f32) {
    // CRT scanlines — every 3 pixels
    let scanline_color = Color::new(0.0, 0.0, 0.0, 0.15);
    let mut y = cam_y;
    while y < cam_y + view_h {
        draw_line(cam_x, y, cam_x + view_w, y, 1.0, scanline_color);
        y += 3.0;
    }

    // Occasional screen tear — horizontal displacement of a strip
    let tear_cycle = (time * 0.7).sin();
    if tear_cycle > 0.85 {
        let tear_y = cam_y + view_h * 0.7 + (time * 50.0).sin() * 30.0;
        let tear_h = 4.0;
        let shift = (time * 100.0).sin() * 8.0;
        draw_rectangle(cam_x + shift, tear_y, view_w, tear_h,
            Color::new(0.0, 1.0, 0.5, 0.15));
    }
}
