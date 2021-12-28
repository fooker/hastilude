pub use lobby::Lobby;

use crate::state::State;

pub mod lobby;
pub mod debug;
pub mod joust;

#[derive(Debug, Copy, Clone)]
pub enum Game {
    Debug,
    Joust,
}

impl Game {
    pub fn create(self) -> Box<dyn State> {
        return match self {
            Self::Debug => Box::new(debug::Debug::new()),
            Self::Joust => Box::new(joust::Joust::new()),
        };
    }
}

