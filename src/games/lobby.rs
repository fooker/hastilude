use std::collections::HashSet;

use crate::games::meta::Countdown;
use crate::psmove::Feedback;
use crate::state::{Data, State, Transition};

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
            return Transition::Sequence(vec![
                Transition::Push(data.game.create(data)),
                Transition::Push(Box::new(Countdown::new())),
            ]);
        }

        return Transition::None;
    }
}