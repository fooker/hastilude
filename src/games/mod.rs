use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::engine::players::{PlayerData, PlayerId};
use crate::engine::World;
use crate::games::debug::Debug;
use crate::games::joust::Joust;
use crate::meta::countdown::{Countdown, PlayerColor};
use crate::state::State;

pub mod debug;
pub mod joust;

pub trait GameData: Game {
    type Data;

    fn data(&mut self) -> &mut PlayerData<Self::Data>;

    fn create(players: HashSet<PlayerId>, world: &mut World) -> Self
        where Self: Sized;

    fn kick_player(&mut self, _player: PlayerId, _world: &mut World) -> bool {
        todo!()
    }
}

pub trait Game {
    fn update(self: Box<Self>, world: &mut World, duration: Duration) -> State;

    /// Removes a player form the game. Returns whether the player was part of the game.
    fn kick_player(&mut self, player: PlayerId, world: &mut World) -> bool;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum GameMode {
    Debug,
    Joust,
}

impl ToString for GameMode {
    fn to_string(&self) -> String {
        return match self {
            GameMode::Debug => "debug",
            GameMode::Joust => "joust",
        }.to_owned();
    }
}

impl FromStr for GameMode {
    type Err = ParseGameTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match s {
            "debug" => Ok(Self::Debug),
            "joust" => Ok(Self::Joust),
            _ => Err(ParseGameTypeError),
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseGameTypeError;

impl fmt::Display for ParseGameTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return "provided string was not a known game mode".fmt(f);
    }
}

fn start<T>(players: HashSet<PlayerId>, world: &mut World) -> State
    where T: Game + GameData + 'static,
          T::Data: PlayerColor {
    let game = T::create(players, world);
    debug!("Game created");

    return State::Countdown(Countdown::new(game, world));
}

impl GameMode {
    pub fn create(self, players: HashSet<PlayerId>, world: &mut World) -> State {
        return match self {
            Self::Debug => State::Playing(Box::new(Debug::new(world))),
            Self::Joust => start::<Joust>(players, world),
        };
    }
}
