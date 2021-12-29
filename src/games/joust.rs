use std::collections::HashMap;
use std::time::{Instant, Duration};

use cgmath::InnerSpace;
use heapless::HistoryBuffer;
use scarlet::color::{Color, RGBColor};
use scarlet::colors::HSVColor;

use crate::psmove::Feedback;
use crate::state::{Data, State, Transition};

struct Winner {
    serial: Option<String>,
    start: Instant,
}

impl Winner {
    pub fn new(serial: Option<String>) -> Self {
        return Self {
            serial,
            start: Instant::now(),
        };
    }
}

impl State for Winner {
    fn on_update(&mut self, data: &mut Data) -> Transition {
        let elapsed = self.start.elapsed();

        for controller in data.controllers.iter_mut() {
            let mut feedback = Feedback::new();

            if self.serial.as_ref().map_or(true, |serial| controller.serial() == serial) {
                feedback = feedback.led_color(HSVColor {
                    h: (elapsed.as_secs_f64() * 90.0) % 360.0,
                    s: 1.0,
                    v: 1.0
                }.convert::<RGBColor>());

                if elapsed < Duration::from_millis(1500) {
                    feedback = feedback.rumble(((elapsed.as_secs_f32() * std::f32::consts::PI * 2.0).sin().abs() * 255.0) as u8);
                }
            }

            controller.feedback(feedback);
        }

        if elapsed >= Duration::from_secs(10) {
            return Transition::Pop;
        }

        return Transition::None;
    }
}

struct Player {
    hue: f64,
    accel_buffer: HistoryBuffer<f32, 4>,
}

pub struct Joust {
    alive: HashMap<String, Player>,
}

impl Joust {
    const MAX_SPEED: f32 = 3.2;

    pub fn new() -> Self {
        return Self {
            alive: HashMap::new(),
        };
    }
}

impl State for Joust {
    fn on_start(&mut self, data: &mut Data) {
        let hue_base: f64 = rand::random();
        let hue_step: f64 = 1.0 / data.controllers.len() as f64;

        // Initially, all players are alive
        self.alive = data.controllers.iter()
            .enumerate()
            .map(|(i, controller)| (controller.serial().to_string(), Player {
                hue: ((hue_base + hue_step * i as f64) * 360.0) % 360.0,
                accel_buffer: HistoryBuffer::new(),
            }))
            .collect();
    }

    fn on_update(&mut self, data: &mut Data) -> Transition {
        for controller in data.controllers.iter_mut() {
            let mut feedback = Feedback::new();
            if let Some(player) = self.alive.get_mut(controller.serial()) {
                player.accel_buffer.write((1.0 - controller.input().accelerometer.magnitude()).abs());
                let accel = player.accel_buffer.iter().sum::<f32>() / Self::MAX_SPEED;

                if accel >= 1.0 {
                    self.alive.remove(controller.serial());

                    feedback = feedback.led_off();
                } else {
                    feedback = feedback.led_color(HSVColor {
                        h: player.hue,
                        s: 1.0,
                        v: 1.0 - accel as f64,
                    }.convert::<RGBColor>());
                }
            } else {}

            controller.feedback(feedback);
        }

        if self.alive.len() <= 1 {
            return Transition::Replace(Box::new(Winner::new(self.alive.keys().next().cloned())));
        }

        return Transition::None;
    }
}
