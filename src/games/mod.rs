use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::engine::players::{PlayerData, PlayerId};
use crate::games::debug::Debug;
use crate::games::joust::Joust;
use crate::meta::countdown::{Countdown, PlayerColor};
use crate::state::{State, World};

pub mod debug;
pub mod joust;

pub struct Session {
    // The time when the session was started
    pub started: Instant,
}

impl Session {
    pub fn new() -> Self {
        return Self {
            started: Instant::now(),
        };
    }

    pub fn age(&self, now: Instant) -> Duration {
        return now - self.started;
    }
}

pub struct GameState {
    game: Box<dyn Game>,
    session: Session,
}

impl GameState {
    pub fn new(game: Box<dyn Game>) -> Self {
        let session = Session::new();
        return Self {
            game,
            session,
        };
    }

    pub fn update(mut self, world: &mut World, duration: Duration) -> State {
        if let Some(state) = self.game.update(world, duration, &self.session) {
            return state;
        } else {
            return State::Playing(self);
        }
    }

    pub fn kick_player(&mut self, player: PlayerId, world: &mut World) -> bool {
        return self.game.kick_player(player, world);
    }
}

pub trait GameData: Game {
    type Data;

    fn data(&mut self) -> &mut PlayerData<Self::Data>;

    fn create(players: HashSet<PlayerId>, world: &mut World) -> Self
        where Self: Sized;
}

pub trait Game {
    fn update(&mut self, world: &mut World, duration: Duration, session: &Session) -> Option<State>;

    /// Removes a player form the game. Returns whether the player was part of the game.
    fn kick_player(&mut self, player: PlayerId, world: &mut World) -> bool;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum GameMode {
    Debug,
    Joust,
}

impl Default for GameMode {
    fn default() -> Self {
            return Self::Joust;
    }
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
            Self::Debug => State::Playing(GameState::new(Box::new(Debug::new(world)))),
            Self::Joust => start::<Joust>(players, world),
        };
    }
}
