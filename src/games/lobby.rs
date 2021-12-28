use std::collections::HashSet;

use crate::psmove::{Battery, Feedback};
use crate::state::{Data, State, Transition};
use tokio::time::Instant;
use std::time::Duration;

struct Countdown {
    start: Instant,
}

impl Countdown {
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
                feedback = feedback.led_color((0xff, 0x00, 0x00));
            } else if elapsed < Duration::from_secs(2) {
                feedback = feedback.led_color((0xff, 0xff, 0x00));
            } else if elapsed < Duration::from_secs(3) {
                feedback = feedback.led_color((0x00, 0xff, 0x00));
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

            // Show battery state while pressing circle
            if controller.input().buttons.circle {
                controller.feedback(Feedback::new()
                    .led_color(match controller.battery() {
                        Battery::Draining(level) => {
                            let level = (level * 255.0) as u8;
                            (0xFF - level, level, 0x00)
                        }
                        Battery::Charging => (0x00, 0x00, 0xFF),
                        Battery::Charged => (0x00, 0xFF, 0x00),
                        Battery::Unknown => (0x00, 0x00, 0x00),
                    }));
            } else if self.ready.contains(controller.serial()) {
                controller.feedback(Feedback::new()
                    .led_color((0xff, 0xff, 0xff)));
            } else {
                controller.feedback(Feedback::new()
                    .led_off());
            }
        }

        if data.controllers.iter().all(|controller| self.ready.contains(controller.serial())) ||
            data.controllers.iter().any(|controller| controller.input().buttons.start) {
            return Transition::Push(Box::new(Countdown::new()));
        }

        return Transition::None;
    }
}