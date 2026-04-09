use macroquad::prelude::*;
use crate::sprites;
use crate::tilemap::TILE_SIZE;

#[derive(Clone, Copy, PartialEq)]
pub enum SpriteType {
    Mommy,
    Sage,
    Dog,
    Kid1,
    Kid2,
    OldOak,
}

#[derive(Clone)]
pub struct Npc {
    pub id: &'static str,
    pub name: &'static str,
    pub tile_x: usize,
    pub tile_y: usize,
    pub sprite: SpriteType,
    pub can_receive_gifts: bool,
    pub never_challenge: bool,
}

impl Npc {
    pub fn draw(&self, time: f32) {
        let x = self.tile_x as f32 * TILE_SIZE;
        let y = self.tile_y as f32 * TILE_SIZE;
        match self.sprite {
            SpriteType::Mommy => sprites::npcs::draw_mommy(x, y, time),
            SpriteType::Sage => sprites::npcs::draw_sage(x, y, time),
            SpriteType::Dog => sprites::npcs::draw_dog(x, y, time),
            SpriteType::Kid1 => sprites::npcs::draw_kid(x, y,
                Color::from_rgba(255, 112, 67, 255),  // orange hair
                Color::from_rgba(253, 216, 53, 255),   // yellow shirt
                true, time),
            SpriteType::Kid2 => sprites::npcs::draw_kid(x, y,
                Color::from_rgba(109, 76, 65, 255),    // brown hair
                Color::from_rgba(102, 187, 106, 255),  // green shirt
                false, time),
            SpriteType::OldOak => sprites::npcs::draw_old_oak(x, y, time),
        }
    }

    pub fn pixel_y(&self) -> f32 {
        self.tile_y as f32 * TILE_SIZE
    }
}

pub fn npcs_for_map(map_id: &str) -> Vec<Npc> {
    match map_id {
        "overworld" => vec![
            Npc { id: "sage", name: "Professor Gizmo", tile_x: 12, tile_y: 12,
                sprite: SpriteType::Sage, can_receive_gifts: true, never_challenge: false },
        ],
        "home" => vec![
            Npc { id: "mommy", name: "Mommy", tile_x: 3, tile_y: 3,
                sprite: SpriteType::Mommy, can_receive_gifts: true, never_challenge: false },
            Npc { id: "kid_1", name: "Tali", tile_x: 6, tile_y: 5,
                sprite: SpriteType::Kid1, can_receive_gifts: true, never_challenge: true },
            Npc { id: "kid_2", name: "Noa", tile_x: 8, tile_y: 5,
                sprite: SpriteType::Kid2, can_receive_gifts: true, never_challenge: true },
        ],
        "lab" => vec![
            Npc { id: "sage_lab", name: "Professor Gizmo", tile_x: 5, tile_y: 3,
                sprite: SpriteType::Sage, can_receive_gifts: true, never_challenge: false },
        ],
        "shop" => vec![
            Npc { id: "shopkeeper", name: "Bolt the Shopkeeper", tile_x: 5, tile_y: 2,
                sprite: SpriteType::Sage, can_receive_gifts: true, never_challenge: false },
        ],
        "dream" => vec![
            Npc { id: "dream_sage", name: "???", tile_x: 15, tile_y: 8,
                sprite: SpriteType::Sage, can_receive_gifts: false, never_challenge: false },
        ],
        "doghouse" => vec![
            Npc { id: "glitch_dog", name: "B0RK.exe", tile_x: 7, tile_y: 5,
                sprite: SpriteType::Dog, can_receive_gifts: true, never_challenge: false },
        ],
        "grove" => vec![
            Npc { id: "grove_spirit", name: "Old Oak", tile_x: 6, tile_y: 4,
                sprite: SpriteType::OldOak, can_receive_gifts: true, never_challenge: false },
        ],
        _ => vec![],
    }
}

/// Check if the player is facing an NPC and return it
pub fn get_interact_target(
    player_tx: usize, player_ty: usize, dir: sprites::Dir, npcs: &[Npc],
) -> Option<&Npc> {
    let (tx, ty) = match dir {
        sprites::Dir::Up => (player_tx as i32, player_ty as i32 - 1),
        sprites::Dir::Down => (player_tx as i32, player_ty as i32 + 1),
        sprites::Dir::Left => (player_tx as i32 - 1, player_ty as i32),
        sprites::Dir::Right => (player_tx as i32 + 1, player_ty as i32),
    };
    if tx < 0 || ty < 0 { return None; }
    npcs.iter().find(|n| n.tile_x == tx as usize && n.tile_y == ty as usize)
}

/// Check if facing Sparky (the robot)
pub fn is_facing_sparky(
    player_tx: usize, player_ty: usize, dir: sprites::Dir,
    sparky_tx: usize, sparky_ty: usize,
) -> bool {
    let (tx, ty) = match dir {
        sprites::Dir::Up => (player_tx as i32, player_ty as i32 - 1),
        sprites::Dir::Down => (player_tx as i32, player_ty as i32 + 1),
        sprites::Dir::Left => (player_tx as i32 - 1, player_ty as i32),
        sprites::Dir::Right => (player_tx as i32 + 1, player_ty as i32),
    };
    tx >= 0 && ty >= 0 && tx as usize == sparky_tx && ty as usize == sparky_ty
}
