use scarlet::color::RGBColor;
use scarlet::colorpoint::ColorPoint;

use crate::psmove::{Battery, Feedback};
use crate::sound::Playback;
use crate::state::{Data, State, Transition};
use std::time::Duration;

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

    pub fn new(data: &Data) -> Self {
        let music = data.assets.music.random();
        let music = data.sound.music(music);

        return Self {
            music,
        };
    }
}

impl State for Debug {
    fn on_update(&mut self, data: &mut Data, _: Duration) -> Transition {
        let triangle = data.controllers.iter()
            .any(|controller| controller.input().buttons.triangle);

        for controller in data.controllers.iter_mut() {
            let mut feedback = Feedback::new();

            if triangle {
                feedback = feedback
                    .led_color(Self::COLOR_WHITE);
            } else if controller.input().buttons.circle {
                feedback = feedback
                    .led_color(battery_to_color(controller.battery()));
            } else {
                feedback = feedback
                    .led_color(vector_to_color(controller.input().accelerometer));
            }

            if controller.input().buttons.swoosh {
                feedback = feedback.rumble((controller.input().buttons.trigger.1 * 255.0) as u8);
            }

            if controller.input().buttons.select {
                self.music = data.sound.music(data.assets.music.random());
            }

            controller.feedback(feedback);

            if controller.input().buttons.start || controller.input().buttons.cross {
                return Transition::Pop;
            }
        }

        if let Some(controller) = data.controllers.first() {
            let speed = if controller.input().buttons.square {
                controller.input().buttons.trigger.1 * 1.5
            } else {
                controller.input().buttons.trigger.1 * -1.5
            };

            self.music.speed(speed);
        }

        return Transition::None;
    }
}