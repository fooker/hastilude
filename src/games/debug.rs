use scarlet::color::RGBColor;

use crate::psmove::{Battery, Feedback};
use crate::state::{Data, State, Transition};
use crate::sound::Music;

pub struct Debug {
    music: Music,
}

pub fn battery_to_color(battery: Battery) -> RGBColor {
    return match battery {
        Battery::Draining(level) => RGBColor { r: 1.0 - level as f64, g: level as f64, b: 1.0 },
        Battery::Charging => RGBColor { r: 0.0, g: 0.0, b: 1.0 },
        Battery::Charged => RGBColor { r: 0.0, g: 1.0, b: 0.0 },
        Battery::Unknown => RGBColor { r: 0.0, g: 0.0, b: 0.0 },
    };
}

pub fn acceleration_to_color(a: cgmath::Vector3<f32>) -> RGBColor {
    return From::<(u8, u8, u8)>::from(a
        .map(|v| (v.abs().clamp(0.0, 1.0) * 255.0) as u8)
        .into());
}

impl Debug {
    pub fn new(data: &Data) -> Self {
        // TODO: Error handling
        let music = data.sound.music("assets/music/loop.ogg")
            .expect("Can not load music");

        return Self {
            music,
        };
    }
}

impl State for Debug {
    fn on_update(&mut self, data: &mut Data) -> Transition {
        for controller in data.controllers.iter_mut() {
            let mut feedback = Feedback::new();

            if controller.input().buttons.circle {
                feedback = feedback
                    .led_color(battery_to_color(controller.battery()));
            } else {
                feedback = feedback
                    .led_color(acceleration_to_color(controller.input().accelerometer));
            }

            if controller.input().buttons.swoosh {
                feedback = feedback.rumble((controller.input().buttons.trigger.1 * 255.0) as u8);
            }

            controller.feedback(feedback);

            if controller.input().buttons.start || controller.input().buttons.cross {
                return Transition::Pop;
            }
        }

        if let Some(controller) = data.controllers.first() {
            let speed = if controller.input().buttons.square {
                (controller.input().buttons.trigger.1 * 127.0) as i8
            } else {
                (controller.input().buttons.trigger.1 * -127.0) as i8
            };

            self.music.speed(speed);
        }

        return Transition::None;
    }
}