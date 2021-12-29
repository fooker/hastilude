use std::collections::HashSet;
use std::time::Duration;

use scarlet::color::RGBColor;
use tokio::time::Instant;

use crate::psmove::Feedback;
use crate::state::{Data, State, Transition};

struct Countdown {
    start: Instant,
}

impl Countdown {
    const COLOR_1: RGBColor = RGBColor { r: 1.0, g: 0.0, b: 0.0 };
    const COLOR_2: RGBColor = RGBColor { r: 1.0, g: 1.0, b: 0.0 };
    const COLOR_3: RGBColor = RGBColor { r: 0.0, g: 1.0, b: 0.0 };

    pub fn new() -> Self {
        return Self {
            start: Instant::now(),
        };
    }
}

impl State for Countdown {
    fn on_update(&mut self, data: &mut Data) -> Transition {
        let elapsed = self.start.elapsed();

        for controller in data.controllers.iter_mut() {
            let mut feedback = Feedback::new();
            if elapsed < Duration::from_millis(250) {
                feedback = feedback.rumble(0xFF);
            }

            if elapsed < Duration::from_secs(1) {
                feedback = feedback.led_color(Self::COLOR_1);
            } else if elapsed < Duration::from_secs(2) {
                feedback = feedback.led_color(Self::COLOR_2);
            } else if elapsed < Duration::from_secs(3) {
                feedback = feedback.led_color(Self::COLOR_3);
            }

            controller.feedback(feedback);
        }

        if elapsed >= Duration::from_secs(3) {
            return Transition::Replace(data.game.create());
        }

        return Transition::None;
    }
}

pub struct Lobby {
    ready: HashSet<String>,
}

impl Lobby {
    pub fn new() -> Self {
        return Self {
            ready: HashSet::new()
        };
    }

    fn reset(&mut self, data: &mut Data) {
        // Reset all controllers
        for controller in data.controllers.iter_mut() {
            controller.feedback(Feedback::default());
        }

        self.ready = HashSet::new();
    }
}

impl State for Lobby {
    fn on_start(&mut self, data: &mut Data) {
        self.reset(data);
    }

    fn on_resume(&mut self, data: &mut Data) {
        self.reset(data);
    }

    fn on_update(&mut self, data: &mut Data) -> Transition {
        for controller in data.controllers.iter_mut() {
            if controller.input().buttons.trigger.0 {
                self.ready.insert(controller.serial().to_string());
            }

            let mut feedback = Feedback::new();

            if controller.input().buttons.circle {
                feedback = feedback.led_color(super::debug::battery_to_color(controller.battery()));
            } else if self.ready.contains(controller.serial()) {
                feedback = feedback.led_color((0xff, 0xff, 0xff));
            }

            controller.feedback(feedback);
        }

        if data.controllers.iter().all(|controller| self.ready.contains(controller.serial())) ||
            data.controllers.iter().any(|controller| controller.input().buttons.start) {
            return Transition::Push(Box::new(Countdown::new()));
        }

        return Transition::None;
    }
}