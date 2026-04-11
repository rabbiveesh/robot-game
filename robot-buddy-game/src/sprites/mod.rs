pub mod player;
pub mod robot;
pub mod npcs;

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    /// Convert from legacy u8 (0=up, 1=down, 2=left, 3=right) for old save compat.
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Dir::Up,
            1 => Dir::Down,
            2 => Dir::Left,
            _ => Dir::Right,
        }
    }
}
