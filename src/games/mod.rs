pub use lobby::Lobby;

use crate::state::{Data, State};

pub mod lobby;
pub mod meta;
pub mod debug;
pub mod joust;

#[derive(Debug, Copy, Clone)]
pub enum Game {
    Debug,
    Joust,
}

impl Game {
    pub fn create(self, data: &Data) -> Box<dyn State> {
        return match self {
            Self::Debug => Box::new(debug::Debug::new(data)),
            Self::Joust => Box::new(joust::Joust::new(data)),
        };
    }
}

