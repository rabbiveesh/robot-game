//! What each NPC says when the kid walks up and talks to them: a small
//! table of lines per NPC kind, one picked at random per chat.

use ::rand::{Rng, rngs::SmallRng};

use super::Npc;
use crate::ui::dialogue::DialogueLine;

/// One line of small talk from `npc`, picked with the seeded `rng`.
pub(crate) fn npc_dialogue_lines(npc: &Npc, rng: &mut SmallRng) -> Vec<DialogueLine> {
    use super::NpcKind::*;
    let lines: &[&str] = match npc.kind {
        Mommy => &[
            "Hi sweetie! I'm so proud of you for exploring!",
            "You and Sparky make the best team!",
            "I love you! Keep being amazing!",
        ],
        Sage | SageLab => &[
            "Ahhhh, young adventurer! The stars told me you'd come!",
            "Welcome! I am Professor Gizmo, master of numbers!",
            "The ancient scrolls speak of a hero... and I think it's YOU!",
        ],
        Kid1 => &[
            "Wanna see me do a cartwheel? Watch! ...okay I can't actually do one yet.",
            "Sparky is SO COOL! I wish I had a robot friend!",
            "Did you know frogs can jump SUPER far? Like, really far!",
        ],
        Kid2 => &[
            "Hi... um... do you like bugs? I found a really cool one.",
            "Sparky beeped at me and I think that means he likes me!",
            "Do you think clouds are soft? I think they're soft.",
        ],
        Shopkeeper => &[
            "Welcome to my shop! Everything costs Dum Dums!",
            "I've got the finest wares in all of Robot Village!",
        ],
        DreamSage => &[
            "You are dreaming... or are you? The numbers whisper here...",
            "In dreams, 2 + 2 can be anything... but it's still 4.",
        ],
        GlitchDog => &[
            "BORK BORK! sys.treat.exe... GOOD BOY overflow!",
            "Woof! *static* I am... a good boy? BORK.dll loaded!",
            "fetch(ball) returned: UNDEFINED... but I still love you!",
        ],
        GroveSpirit => &[
            "How... did you find this place? The trees have hidden it for ages...",
            "It's dangerous to go alone... take this!",
            "The leaves whisper your name... they say you are very clever.",
        ],
        Pip => &[
            "Squeak! You found my little clearing!",
            "I like to wander in circles. It's very fun!",
            "Got any snacks? I'm always a bit hungry, hehe!",
        ],
        Signpost => &[
            "Howdy! I've pointed the way for YEARS. Bit lonely, though.",
            "Psst... give a fella a Dum Dum and I'll come adventuring with you!",
            "I know ALL the directions. Left, right, up... and the other one!",
        ],
        // ReefShark normally reaches the player through the gate-challenge path,
        // not here — but if you chat after he's stepped aside, he's a sweetie.
        ReefShark => &[
            "Thanks for the puzzle, pal! Naps are better after a good brain stretch.",
            "Toothy grin, gentle heart. That's me!",
            "Swim on through, the cove's all yours!",
        ],
        SeaTurtle => &[
            "Greetings, little diver. I've ridden these currents a hundred years.",
            "Slow and steady finds the most pearls, you know.",
            "The coral grows a tiny bit every day. Just like you!",
        ],
        Dolphin => &[
            "Eee-eee! Give me a Dum Dum and you can ride on my back! Zoooom!",
            "Wanna race? I'll give you a head start! ...okay maybe two!",
            "Did you see my flip? I've been practicing!",
            "Bubbles are the BEST. Watch — bloop bloop bloop!",
        ],
        Crab => &[
            "Snip snap! Mind the claws, I'm just saying hi!",
            "Sideways is the only way to walk, obviously.",
            "I keep the sand tidy around here. Very important job.",
        ],
        Jelly => &[
            "...blub... (the jellyfish wobbles a friendly hello)",
            "Drifting is a perfectly good plan, thank you very much.",
            "Don't worry, I'm the no-sting kind!",
        ],
        Octopus => &[
            "Want to see the trench? Take the shaft — but you have to land RIGHT on the door!",
            "Kick down in big kicks or little ones. Five and five and two, that sort of thing!",
            "Mind the rock ledges — you can't rest on those. Eight arms and I still bonk them.",
        ],
        Clam => &[
            "Brrbl! Pull me back and let go — I'll FLY to my pearl! Wheee!",
            "Too far? Sploosh! I don't mind a swim. Too short? I'll just hop back!",
            "I always hop the same size. Three, six, nine... that's MY kind of counting!",
            "Every time you find my pearl, I hide it again. It's my favorite game!",
        ],
        Anglerfish => &[
            "Like my light? I grew it myself! It's for finding pearls... and friends!",
            "Down here the dark is friendly — especially with a lamp on your head!",
            "If you get lost in the trench, just follow the glow. That's me!",
        ],
        Eel => &[
            "Wiggle wiggle! I know every crack and cranny in this trench!",
            "Did somebody say TREASURE? There's a chest past the vents, you know.",
            "I'm not slimy, I'm streamlined!",
        ],
        HermitCrab => &[
            "Pssst! Down here! I carry my whole shop on my back, see?",
            "Pearls only, friend. Dum Dums are for surface folk!",
            "Got pearls? I've got kelp crowns, and a net that finds you MORE pearls.",
            "Three pearls make a Dum Dum. That's the going rate and I'll not budge.",
        ],
        TurtleElder => &[
            "Come in, come in, little swimmer! Mind the kettle vent, it bubbles.",
            "I've lived on this reef two hundred years. The numbered stones? I helped lay them!",
            "Rest your fins a moment, dear. Adventuring is hungry work.",
        ],
        MoonAlien => &[
            "Zorp! You bounced all the way to the Moon! Boing boing!",
            "Low gravity is the BEST. Watch me jump super high! Wheee!",
            "I collect moon rocks. Wanna see? I have... a LOT.",
        ],
        // FuelBot reaches the player through the refuel-challenge path, not here.
        FuelBot => &[
            "BEEP. Tank online. Solve my puzzle and I'll top off your rocket!",
            "Fuel is friendship. ...no wait, that's not right. BEEP.",
        ],
        // MarsGuardian normally reaches the player via the gate path; this is
        // for after he's waved them through.
        MarsGuardian => &[
            "Course plotted! Safe travels, little astronaut. Rok approves.",
            "The cove's all yours now. Mind the red dust!",
            "Numbers are the best maps. You read them like a pro!",
        ],
        StarKeeper => &[
            "Welcome to the star chart, navigator! Spot the pattern in the stars?",
            "Every constellation hides a sequence. Can you finish it?",
            "Cassi has mapped a thousand skies. Today we map one together!",
        ],
        StationAlien => &[
            "Bleep bloop! A visitor! It's been AGES since anyone docked here!",
            "I keep the station tidy. Floating crumbs are a real problem.",
            "Did you know space has no up or down? My feet sure don't.",
        ],
        // Blaster Bubbe normally launches the shooter on interact; this line is
        // only a fallback so the match stays exhaustive.
        ArcadeAlien => &[
            "Bubeleh! Step right up to the cabinet and blast some number bonds!",
        ],
        // Dev-control NPCs go through apply_dev_control, never this path.
        CtrlBand | CtrlKenkenLevel | CtrlCraReset | CtrlIntroReset
        | CtrlTriggerKenken | CtrlTriggerPattern | CtrlTriggerBalance
        | CtrlTriggerSudoku | CtrlTriggerChallenge
        | CtrlToggleEncounters | CtrlTriggerEncounter
        | CtrlToggleQuest | CtrlStartQuest => &["Hello there!"],
    };
    let idx = rng.gen_range(0..lines.len());
    vec![DialogueLine { speaker: npc.name().into(), text: lines[idx].into() }]
}
