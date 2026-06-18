use crate::prelude::*;

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
    // Aquatic tiles (reef and future water maps)
    SeaFloor = 18, // walkable underwater ground
    Sand = 19,     // walkable sandy patch
    Coral = 20,    // solid coral outcrop
    Kelp = 21,     // solid kelp/seaweed wall
    Bubble = 22,   // walkable bubble vent (decorative)
    // Portal markers — walkable tiles that visibly flag a special exit. A
    // reusable family: each themed map gets its own (DiveSpot today).
    DiveSpot = 23, // swirling whirlpool → reef dive portal
    // Space tiles (hub + planets, and future cosmic maps)
    Space = 24,       // deep-space floor (walkable, dark)
    Star = 25,        // twinkling star decoration (walkable)
    SpaceRock = 26,   // solid asteroid / hull wall
    Launchpad = 27,   // walkable launch marker (lab → hub, hub → lab)
    MoonPad = 28,     // hub marker → Moon
    MarsPad = 29,     // hub marker → Red Planet
    AsteroidPad = 30, // hub marker → Asteroid Base
    MoonGround = 31,  // walkable gray crater floor
    MarsGround = 32,  // walkable red dust
    StationFloor = 33,// walkable metal station floor
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
    /// Dum Dums the player must spend to pass. 0 = free. A reusable one-time
    /// toll so any map can sit behind a price; `handle_portal` charges once.
    pub cost: u32,
    /// Fuel spent each time the player takes this portal (0 = free). Unlike
    /// `cost` this is charged every trip — it's how rocket jumps burn fuel.
    pub fuel_cost: u32,
}

/// All portals in the game. Checked after each player move.
pub fn all_portals() -> &'static [Portal] {
    &[
        // Home: overworld door → home interior, home door → overworld
        Portal { from_map: "overworld", from_x: 5, from_y: 7, to_map: "home", to_x: 4, to_y: 5, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
        Portal { from_map: "home", from_x: 4, from_y: 6, to_map: "overworld", to_x: 5, to_y: 8, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 0 },
        // Lab: overworld east house → lab interior
        Portal { from_map: "overworld", from_x: 22, from_y: 5, to_map: "lab", to_x: 5, to_y: 6, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
        Portal { from_map: "lab", from_x: 5, from_y: 7, to_map: "overworld", to_x: 22, to_y: 6, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 0 },
        // Shop: overworld south house → shop interior
        Portal { from_map: "overworld", from_x: 24, from_y: 17, to_map: "shop", to_x: 4, to_y: 5, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
        Portal { from_map: "shop", from_x: 4, from_y: 6, to_map: "overworld", to_x: 24, to_y: 18, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 0 },
        // SECRET: Dream world — water tile past bridge
        Portal { from_map: "overworld", from_x: 16, from_y: 14, to_map: "dream", to_x: 14, to_y: 13, dir: Dir::Down, secret: true, cost: 0, fuel_cost: 0 },
        Portal { from_map: "dream", from_x: 16, from_y: 14, to_map: "overworld", to_x: 13, to_y: 14, dir: Dir::Left, secret: false, cost: 0, fuel_cost: 0 },
        // Dream-mode mirrors of overworld portals (same doors work in dream)
        Portal { from_map: "dream", from_x: 5, from_y: 7, to_map: "home", to_x: 4, to_y: 5, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
        Portal { from_map: "dream", from_x: 22, from_y: 5, to_map: "lab", to_x: 5, to_y: 6, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
        Portal { from_map: "dream", from_x: 24, from_y: 17, to_map: "shop", to_x: 4, to_y: 5, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
        // SECRET: Doghouse land — roof tile behind home
        Portal { from_map: "overworld", from_x: 5, from_y: 5, to_map: "doghouse", to_x: 7, to_y: 1, dir: Dir::Down, secret: true, cost: 0, fuel_cost: 0 },
        Portal { from_map: "dream", from_x: 5, from_y: 5, to_map: "doghouse", to_x: 7, to_y: 1, dir: Dir::Down, secret: true, cost: 0, fuel_cost: 0 },
        Portal { from_map: "doghouse", from_x: 7, from_y: 10, to_map: "overworld", to_x: 5, to_y: 4, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 0 },
        // SECRET: Hidden grove — tree at top border
        Portal { from_map: "overworld", from_x: 15, from_y: 0, to_map: "grove", to_x: 5, to_y: 8, dir: Dir::Up, secret: true, cost: 0, fuel_cost: 0 },
        Portal { from_map: "dream", from_x: 15, from_y: 0, to_map: "grove", to_x: 5, to_y: 8, dir: Dir::Up, secret: true, cost: 0, fuel_cost: 0 },
        Portal { from_map: "grove", from_x: 5, from_y: 8, to_map: "overworld", to_x: 15, to_y: 1, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 0 },
        // Dev → Control Room (knob bay for testing puzzle parameters in isolation)
        Portal { from_map: "dev", from_x: 1, from_y: 9, to_map: "control", to_x: 6, to_y: 1, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 0 },
        Portal { from_map: "control", from_x: 6, from_y: 7, to_map: "dev", to_x: 1, to_y: 9, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
        // Dev → Annex (validation field for new-map genericity)
        Portal { from_map: "dev", from_x: 13, from_y: 10, to_map: "annex", to_x: 4, to_y: 5, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 0 },
        Portal { from_map: "annex", from_x: 4, from_y: 6, to_map: "dev", to_x: 13, to_y: 9, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
        // SECRET: Coral reef — dive spot at the south edge of the pond. Costs a
        // few Dum Dums to dive in (the reef's toll); resurface for free.
        Portal { from_map: "overworld", from_x: 17, from_y: 15, to_map: "reef", to_x: 8, to_y: 9, dir: Dir::Up, secret: true, cost: 3, fuel_cost: 0 },
        Portal { from_map: "dream",     from_x: 17, from_y: 15, to_map: "reef", to_x: 8, to_y: 9, dir: Dir::Up, secret: true, cost: 3, fuel_cost: 0 },
        Portal { from_map: "reef", from_x: 8, from_y: 10, to_map: "overworld", to_x: 17, to_y: 16, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 0 },
        // SPACE: Gizmo's lab launchpad → the orbital hub, and back.
        Portal { from_map: "lab", from_x: 10, from_y: 2, to_map: "space_hub", to_x: 8, to_y: 9, dir: Dir::Up, secret: true, cost: 0, fuel_cost: 0 },
        Portal { from_map: "space_hub", from_x: 8, from_y: 10, to_map: "lab", to_x: 10, to_y: 3, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 0 },
        // Hub → planets. The Moon is a free hop; deeper worlds burn fuel.
        Portal { from_map: "space_hub", from_x: 3,  from_y: 2, to_map: "moon",          to_x: 6, to_y: 2, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 0 },
        Portal { from_map: "space_hub", from_x: 11, from_y: 2, to_map: "mars",          to_x: 6, to_y: 7, dir: Dir::Up,   secret: false, cost: 0, fuel_cost: 3 },
        Portal { from_map: "space_hub", from_x: 8,  from_y: 4, to_map: "asteroid_base", to_x: 6, to_y: 2, dir: Dir::Down, secret: false, cost: 0, fuel_cost: 4 },
        // Planet return pads → back to the hub (free).
        Portal { from_map: "moon",          from_x: 6, from_y: 7, to_map: "space_hub", to_x: 8, to_y: 9, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
        Portal { from_map: "mars",          from_x: 2, from_y: 7, to_map: "space_hub", to_x: 8, to_y: 9, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
        Portal { from_map: "asteroid_base", from_x: 3, from_y: 7, to_map: "space_hub", to_x: 8, to_y: 9, dir: Dir::Up, secret: false, cost: 0, fuel_cost: 0 },
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
    ("dream", 5, 5),       // roof tile → doghouse portal (dream-side)
    ("dream", 15, 0),      // tree tile → grove portal (dream-side)
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
    /// Underwater theme — teal palette + drifting bubble overlay. Reusable by
    /// any submerged map (reef today, deeper zones later).
    Aquatic,
    /// Space theme — dark void + twinkling starfield overlay. Used by the hub
    /// and every planet surface; per-tile colors carry each world's look.
    Cosmic,
}

impl Map {
    pub fn pixel_width(&self) -> f32 { self.width as f32 * TILE_SIZE }
    pub fn pixel_height(&self) -> f32 { self.height as f32 * TILE_SIZE }

    pub fn is_solid(&self, col: usize, row: usize) -> bool {
        if col >= self.width || row >= self.height { return true; }
        if is_secret_walkable(self.id, col, row) { return false; }
        let tile = self.tiles[row][col];
        matches!(tile, Tile::Water | Tile::Wall | Tile::Tree | Tile::HouseWall | Tile::Roof | Tile::Window | Tile::Fence | Tile::Sign | Tile::Chest | Tile::Table | Tile::Bookshelf | Tile::GlitchWall | Tile::Coral | Tile::Kelp | Tile::SpaceRock)
    }

    #[allow(non_snake_case)]
    pub fn overworld() -> Self {
        use Tile::*;
        let (Gr, Pa, Wa, Tr, Fl) = (Grass, Path, Water, Tree, Flower);
        let (HW, Rf, Dr, Wi, Fc, Sg) = (HouseWall, Roof, Door, Window, Fence, Sign);
        let (Br, Ch, Dv) = (Bridge, Chest, DiveSpot);
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
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Pa,Gr,Gr,Gr,Gr,Dv,Wa,Gr,Gr,Gr,Gr,Rf,Rf,Rf,Gr,Gr,Gr,Tr],
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
        let Lp = Launchpad;
        Map {
            id: "lab", width: 12, height: 9, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
                vec![Wl,WF,WF,Bs,Bs,WF,WF,Bs,Bs,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,Lp,Wl],
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

    #[allow(non_snake_case)]
    pub fn dev() -> Self {
        use Tile::*;
        let (Wl, WF, Rg, Tb, Bs, Ch, Dr) = (Wall, WoodFloor, Rug, Table, Bookshelf, Chest, Door);
        Map {
            id: "dev", width: 16, height: 12, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,Bs,WF,WF,WF,Tb,WF,WF,WF,WF,Tb,WF,WF,WF,Bs,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,Ch,WF,Ch,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,Dr,WF,WF,WF,WF,Rg,Rg,Rg,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,Rg,WF,Rg,WF,WF,WF,WF,Dr,WF,Wl],
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
            ],
        }
    }

    /// Dev-tier validation field, reached through the Door at (13,10) of the
    /// dev map. Plain grass clearing — exists to prove a brand-new map plugs
    /// into the portal / wander / companion machinery with no special-casing.
    #[allow(non_snake_case)]
    pub fn annex() -> Self {
        use Tile::*;
        let (Tr, Gr, Dr) = (Tree, Grass, Door);
        Map {
            id: "annex", width: 10, height: 8, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Gr,Gr,Gr,Dr,Gr,Gr,Gr,Gr,Tr],
                vec![Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr,Tr],
            ],
        }
    }

    /// Knob bay for dev/test runs. Each NPC is a single-purpose control —
    /// cycle a profile field, reset a flag, or fire a fresh puzzle. Reachable
    /// via the Door tile in the dev map's lower-left corner.
    #[allow(non_snake_case)]
    pub fn control() -> Self {
        use Tile::*;
        let (Wl, WF, Dr) = (Wall, WoodFloor, Door);
        Map {
            id: "control", width: 12, height: 9, render_mode: RenderMode::Normal,
            tiles: vec![
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,WF,WF,WF,WF,WF,Wl],
                vec![Wl,WF,WF,WF,WF,WF,Dr,WF,WF,WF,WF,Wl],
                vec![Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl,Wl],
            ],
        }
    }

    /// Coral reef — the first underwater map. A lower arrival lagoon, a coral
    /// wall with a single gap guarded by a napping shark, and a treasure cove
    /// up top. Border is solid kelp. `RenderMode::Aquatic` paints it teal and
    /// floats bubbles. Built as plain data so future water maps copy the shape.
    #[allow(non_snake_case)]
    pub fn reef() -> Self {
        use Tile::*;
        let (Ke, Co, SF, Sa, Bu, Ch) = (Kelp, Coral, SeaFloor, Sand, Bubble, Chest);
        Map {
            id: "reef", width: 16, height: 12, render_mode: RenderMode::Aquatic,
            tiles: vec![
                vec![Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke],
                vec![Ke,SF,SF,SF,SF,Bu,Co,SF,SF,Co,SF,SF,Bu,SF,SF,Ke],
                vec![Ke,SF,Co,SF,SF,SF,SF,Ch,SF,SF,SF,SF,Co,SF,SF,Ke],
                vec![Ke,SF,SF,SF,SF,Co,SF,SF,SF,Co,SF,SF,SF,SF,SF,Ke],
                vec![Ke,SF,Co,SF,SF,SF,SF,SF,SF,SF,SF,SF,Co,SF,SF,Ke],
                vec![Ke,Co,Co,Co,Co,Co,Co,Co,SF,Co,Co,Co,Co,Co,Co,Ke],
                vec![Ke,SF,SF,SF,SF,SF,SF,SF,SF,SF,SF,SF,SF,SF,SF,Ke],
                vec![Ke,SF,Bu,Sa,Sa,SF,SF,SF,SF,SF,SF,Sa,Sa,Bu,SF,Ke],
                vec![Ke,SF,SF,Sa,Sa,SF,SF,SF,SF,SF,SF,Sa,Sa,SF,SF,Ke],
                vec![Ke,SF,SF,SF,SF,SF,SF,SF,SF,SF,SF,SF,SF,SF,SF,Ke],
                vec![Ke,SF,SF,SF,SF,SF,SF,SF,Sa,SF,SF,SF,SF,SF,SF,Ke],
                vec![Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke,Ke],
            ],
        }
    }

    /// Orbital hub — the flyable starfield. You pilot the rocket (arrow keys)
    /// between planet pads. Border is solid asteroid rock; pads are portals.
    /// Cosmic render mode floats a starfield over everything.
    #[allow(non_snake_case)]
    pub fn space_hub() -> Self {
        use Tile::*;
        let (SR, Sp, St, Lp) = (SpaceRock, Space, Star, Launchpad);
        let (Mo, Ma, As) = (MoonPad, MarsPad, AsteroidPad);
        Map {
            id: "space_hub", width: 16, height: 12, render_mode: RenderMode::Cosmic,
            tiles: vec![
                vec![SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR],
                vec![SR,Sp,St,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,St,Sp,Sp,SR],
                vec![SR,Sp,Sp,Mo,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Ma,Sp,Sp,Sp,SR],
                vec![SR,Sp,Sp,Sp,Sp,Sp,St,Sp,Sp,St,Sp,Sp,Sp,Sp,Sp,SR],
                vec![SR,Sp,Sp,Sp,Sp,Sp,Sp,Sp,As,Sp,Sp,Sp,Sp,Sp,Sp,SR],
                vec![SR,Sp,St,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,St,Sp,Sp,SR],
                vec![SR,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,SR],
                vec![SR,Sp,Sp,Sp,St,Sp,Sp,Sp,Sp,Sp,St,Sp,Sp,Sp,Sp,SR],
                vec![SR,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,SR],
                vec![SR,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Sp,SR],
                vec![SR,Sp,Sp,Sp,Sp,Sp,Sp,Sp,Lp,Sp,Sp,Sp,Sp,Sp,Sp,SR],
                vec![SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR],
            ],
        }
    }

    /// The Moon — a gentle gray crater world, the free first stop.
    #[allow(non_snake_case)]
    pub fn moon() -> Self {
        use Tile::*;
        let (SR, MG, Lp) = (SpaceRock, MoonGround, Launchpad);
        Map {
            id: "moon", width: 12, height: 9, render_mode: RenderMode::Cosmic,
            tiles: vec![
                vec![SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR],
                vec![SR,MG,MG,MG,MG,MG,MG,MG,MG,MG,MG,SR],
                vec![SR,MG,MG,MG,MG,MG,MG,MG,MG,MG,MG,SR],
                vec![SR,MG,MG,SR,MG,MG,MG,MG,SR,MG,MG,SR],
                vec![SR,MG,MG,MG,MG,MG,MG,MG,MG,MG,MG,SR],
                vec![SR,MG,SR,MG,MG,MG,MG,MG,MG,SR,MG,SR],
                vec![SR,MG,MG,MG,MG,MG,MG,MG,MG,MG,MG,SR],
                vec![SR,MG,MG,MG,MG,MG,Lp,MG,MG,MG,MG,SR],
                vec![SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR],
            ],
        }
    }

    /// The Red Planet — dusty, with a rock wall split by a single gap that a
    /// friendly alien guards. Treasure waits in the cove above the wall.
    #[allow(non_snake_case)]
    pub fn mars() -> Self {
        use Tile::*;
        let (SR, RG, Lp, Ch) = (SpaceRock, MarsGround, Launchpad, Chest);
        Map {
            id: "mars", width: 12, height: 9, render_mode: RenderMode::Cosmic,
            tiles: vec![
                vec![SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR],
                vec![SR,RG,RG,RG,RG,RG,RG,RG,RG,RG,RG,SR],
                vec![SR,RG,RG,RG,RG,Ch,RG,RG,RG,RG,RG,SR],
                vec![SR,RG,RG,RG,RG,RG,RG,RG,RG,RG,RG,SR],
                vec![SR,SR,SR,SR,SR,RG,SR,SR,SR,SR,SR,SR],
                vec![SR,RG,RG,RG,RG,RG,RG,RG,RG,RG,RG,SR],
                vec![SR,RG,RG,RG,RG,RG,RG,RG,RG,RG,RG,SR],
                vec![SR,RG,Lp,RG,RG,RG,RG,RG,RG,RG,RG,SR],
                vec![SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR],
            ],
        }
    }

    /// Asteroid Base — a metal station interior. A star-chart terminal alien
    /// runs constellation (pattern) puzzles; other aliens mill about.
    #[allow(non_snake_case)]
    pub fn asteroid_base() -> Self {
        use Tile::*;
        let (SR, ST, Lp) = (SpaceRock, StationFloor, Launchpad);
        Map {
            id: "asteroid_base", width: 12, height: 9, render_mode: RenderMode::Cosmic,
            tiles: vec![
                vec![SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR],
                vec![SR,ST,ST,ST,ST,ST,ST,ST,ST,ST,ST,SR],
                vec![SR,ST,ST,ST,ST,ST,ST,ST,ST,ST,ST,SR],
                vec![SR,ST,SR,ST,ST,ST,ST,ST,SR,ST,ST,SR],
                vec![SR,ST,ST,ST,ST,ST,ST,ST,ST,ST,ST,SR],
                vec![SR,ST,ST,ST,ST,ST,ST,ST,ST,ST,ST,SR],
                vec![SR,ST,SR,ST,ST,ST,ST,ST,ST,SR,ST,SR],
                vec![SR,ST,ST,Lp,ST,ST,ST,ST,ST,ST,ST,SR],
                vec![SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR,SR],
            ],
        }
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
            "dev" => Self::dev(),
            "control" => Self::control(),
            "annex" => Self::annex(),
            "reef" => Self::reef(),
            "space_hub" => Self::space_hub(),
            "moon" => Self::moon(),
            "mars" => Self::mars(),
            "asteroid_base" => Self::asteroid_base(),
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
        // Dreamy palette — muted purples and blues. Exhaustive so the compiler
        // catches any new Tile variant that needs a dream color.
        let dream_grass = Color::from_rgba(106, 90, 205, 255);      // lavender
        let dream_water = Color::from_rgba(50, 50, 140, 255);       // deep indigo
        let dream_cream = Color::from_rgba(160, 140, 190, 255);     // muted cream
        let dream_wood  = Color::from_rgba(120, 100, 140, 255);     // muted wood
        let dream_dark  = Color::from_rgba(60, 50, 80, 255);        // muted dark
        return match tile {
            Tile::Grass     => dream_grass,
            Tile::Path      => Color::from_rgba(180, 160, 200, 255), // misty path
            Tile::Water     => dream_water,
            Tile::Tree      => dream_grass,
            Tile::Flower    => dream_grass,
            Tile::Fence     => dream_grass,
            Tile::Sign      => dream_grass,
            Tile::Chest     => dream_grass,
            Tile::Bridge    => dream_water,
            Tile::Wall      => dream_dark,
            Tile::HouseWall => dream_cream,
            Tile::Roof      => Color::from_rgba(130, 80, 140, 255),  // muted plum
            Tile::Door      => dream_cream,
            Tile::Window    => dream_cream,
            Tile::WoodFloor => dream_wood,
            Tile::Rug       => dream_wood,
            Tile::Table     => dream_dark,
            Tile::Bookshelf => dream_dark,
            Tile::SeaFloor  => dream_water,
            Tile::Sand      => dream_cream,
            Tile::Coral     => Color::from_rgba(120, 80, 140, 255),  // muted plum coral
            Tile::Kelp      => dream_grass,
            Tile::Bubble    => dream_water,
            Tile::DiveSpot  => dream_water,
            Tile::Space | Tile::Star => dream_dark,
            Tile::SpaceRock => dream_dark,
            Tile::Launchpad | Tile::MoonPad | Tile::MarsPad | Tile::AsteroidPad
                            => dream_cream,
            Tile::MoonGround | Tile::MarsGround | Tile::StationFloor => dream_grass,
            Tile::Glitch95 | Tile::Glitch96 | Tile::Glitch97 | Tile::Glitch98
                            => dream_dark,
            Tile::GlitchWall => dream_dark,
        };
    }

    if mode == RenderMode::Aquatic {
        return tile_color_aquatic(tile);
    }

    if mode == RenderMode::Cosmic {
        return tile_color_cosmic(tile);
    }

    tile_color_normal(tile)
}

/// Space palette. Dark void with per-world ground colors so the hub, Moon,
/// Mars and the station each read distinctly under one Cosmic render mode.
fn tile_color_cosmic(tile: Tile) -> Color {
    let void = Color::from_rgba(10, 12, 28, 255);     // deep space
    match tile {
        Tile::Space        => void,
        Tile::Star         => void,
        Tile::SpaceRock    => Color::from_rgba(58, 54, 74, 255),    // asteroid rock
        Tile::Launchpad    => Color::from_rgba(40, 44, 70, 255),    // pad base
        Tile::MoonPad      => Color::from_rgba(40, 44, 70, 255),
        Tile::MarsPad      => Color::from_rgba(40, 44, 70, 255),
        Tile::AsteroidPad  => Color::from_rgba(40, 44, 70, 255),
        Tile::MoonGround   => Color::from_rgba(120, 120, 130, 255), // lunar gray
        Tile::MarsGround   => Color::from_rgba(160, 78, 54, 255),   // rusty red
        Tile::StationFloor => Color::from_rgba(70, 78, 96, 255),    // metal deck
        // Land tiles a future cosmic map might reuse fade into the void.
        _                  => void,
    }
}

/// Underwater palette. Land tiles still appear (a map can reuse Grass/Path),
/// repainted in teal so any tile reads as "submerged"; the dedicated SeaFloor /
/// Coral / Kelp tiles carry the reef's real look.
fn tile_color_aquatic(tile: Tile) -> Color {
    let deep   = Color::from_rgba(13, 71, 102, 255);    // deep teal water
    let floor  = Color::from_rgba(38, 120, 140, 255);   // lit sea floor
    let sand   = Color::from_rgba(214, 205, 160, 255);  // pale sand
    let coral  = Color::from_rgba(255, 111, 97, 255);   // warm coral
    let kelp   = Color::from_rgba(34, 110, 80, 255);    // green kelp
    match tile {
        Tile::SeaFloor  => floor,
        Tile::Sand      => sand,
        Tile::Coral     => coral,
        Tile::Kelp      => kelp,
        Tile::Bubble    => floor,
        Tile::Water     => deep,
        Tile::Grass     => floor,
        Tile::Path      => sand,
        Tile::Bridge    => sand,
        // Anything else a future water map drops in tints toward deep water so
        // it never flashes a jarring land color underwater.
        _               => deep,
    }
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
        Tile::SeaFloor  => Color::from_rgba(38, 120, 140, 255),    // sea floor
        Tile::Sand      => Color::from_rgba(214, 205, 160, 255),   // sand
        Tile::Coral     => Color::from_rgba(255, 111, 97, 255),    // coral
        Tile::Kelp      => Color::from_rgba(34, 110, 80, 255),     // kelp
        Tile::Bubble    => Color::from_rgba(38, 120, 140, 255),    // bubble vent (sea floor base)
        Tile::DiveSpot  => Color::from_rgba(66, 165, 245, 255),    // dive spot (water base)
        Tile::Space | Tile::Star => Color::from_rgba(10, 12, 28, 255),
        Tile::SpaceRock => Color::from_rgba(58, 54, 74, 255),
        Tile::Launchpad | Tile::MoonPad | Tile::MarsPad | Tile::AsteroidPad
                        => Color::from_rgba(40, 44, 70, 255),
        Tile::MoonGround => Color::from_rgba(120, 120, 130, 255),
        Tile::MarsGround => Color::from_rgba(160, 78, 54, 255),
        Tile::StationFloor => Color::from_rgba(70, 78, 96, 255),
        Tile::Glitch95 | Tile::Glitch96 | Tile::Glitch97 | Tile::Glitch98
                        => Color::from_rgba(50, 50, 50, 255),      // glitch tiles
        Tile::GlitchWall => Color::from_rgba(50, 50, 50, 255),     // glitch wall
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

    // Underwater bubbles + a soft blue light tint
    if map.render_mode == RenderMode::Aquatic {
        draw_aquatic_overlay(cam_x, cam_y, view_w, view_h, time);
    }

    // Twinkling starfield drifting over space
    if map.render_mode == RenderMode::Cosmic {
        draw_cosmic_overlay(cam_x, cam_y, view_w, view_h, time);
    }
}

fn draw_cosmic_overlay(cam_x: f32, cam_y: f32, view_w: f32, view_h: f32, time: f32) {
    for i in 0..40 {
        let seed = i as f32 * 137.5; // golden-angle spread
        let sx = cam_x + ((seed * 7.3).sin() * 0.5 + 0.5) * view_w;
        let sy = cam_y + ((seed * 13.1).cos() * 0.5 + 0.5) * view_h;
        let tw = ((time * 2.0 + seed).sin() * 0.5 + 0.5).powf(2.0);
        let size = 0.6 + tw * 1.6;
        draw_circle(sx, sy, size, Color::new(1.0, 1.0, 0.95, 0.25 + tw * 0.6));
    }
}

fn draw_aquatic_overlay(cam_x: f32, cam_y: f32, view_w: f32, view_h: f32, time: f32) {
    // Cool blue depth tint over everything.
    draw_rectangle(cam_x, cam_y, view_w, view_h, Color::new(0.05, 0.35, 0.55, 0.12));
    // Bubbles drifting up across the viewport.
    for i in 0..24 {
        let seed = i as f32 * 137.5; // golden-angle spread
        let bx = cam_x + ((seed * 7.3).sin() * 0.5 + 0.5) * view_w
            + (time * 0.6 + seed).sin() * 6.0;
        let rise = ((time * 0.25 + seed * 0.13) % 1.0).abs();
        let by = cam_y + view_h - rise * view_h;
        let size = 1.5 + (seed * 0.5).sin().abs() * 3.0;
        draw_circle(bx, by, size, Color::new(0.85, 0.95, 1.0, 0.20));
        draw_circle_lines(bx, by, size, 1.0, Color::new(0.85, 0.95, 1.0, 0.30));
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
        Tile::SeaFloor  => draw_seafloor_detail(x, y),
        Tile::Sand      => draw_sand_detail(x, y),
        Tile::Coral     => draw_coral_detail(x, y, time),
        Tile::Kelp      => draw_kelp_detail(x, y, time),
        Tile::Bubble    => draw_bubble_vent_detail(x, y, time),
        Tile::DiveSpot  => draw_dive_spot_detail(x, y, time),
        Tile::Star      => draw_star_detail(x, y, time),
        Tile::SpaceRock => draw_space_rock_detail(x, y),
        Tile::Launchpad => draw_launchpad_detail(x, y, time),
        Tile::MoonPad   => draw_planet_pad_detail(x, y, time, Color::from_rgba(200, 200, 210, 255)),
        Tile::MarsPad   => draw_planet_pad_detail(x, y, time, Color::from_rgba(230, 110, 70, 255)),
        Tile::AsteroidPad => draw_planet_pad_detail(x, y, time, Color::from_rgba(150, 140, 120, 255)),
        Tile::MoonGround => draw_moon_ground_detail(x, y),
        Tile::MarsGround => draw_mars_ground_detail(x, y),
        Tile::StationFloor => draw_station_floor_detail(x, y),
        _               => {}
    }
}

fn draw_star_detail(x: f32, y: f32, time: f32) {
    let cx = x + TILE_SIZE / 2.0;
    let cy = y + TILE_SIZE / 2.0;
    let tw = (time * 2.0 + x * 0.3 + y * 0.2).sin() * 0.5 + 0.5;
    let r = 2.0 + tw * 2.5;
    let c = Color::new(1.0, 1.0, 0.9, 0.5 + tw * 0.5);
    draw_circle(cx, cy, r, c);
    // sparkle cross
    draw_line(cx - r - 2.0, cy, cx + r + 2.0, cy, 1.0, Color::new(1.0, 1.0, 1.0, tw * 0.6));
    draw_line(cx, cy - r - 2.0, cx, cy + r + 2.0, 1.0, Color::new(1.0, 1.0, 1.0, tw * 0.6));
}

fn draw_space_rock_detail(x: f32, y: f32) {
    // Cratered asteroid lumps
    let dark = Color::from_rgba(40, 38, 54, 255);
    let light = Color::from_rgba(78, 74, 96, 255);
    for i in 0..3 {
        let rx = seeded_random(x, y, i * 5) * (TILE_SIZE - 14.0) + 7.0;
        let ry = seeded_random(x, y, i * 5 + 1) * (TILE_SIZE - 14.0) + 7.0;
        draw_circle(x + rx, y + ry, 4.0, light);
        draw_circle(x + rx + 1.0, y + ry + 1.0, 2.0, dark);
    }
}

fn draw_launchpad_detail(x: f32, y: f32, time: f32) {
    let cx = x + TILE_SIZE / 2.0;
    let cy = y + TILE_SIZE / 2.0;
    // Hazard ring
    draw_circle(cx, cy, 18.0, Color::from_rgba(255, 193, 7, 200));
    draw_circle(cx, cy, 14.0, Color::from_rgba(40, 44, 70, 255));
    // Pulsing "H" pad lights
    let pulse = (time * 3.0).sin() * 0.5 + 0.5;
    let lc = Color::new(0.4, 0.9, 1.0, 0.5 + pulse * 0.5);
    draw_line(cx - 6.0, cy - 7.0, cx - 6.0, cy + 7.0, 2.5, lc);
    draw_line(cx + 6.0, cy - 7.0, cx + 6.0, cy + 7.0, 2.5, lc);
    draw_line(cx - 6.0, cy, cx + 6.0, cy, 2.5, lc);
}

fn draw_planet_pad_detail(x: f32, y: f32, time: f32, planet: Color) {
    let cx = x + TILE_SIZE / 2.0;
    let cy = y + TILE_SIZE / 2.0;
    // Glowing landing ring
    let pulse = (time * 2.0 + x * 0.1).sin() * 0.5 + 0.5;
    draw_circle_lines(cx, cy, 20.0, 2.0, Color::new(0.5, 0.8, 1.0, 0.3 + pulse * 0.4));
    // The destination planet floating above the pad
    let bob = (time * 1.5).sin() * 2.0;
    draw_circle(cx, cy - 2.0 + bob, 11.0, planet);
    // little shading crescent
    draw_circle(cx + 4.0, cy - 4.0 + bob, 7.0, Color::new(1.0, 1.0, 1.0, 0.12));
}

fn draw_moon_ground_detail(x: f32, y: f32) {
    let crater = Color::from_rgba(98, 98, 108, 255);
    for i in 0..3 {
        let rx = seeded_random(x, y, i * 4) * (TILE_SIZE - 12.0) + 6.0;
        let ry = seeded_random(x, y, i * 4 + 1) * (TILE_SIZE - 12.0) + 6.0;
        let rr = 2.0 + seeded_random(x, y, i * 4 + 2) * 3.0;
        draw_circle_lines(x + rx, y + ry, rr, 1.5, crater);
    }
}

fn draw_mars_ground_detail(x: f32, y: f32) {
    let dust = Color::from_rgba(140, 64, 44, 255);
    for i in 0..4 {
        let rx = seeded_random(x, y, i * 6) * (TILE_SIZE - 8.0) + 4.0;
        let ry = seeded_random(x, y, i * 6 + 1) * (TILE_SIZE - 8.0) + 4.0;
        draw_circle(x + rx, y + ry, 1.8, dust);
    }
}

fn draw_station_floor_detail(x: f32, y: f32) {
    // Panel grid + rivets
    let line = Color::from_rgba(54, 60, 76, 255);
    draw_rectangle_lines(x + 2.0, y + 2.0, TILE_SIZE - 4.0, TILE_SIZE - 4.0, 1.0, line);
    let rivet = Color::from_rgba(96, 104, 124, 255);
    draw_circle(x + 6.0, y + 6.0, 1.3, rivet);
    draw_circle(x + TILE_SIZE - 6.0, y + 6.0, 1.3, rivet);
    draw_circle(x + 6.0, y + TILE_SIZE - 6.0, 1.3, rivet);
    draw_circle(x + TILE_SIZE - 6.0, y + TILE_SIZE - 6.0, 1.3, rivet);
}

/// A swirling whirlpool that visibly says "dive in here." Concentric rotating
/// rings over a darker water pool, with a downward arrow hint.
fn draw_dive_spot_detail(x: f32, y: f32, time: f32) {
    let cx = x + TILE_SIZE / 2.0;
    let cy = y + TILE_SIZE / 2.0;
    // Darker pool so it reads as deeper than surrounding water.
    draw_circle(cx, cy, 22.0, Color::from_rgba(30, 110, 190, 255));
    draw_circle(cx, cy, 16.0, Color::from_rgba(20, 90, 170, 255));
    // Swirl: dots spiralling inward, rotating over time.
    let foam = Color::from_rgba(190, 230, 255, 230);
    for i in 0..10 {
        let t = i as f32 / 10.0;
        let ang = time * 2.0 + t * std::f32::consts::TAU * 1.6;
        let r = 4.0 + t * 16.0;
        draw_circle(cx + ang.cos() * r, cy + ang.sin() * r, 1.6, foam);
    }
    // Gentle downward arrow at the center — "go down".
    let bob = (time * 2.0).sin() * 2.0;
    let ac = Color::from_rgba(220, 245, 255, 235);
    draw_triangle(
        vec2(cx - 5.0, cy - 3.0 + bob),
        vec2(cx + 5.0, cy - 3.0 + bob),
        vec2(cx, cy + 5.0 + bob),
        ac,
    );
}

fn draw_seafloor_detail(x: f32, y: f32) {
    // Scattered pebbles / shells on the lit floor
    let pebble = Color::from_rgba(26, 95, 112, 255);
    for i in 0..4 {
        let rx = seeded_random(x, y, i * 3) * (TILE_SIZE - 8.0) + 4.0;
        let ry = seeded_random(x, y, i * 3 + 1) * (TILE_SIZE - 8.0) + 4.0;
        draw_circle(x + rx, y + ry, 2.0, pebble);
    }
}

fn draw_sand_detail(x: f32, y: f32) {
    // Ripple lines in the sand
    let ripple = Color::from_rgba(196, 186, 138, 255);
    for i in 0..3 {
        let ly = y + 12.0 + i as f32 * 14.0;
        draw_line(x + 4.0, ly, x + TILE_SIZE - 4.0, ly + 2.0, 1.0, ripple);
    }
}

fn draw_coral_detail(x: f32, y: f32, time: f32) {
    // Branching coral on a sea-floor base
    draw_rectangle(x, y, TILE_SIZE, TILE_SIZE, Color::from_rgba(38, 120, 140, 255));
    let sway = (time * 1.2 + x * 0.2).sin() * 1.0;
    let colors = [
        Color::from_rgba(255, 111, 97, 255),
        Color::from_rgba(255, 159, 128, 255),
        Color::from_rgba(244, 143, 177, 255),
    ];
    for i in 0..3 {
        let bx = x + 10.0 + i as f32 * 12.0;
        draw_line(bx, y + TILE_SIZE - 4.0, bx + sway, y + 18.0 - i as f32 * 2.0, 4.0, colors[i as usize % 3]);
        draw_circle(bx + sway, y + 16.0 - i as f32 * 2.0, 4.0, colors[i as usize % 3]);
    }
}

fn draw_kelp_detail(x: f32, y: f32, time: f32) {
    // Tall swaying kelp fronds
    let base = Color::from_rgba(34, 110, 80, 255);
    let frond = Color::from_rgba(46, 140, 100, 255);
    for i in 0..3 {
        let bx = x + 10.0 + i as f32 * 14.0;
        let sway = (time * 1.5 + i as f32 * 1.3 + x * 0.1).sin() * 5.0;
        draw_line(bx, y + TILE_SIZE, bx + sway, y + 4.0, 5.0, base);
        draw_line(bx, y + TILE_SIZE, bx + sway * 0.7, y + 4.0, 2.5, frond);
    }
}

fn draw_bubble_vent_detail(x: f32, y: f32, time: f32) {
    // Sea floor base with a stream of rising bubbles
    draw_seafloor_detail(x, y);
    let bubble = Color::from_rgba(220, 245, 255, 160);
    for i in 0..4 {
        let phase = (time * 0.8 + i as f32 * 0.45 + seeded_random(x, y, i) * 3.0) % 1.0;
        let bx = x + 12.0 + seeded_random(x, y, i + 7) * 24.0;
        let by = y + TILE_SIZE - phase * (TILE_SIZE - 6.0);
        draw_circle(bx, by, 1.5 + phase * 2.0, bubble);
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
