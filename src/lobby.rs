use std::collections::HashSet;
use std::time::Duration;

use crate::engine::players::{ControllerId, Controllers};
use crate::engine::state::{State, World};
use crate::psmove::Feedback;

pub struct Lobby {
    ready: HashSet<ControllerId>,
}

impl Lobby {
    pub fn new(controllers: &mut Controllers) -> Self {
        // Reset all controllers
        for controller in controllers.iter_mut() {
            controller.feedback(Feedback::new());
        }

        return Self {
            ready: HashSet::new(),
        };
    }
}

impl State for Lobby {
    fn update(mut self: Box<Self>, world: &mut World, _: Duration) -> Box<dyn State> {
        // Players can start the game by pressing the start button. But only if more than one player
        // is ready. By this they will become ready themself.
        let mut start = false;

        for controller in world.controllers.iter_mut() {
            let mut feedback = Feedback::new();

            if !self.ready.contains(&controller.id()) && controller.input().buttons.trigger.0 {
                self.ready.insert(controller.id());

                // TODO: Make animation
                feedback = feedback.rumble(64);
            }

            if self.ready.len() > 1 && controller.input().buttons.start {
                self.ready.insert(controller.id());
                start = true;
            }

            if controller.input().buttons.circle {
                feedback = feedback.led_color(super::debug::battery_to_color(controller.battery()));
            } else if self.ready.contains(&controller.id()) {
                feedback = feedback.led_color((0xff, 0xff, 0xff));
            } else {
                feedback = feedback.led_off();
            }

            controller.feedback(feedback);
        }

        if self.ready.len() >= world.controllers.count() || start {
            // Collect players and reset ready list for next game
            let players = std::mem::take(&mut self.ready);

            let game = world.game.create(players, world);

            return game;
        }

        return self;
    }
}