use std::time::Duration;

use scarlet::color::RGBColor;
use tracing::debug;

use crate::engine::state::{State, World};
use crate::games::Game;
use crate::keyframes;

pub trait PlayerColor {
    fn color(&self) -> RGBColor;
}

pub struct Countdown<T>
    where
        T: Game,
        T::Data: PlayerColor,
{
    game: T,
    elapsed: Duration,
}

impl<T> Countdown<T>
    where
        T: Game,
        T::Data: PlayerColor,
{
    pub fn new(mut game: T, world: &mut World) -> Self {
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
            game,
            elapsed: Duration::ZERO,
        };
    }
}

impl<T> State for Countdown<T>
    where
        T: Game,
        T::Data: PlayerColor,
{
    fn update(mut self: Box<Self>, _: &mut World, duration: Duration) -> Box<dyn State> {
        self.elapsed += duration;

        if self.elapsed >= Duration::from_secs(3) {
            debug!("Countdown finished - start game");
            return Box::new(self.game);
        }

        return self;
    }
}
