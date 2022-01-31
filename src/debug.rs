use std::time::Duration;

use scarlet::color::RGBColor;
use scarlet::colorpoint::ColorPoint;

use crate::engine::sound::Playback;
use crate::engine::state::{State, World};
use crate::lobby::Lobby;
use crate::psmove::{Battery, Feedback};

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
        let triangle = world.controllers.iter()
            .any(|controller| controller.input().buttons.triangle);

        for controller in world.controllers.iter_mut() {
            let mut feedback = Feedback::new();

            if triangle {
                feedback = feedback.led_color(Self::COLOR_WHITE);
            } else if controller.input().buttons.circle {
                feedback = feedback.led_color(battery_to_color(controller.battery()));
            } else {
                feedback = feedback.led_color(vector_to_color(controller.input().accelerometer));
            }

            if controller.input().buttons.swoosh {
                feedback = feedback.rumble((controller.input().buttons.trigger.1 * 255.0) as u8);
            }

            if controller.input().buttons.select {
                self.music = world.sound.music(world.assets.music.random());
            }

            controller.feedback(feedback);
        }

        if world.controllers.iter()
            .any(|controller| controller.input().buttons.start || controller.input().buttons.cross) {
            return Box::new(Lobby::new(world.controllers));
        }

        if let Some(controller) = world.controllers.iter().next() {
            let speed = if controller.input().buttons.square {
                controller.input().buttons.trigger.1 * 1.5
            } else {
                controller.input().buttons.trigger.1 * -1.5
            };

            self.music.speed(speed);
        }

        return self;
    }
}