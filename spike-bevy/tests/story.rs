//! ADR-002-style story test on Bevy: no window, no GPU, plain `cargo test`.
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use spike_bevy::*;

struct Harness { app: App }

impl Harness {
    fn new() -> Self {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(SpikeLogicPlugin { seed: 42 });
        app.update();
        Harness { app }
    }
    fn press(&mut self, key: KeyCode) {
        self.app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(key);
        self.app.update();
        let mut keys = self.app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
        self.app.update();
    }
    fn walk_to_sparky(&mut self) {
        for _ in 0..MAP_W {
            if self.mode() == Mode::Challenge { return; }
            self.press(KeyCode::ArrowRight);
        }
    }
    fn mode(&self) -> Mode { *self.app.world().resource::<State<Mode>>().get() }
    fn pick(&mut self, correct: bool) {
        let world = self.app.world_mut();
        let button = world
            .query::<(Entity, &ChoiceButton)>()
            .iter(world)
            .find(|(_, b)| b.correct == correct)
            .map(|(e, _)| e)
            .expect("a choice button");
        world.trigger(ChoicePicked { entity: button });
        self.app.update();
    }
    fn answer_correctly(&mut self) { self.pick(true) }
    fn answer_wrong(&mut self) { self.pick(false) }
    fn events(&self) -> &[SpikeEvent] { &self.app.world().resource::<EventLog>().0 }
    fn count<C: Component>(&mut self) -> usize {
        let world = self.app.world_mut();
        world.query_filtered::<(), With<C>>().iter(world).count()
    }
}

#[test]
fn bumping_sparky_opens_a_challenge_and_a_right_answer_closes_it() {
    let mut g = Harness::new();
    g.walk_to_sparky();
    assert_eq!(g.mode(), Mode::Challenge);
    assert!(matches!(g.events()[0], SpikeEvent::ChallengeStarted { .. }));
    assert!(g.count::<ChoiceButton>() >= 2);

    g.answer_correctly();
    assert_eq!(g.mode(), Mode::Exploring);
    assert_eq!(g.events().last(), Some(&SpikeEvent::ChallengeResolved { correct: true }));
    assert_eq!(g.count::<ChoiceButton>(), 0, "panel despawns on leaving the state");
}

#[test]
fn a_wrong_answer_nudges_and_keeps_the_challenge_open() {
    let mut g = Harness::new();
    g.walk_to_sparky();
    g.answer_wrong();
    assert_eq!(g.mode(), Mode::Challenge);
    assert_eq!(g.count::<Nudge>(), 1);
    g.answer_correctly();
    assert_eq!(g.count::<Nudge>(), 0);
}

#[test]
fn same_seed_same_question() {
    let q = |_| { let mut g = Harness::new(); g.walk_to_sparky(); format!("{:?}", g.events()[0]) };
    assert_eq!(q(0), q(1));
}
