use std::time::Duration;

use scarlet::color::RGBColor;
use scarlet::colorpoint::ColorPoint;

use crate::engine::sound::Playback;
use crate::engine::state::{State, World};
use crate::lobby::Lobby;
use crate::psmove::Battery;

pub struct Debug {
    music: Playback,
}

pub fn battery_to_color(battery: Battery) -> RGBColor {
    const COLOR_EMPTY: RGBColor = RGBColor { r: 1.0, g: 0.0, b: 0.0 };
    const COLOR_FULL: RGBColor = RGBColor { r: 0.0, g: 1.0, b: 0.0 };
    const COLOR_CHARGING: RGBColor = RGBColor { r: 0.0, g: 0.0, b: 1.0 };
    const COLOR_CHARGED: RGBColor = RGBColor { r: 0.0, g: 1.0, b: 1.0 };
    const COLOR_UNKNOWN: RGBColor = RGBColor { r: 0.3, g: 0.3, b: 0.3 };

    return match battery {
        Battery::Draining(level) => ColorPoint::weighted_midpoint(COLOR_EMPTY, COLOR_FULL, level as f64),
        Battery::Charging => COLOR_CHARGING,
        Battery::Charged => COLOR_CHARGED,
        Battery::Unknown => COLOR_UNKNOWN,
    };
}

pub fn vector_to_color(a: cgmath::Vector3<f32>) -> RGBColor {
    return From::<(u8, u8, u8)>::from(a
        .map(|v| (v.abs().clamp(0.0, 1.0) * 255.0) as u8)
        .into());
}

impl Debug {
    const COLOR_WHITE: RGBColor = RGBColor { r: 1.0, g: 1.0, b: 1.0 };

    pub fn new(world: &mut World) -> Self {
        let music = world.assets.music.random();
        let music = world.sound.music(music);

        return Self { music };
    }
}

impl State for Debug {
    fn update(mut self: Box<Self>, world: &mut World, _: Duration) -> Box<dyn State> {
        let triangle = world.players.iter()
            .any(|player| player.input().buttons.triangle);

        for player in world.players.iter_mut() {
            if triangle {
                player.color.set(Self::COLOR_WHITE);
            } else if player.input().buttons.circle {
                player.color.set(battery_to_color(player.battery()));
            } else {
                player.color.set(vector_to_color(player.input().accelerometer));
            }

            if player.input().buttons.swoosh {
                player.rumble.set((player.input().buttons.trigger.1 * 255.0) as u8);
            }

            if player.input().buttons.select {
                self.music = world.sound.music(world.assets.music.random());
            }
        }

        if world.players.iter()
            .any(|player| player.input().buttons.start || player.input().buttons.cross) {
            return Box::new(Lobby::new(world.players));
        }

        if let Some(player) = world.players.iter().next() {
            let speed = if player.input().buttons.square {
                player.input().buttons.trigger.1 * 1.5
            } else {
                player.input().buttons.trigger.1 * -1.5
            };

            self.music.speed(speed);
        }

        return self;
    }
}