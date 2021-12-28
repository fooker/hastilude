use crate::psmove::{Battery, Feedback};
use crate::state::{Data, State, Transition};

pub struct Debug;

impl Debug {
    pub fn new() -> Self {
        return Self {};
    }

    fn battery_to_color(battery: &Battery) -> (u8, u8, u8) {
        return match battery {
            Battery::Draining(level) => {
                let level = (level * 255.0) as u8;
                (0xFF - level, level, 0x00)
            }
            Battery::Charging => (0x00, 0x00, 0xFF),
            Battery::Charged => (0x00, 0xFF, 0x00),
            Battery::Unknown => (0x00, 0x00, 0x00),
        };
    }

    fn acceleration_to_color(a: cgmath::Vector3<f32>) -> (u8, u8, u8) {
        return a
            .map(|v| (v.abs().clamp(0.0, 1.0) * 255.0) as u8)
            .into();
    }
}

impl State for Debug {
    fn on_update(&mut self, data: &mut Data) -> Transition {
        for controller in data.controllers.iter_mut() {
            let mut feedback = Feedback::new();

            if controller.input().buttons.circle {
                feedback = feedback
                    .led_color(Self::battery_to_color(controller.battery()));
            } else {
                feedback = feedback
                    .led_color(Self::acceleration_to_color(controller.input().accelerometer));
            }

            feedback = feedback.rumble((controller.input().buttons.trigger.1 * 255.0) as u8);

            controller.feedback(feedback);

            if controller.input().buttons.start || controller.input().buttons.cross {
                return Transition::Pop;
            }
        }

        return Transition::None;
    }
}