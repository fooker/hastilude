use std::collections::HashMap;
use std::time::{Duration, Instant};

use cgmath::InnerSpace;
use heapless::HistoryBuffer;
use rand::Rng;
use scarlet::color::{Color, RGBColor};
use scarlet::colors::HSVColor;

use crate::games::meta::Winner;
use crate::psmove::Feedback;
use crate::sound::Playback;
use crate::state::{Data, State, Transition};
use crate::animation::Animated;

struct Player {
    hue: f64,
    accel_buffer: HistoryBuffer<f32, 4>,
}

#[derive(Debug,Copy, Clone)]
enum Speed {
    NORMAL,
    FAST,
    SLOW,
}

impl Speed {
    pub fn music(self) -> f32 {
        return match self {
            Speed::NORMAL => 0.0,
            Speed::FAST => 1.5,
            Speed::SLOW => 0.5,
        };
    }

    pub fn threshold(self) -> f32 {
        return match self {
            Speed::NORMAL => 3.2,
            Speed::FAST => 4.5,
            Speed::SLOW => 2.1,
        };
    }
}

pub struct Joust {
    alive: HashMap<String, Player>,

    speed: (Speed, Instant),

    music: Playback,
    music_speed: Animated<f32>,

    threshold: Animated<f32>,
}

impl Joust {
    const CHANGE_SPEED_MUSIC: f32 = 2.0;

    // Change threshold slower than music to give some players time to adapt
    const CHANGE_SPEED_THRESHOLD: f32 = 0.7;

    const MUSIC_TIME_MIN: Duration = Duration::from_secs(10);
    const MUSIC_TIME_MAX: Duration = Duration::from_secs(23);

    pub fn new(data: &Data) -> Self {
        let music = data.assets.music.random();
        let music = data.sound.music(music);

        return Self {
            alive: HashMap::new(),
            speed: (Speed::NORMAL, Instant::now()),
            music,
            music_speed: Animated::new(Speed::NORMAL.music(), Self::CHANGE_SPEED_MUSIC),
            threshold: Animated::new(Speed::NORMAL.threshold(), Self::CHANGE_SPEED_THRESHOLD),
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

    fn on_resume(&mut self, _: &mut Data) {
        let duration = rand::thread_rng().gen_range(Self::MUSIC_TIME_MIN..Self::MUSIC_TIME_MAX);
        self.speed = (Speed::NORMAL, Instant::now() + duration);
        self.music_speed = Animated::new(Speed::NORMAL.music(), Self::CHANGE_SPEED_MUSIC);
        self.threshold = Animated::new(Speed::NORMAL.threshold(), Self::CHANGE_SPEED_THRESHOLD);
    }

    fn on_update(&mut self, data: &mut Data, duration: Duration) -> Transition {
        self.music_speed.update(duration);
        self.threshold.update(duration);

        let now = Instant::now();
        if self.speed.1 < now {
            let duration = rand::thread_rng().gen_range(Self::MUSIC_TIME_MIN..Self::MUSIC_TIME_MAX);

            let speed = match self.speed.0 {
                Speed::NORMAL => if rand::thread_rng().gen() {
                    Speed::FAST
                } else {
                    Speed::SLOW
                }
                Speed::FAST |
                Speed::SLOW => Speed::NORMAL,
            };

            self.music.speed(self.music_speed.value());

            self.speed = (speed, now + duration);
        }

        for controller in data.controllers.iter_mut() {
            let mut feedback = Feedback::new();
            if let Some(player) = self.alive.get_mut(controller.serial()) {
                player.accel_buffer.write((1.0 - controller.input().accelerometer.magnitude()).abs());
                let accel = player.accel_buffer.iter().sum::<f32>() / self.threshold.value();

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

        if self.alive.len() <= 0 {
            return Transition::Replace(Box::new(Winner::new(self.alive.keys().cloned())));
        }

        return Transition::None;
    }
}
