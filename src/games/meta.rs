use std::collections::HashSet;
use std::time::Duration;

use rand::Rng;
use scarlet::color::{Color, RGBColor};
use scarlet::colors::HSVColor;
use tracing::debug;

use crate::{keyframe, keyframes};
use crate::engine::players::{PlayerData, PlayerId};
use crate::engine::state::{State, World};
use crate::games::Game;
use crate::lobby::Lobby;

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

pub struct Winner {
    elapsed: Duration,
}

impl Winner {
    const TIME: Duration = Duration::from_secs(10);

    pub fn new(winners: HashSet<PlayerId>, world: &mut World) -> Self {
        debug!("Celebrating winners: {:?}", winners);

        // TODO: Get rid of this
        let mut winners = PlayerData::init(winners, || ());

        for (player, _) in world.players.with_data(&mut winners).existing() {
            player.rumble.animate(keyframes![
                0.0 => 0   @ quadratic_in_out,
                0.8 => 200 @ quadratic_in_out,
                0.2 => 0   @ quadratic_in_out,

                0.5 => 0   @ quadratic_in_out,
                0.8 => 200 @ quadratic_in_out,
                0.2 => 0   @ quadratic_in_out,

                0.5 => 0   @ quadratic_in_out,
                0.8 => 200 @ quadratic_in_out,
                0.2 => 0   @ quadratic_in_out,
            ]);

            // Generate fireworks animation
            let fireworks = std::iter::from_fn({
                let mut elapsed = Duration::ZERO;

                move || {
                    if elapsed >= Self::TIME {
                        return None;
                    }

                    let duration = Duration::from_millis(rand::thread_rng().gen_range(100..700));
                    let color = HSVColor {
                        h: rand::thread_rng().gen_range(0.0..360.0),
                        s: 1.0,
                        v: 1.0,
                    }.convert::<RGBColor>();

                    elapsed += duration;

                    return Some(keyframe!(duration => { color }));
                }
            }).intersperse(keyframe!(0.2 => { (0,0,0) } @ quadratic_out));

            player.color.animate(fireworks);
        }

        return Self {
            elapsed: Duration::ZERO,
        };
    }
}

impl State for Winner {
    fn update(mut self: Box<Self>, world: &mut World, duration: Duration) -> Box<dyn State> {
        self.elapsed += duration;

        if self.elapsed >= Duration::from_secs(10) {
            debug!("Enough partying - back to lobby");
            return Box::new(Lobby::new(world.players));
        }

        return self;
    }
}