//! Every buddy on screen wears what they've been given: the only
//! caller of `draw_swag` is `sprites::dressed` (plus the kid's own cosmetics
//! in `sprites::player`), and no game or UI file draws an NPC or Sparky body
//! directly. A new screen that shows a buddy has to go through
//! `dressed::draw_dressed`, so it can't forget what they're wearing.

use std::path::Path;

#[test]
fn characters_are_drawn_dressed() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = path.strip_prefix(&src).unwrap().to_string_lossy().replace('\\', "/");
            if !rel.ends_with(".rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for (needle, allowed) in [
                ("draw_swag(", &["sprites/swag.rs", "sprites/dressed.rs", "sprites/player.rs"][..]),
                ("draw_sprite(", &["npc.rs", "sprites/dressed.rs"][..]),
            ] {
                if text.contains(needle) && !allowed.contains(&rel.as_str()) {
                    offenders.push(format!("{rel} calls {needle}..)"));
                }
            }
        }
    }
    assert!(offenders.is_empty(), "draw characters with sprites::dressed::draw_dressed:\n  {}", offenders.join("\n  "));
    let hop = std::fs::read_to_string(src.join("ui/pearl_hop.rs")).unwrap();
    assert!(hop.contains("dressed::draw_dressed"), "Pearl Hop draws Shelly dressed");
}
