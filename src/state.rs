use std::time::Duration;

use crate::engine::World;
use crate::games::Game;
use crate::meta::celebration::Celebration;
use crate::meta::countdown::Countdown;
use crate::meta::lobby::Lobby;
use crate::engine::players::Players;

pub enum State {
    Lobby(Lobby),
    Countdown(Countdown),
    Playing(Box<dyn Game>),
    Celebration(Celebration),
}

impl State {
    pub fn lobby(players: &mut Players) -> Self {
        return Self::Lobby(Lobby::new(players));
    }

    pub fn update(self, world: &mut World, duration: Duration) -> Self {
        return match self {
            State::Lobby(lobby) => lobby.update(world),
            State::Countdown(countdown) => countdown.update(world, duration),
            State::Playing(game) => game.update(world, duration),
            State::Celebration(celebration) => celebration.update(world, duration),
        };
    }

    pub fn cancel(self, world: &mut World) -> Self {
        return match self {
            State::Lobby(_) => self,
            State::Countdown(_) => Self::lobby(world.players),
            State::Playing(_) => Self::lobby(world.players),
            State::Celebration(_) => self,
        }
    }
}
