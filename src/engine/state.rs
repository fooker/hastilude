use std::time::{Duration, Instant};

use replace_with::replace_with_or_abort;
use tracing::instrument;

use crate::engine::assets::Assets;
use crate::engine::players::Players;
use crate::engine::sound::Sound;
use crate::games::GameType;

pub struct World<'a> {
    // The currently selected game
    pub game: GameType,

    // Current time of the frame
    pub now: Instant,

    pub players: &'a mut Players,

    pub sound: &'a mut Sound,

    pub assets: &'a Assets,
}

pub trait State {
    fn update(self: Box<Self>, world: &mut World, duration: Duration) -> Box<dyn State>;
}

pub struct StateMachine {
    state: Box<dyn State>,
}

impl StateMachine {
    pub fn new<S>(state: S) -> Self
        where
            S: State + 'static,
    {
        return Self {
            state: Box::new(state),
        };
    }

    #[instrument(level = "trace", skip(self, world))]
    pub fn update(&mut self, world: &mut World, duration: Duration) {
        replace_with_or_abort(&mut self.state, |state| state.update(world, duration));
    }
}