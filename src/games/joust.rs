use std::collections::{HashMap};

use cgmath::Array;

use crate::state::{Data, State, Transition};

struct Player {
    color: (u8, u8, u8),
    last_accel: f32,
}

pub struct Joust {
    alive: HashMap<String, Player>,
}

impl Joust {
    pub fn new() -> Self {
        return Self {
            alive: HashMap::new(),
        };
    }
}

impl State for Joust {
    fn on_start(&mut self, data: &mut Data) {
        // Initially, all players are alive
        self.alive = data.controllers.iter()
            .map(|controller| (controller.serial().to_string(), Player {
                color: (0, 0, 0),
                last_accel: controller.input().accelerometer.sum(),
            }))
            .collect();
    }

    fn on_update(&mut self, data: &mut Data) -> Transition {
        for controller in data.controllers.iter_mut() {
            if let Some(player) = self.alive.get_mut(controller.serial()) {
                let total = controller.input().accelerometer.sum();
            } else {

            }
        }

        return Transition::None;
    }
}
