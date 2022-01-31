use std::collections::HashSet;
use std::ops::Range;
use std::time::Duration;

use scarlet::color::{Color, RGBColor};
use scarlet::colors::HSVColor;

use crate::engine::players::{ControllerId, PlayerData};
use crate::engine::state::{State, World};
use crate::games::Game;
use crate::lobby::Lobby;
use crate::psmove::Feedback;

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

    pub fn new(game: T) -> Self {
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

        for (controller, data) in world.controllers.with_data(self.game.data()).existing() {
            let mut feedback = Feedback::new();

            // Short initial buzz
            if self.elapsed < Duration::from_millis(100) {
                feedback = feedback.rumble(0x7F);
            }

            if Self::STEPS[0].contains(&self.elapsed) ||
                Self::STEPS[1].contains(&self.elapsed) ||
                Self::STEPS[2].contains(&self.elapsed) {
                feedback = feedback.led_color(data.color());
            }

            controller.feedback(feedback);
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
    pub fn new(winners: HashSet<ControllerId>) -> Self {
        return Self {
            winners: PlayerData::init(winners, || ()),
            elapsed: Duration::ZERO,
        };
    }
}

impl State for Winner {
    fn update(mut self: Box<Self>, world: &mut World, duration: Duration) -> Box<dyn State> {
        self.elapsed += duration;

        for (controller, ()) in world.controllers.with_data(&mut self.winners)
            .existing() {
            let mut feedback = Feedback::new();

            feedback = feedback.led_color(HSVColor {
                h: (self.elapsed.as_secs_f64() * 90.0) % 360.0,
                s: 1.0,
                v: 1.0
            }.convert::<RGBColor>());

            if self.elapsed < Duration::from_millis(1500) {
                feedback = feedback.rumble(((self.elapsed.as_secs_f32() * std::f32::consts::PI * 2.0).sin().abs() * 255.0) as u8);
            }

            controller.feedback(feedback);
        }

        if self.elapsed >= Duration::from_secs(10) {
            return Box::new(Lobby::new(world.controllers));
        }

        return self;
    }
}