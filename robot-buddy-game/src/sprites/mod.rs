pub mod player;
pub mod robot;
pub mod npcs;

#[derive(Clone, Copy, PartialEq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}
