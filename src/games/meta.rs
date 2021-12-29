use std::collections::HashSet;
use std::time::{Duration, Instant};

use scarlet::color::{Color, RGBColor};
use scarlet::colors::HSVColor;

use crate::psmove::Feedback;
use crate::state::{Data, State, Transition};

pub struct Countdown {
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
            return Transition::Pop;
        }

        return Transition::None;
    }
}

pub struct Winner {
    winners: HashSet<String>,
    start: Instant,
}

impl Winner {
    pub fn new(winners: impl Iterator<Item=String>) -> Self {
        return Self {
            winners: winners.collect(),
            start: Instant::now(),
        };
    }
}

impl State for Winner {
    fn on_update(&mut self, data: &mut Data) -> Transition {
        let elapsed = self.start.elapsed();

        for controller in data.controllers.iter_mut() {
            let mut feedback = Feedback::new();

            if self.winners.is_empty() ||
                self.winners.contains(controller.serial()) {
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