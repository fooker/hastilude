use std::time::Instant;

use crate::engine::assets::Assets;
use crate::engine::players::Players;
use crate::engine::sound::Sound;

pub mod players;
pub mod sound;
pub mod assets;
pub mod animation;

pub struct World<'a, S> {
    // Current time of the frame
    pub now: Instant,

    pub players: &'a mut Players,

    pub sound: &'a mut Sound,

    pub assets: &'a Assets,

    pub settings: &'a mut S,
}
