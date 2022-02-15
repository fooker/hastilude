use std::collections::HashSet;

use tracing::debug;

use crate::engine::players::{PlayerData, PlayerId};
use crate::engine::state::{State, World};
use crate::games::joust::Joust;
use crate::games::meta::countdown::{Countdown, PlayerColor};
use crate::games::meta::debug::Debug;

pub mod meta;
pub mod joust;

pub trait Game: State + Sized + 'static {
    type Data;

    fn create(players: HashSet<PlayerId>, world: &mut World) -> Self;

    fn data(&mut self) -> &mut PlayerData<Self::Data>;
}

#[derive(Debug, Copy, Clone)]
pub enum GameType {
    Debug,
    Joust,
}

fn start<T>(players: HashSet<PlayerId>, world: &mut World) -> impl State
    where T: Game,
          T::Data: PlayerColor {
    let game = T::create(players, world);
    debug!("Game created");

    return Countdown::new(game, world);
}

impl GameType {
    pub fn create(self, players: HashSet<PlayerId>, world: &mut World) -> Box<dyn State> {
        return match self {
            Self::Debug => Box::new(Debug::new(world)),
            Self::Joust => Box::new(start::<Joust>(players, world)),
        };
    }
}

