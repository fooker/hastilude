use std::time::Duration;

use scarlet::color::RGBColor;
use tracing::debug;

use crate::engine::World;
use crate::games::{Game, GameData};
use crate::keyframes;
use crate::state::State;

pub trait PlayerColor {
    fn color(&self) -> RGBColor;
}

pub struct Countdown {
    game: Box<dyn Game>,
    elapsed: Duration,
}

impl Countdown {
    pub fn new<T>(mut game: T, world: &mut World) -> Self
        where
            T: Game + GameData + 'static,
            T::Data: PlayerColor,
    {
        debug!("Start countdown");

        // Short initial buzz for all players
        for (player, data) in world.players.with_data(game.data()).existing() {
            player.rumble.animate(keyframes![
            0.0 => 127,
            0.1 => 0,
        ]);

            player.color.animate(keyframes![
            0.0 => { (0, 0, 0) },

            0.75 => { data.color() } @ end,

            0.10 => { (0, 0, 0) } @ linear,
            0.65 => { data.color() } @ end,

            0.20 => { (0, 0, 0) } @ linear,
            0.55 => { data.color() } @ end,

            0.30 => { (0, 0, 0) } @ linear,
            0.45 => { data.color() } @ end,
        ]);
        }

        return Self {
            game: Box::new(game),
            elapsed: Duration::ZERO,
        };
    }

    pub fn update(mut self, _: &mut World, duration: Duration) -> State {
        self.elapsed += duration;

        if self.elapsed >= Duration::from_secs(3) {
            debug!("Countdown finished - start game");
            return State::Playing(self.game);
        }

        return State::Countdown(self);
    }
}
