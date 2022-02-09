use std::collections::HashSet;
use std::ops::Range;
use std::time::Duration;

use scarlet::color::{Color, RGBColor};
use scarlet::colors::HSVColor;

use crate::engine::players::{PlayerData, PlayerId};
use crate::engine::state::{State, World};
use crate::games::Game;
use crate::keyframes;
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
    const PHASE: Duration = Duration::from_millis(750);
    const LIGHT: Duration = Duration::from_millis(150);

    const STEPS: [Range<Duration>; 3] = [
        (Self::PHASE.saturating_mul(1)..Self::PHASE.saturating_mul(1).saturating_add(Self::LIGHT)),
        (Self::PHASE.saturating_mul(2)..Self::PHASE.saturating_mul(2).saturating_add(Self::LIGHT)),
        (Self::PHASE.saturating_mul(3)..Self::PHASE.saturating_mul(3).saturating_add(Self::LIGHT)),
    ];

    pub fn new(mut game: T, world: &mut World) -> Self {
        // Short initial buzz for all players
        for (player, _) in world.players.with_data(game.data()).existing() {
            player.rumble.animate(keyframes![
                0.0 => 127,
                0.1 => 0,
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
    fn update(mut self: Box<Self>, world: &mut World, duration: Duration) -> Box<dyn State> {
        self.elapsed += duration;

        for (player, data) in world.players.with_data(self.game.data()).existing() {
            if Self::STEPS[0].contains(&self.elapsed) ||
                Self::STEPS[1].contains(&self.elapsed) ||
                Self::STEPS[2].contains(&self.elapsed) {
                player.color.set(data.color());
            }
        }

        if self.elapsed >= Self::PHASE * 4 {
            return Box::new(self.game);
        }

        return self;
    }
}

pub struct Winner {
    winners: PlayerData<()>,
    elapsed: Duration,
}

impl Winner {
    pub fn new(winners: HashSet<PlayerId>, world: &mut World) -> Self {
        let mut winners = PlayerData::init(winners, || ());

        for (player, _) in world.players.with_data(&mut winners).existing() {
            player.rumble.animate(keyframes![
                0.4 => 0   @ quadratic_in_out,
                0.1 => 200 @ quadratic_in_out,
                0.1 => 0   @ quadratic_in_out,

                0.4 => 0   @ quadratic_in_out,
                0.1 => 200 @ quadratic_in_out,
                0.1 => 0   @ quadratic_in_out,

                0.4 => 0   @ quadratic_in_out,
                0.4 => 200 @ quadratic_in_out,
                0.4 => 0   @ quadratic_in_out,
            ]);
        }

        return Self {
            winners,
            elapsed: Duration::ZERO,
        };
    }
}

impl State for Winner {
    fn update(mut self: Box<Self>, world: &mut World, duration: Duration) -> Box<dyn State> {
        self.elapsed += duration;

        for (player, ()) in world.players.with_data(&mut self.winners)
            .existing() {
            // TODO: Make this an animation
            // TODO: Flashing in random colors - like a firework
            player.color.set(HSVColor {
                h: (self.elapsed.as_secs_f64() * 90.0) % 360.0,
                s: 1.0,
                v: 1.0,
            }.convert::<RGBColor>());
        }

        if self.elapsed >= Duration::from_secs(10) {
            return Box::new(Lobby::new(world.players));
        }

        return self;
    }
}