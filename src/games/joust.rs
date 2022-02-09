use std::collections::HashSet;
use std::time::{Duration, Instant};

use cgmath::InnerSpace;
use heapless::HistoryBuffer;
use rand::Rng;
use scarlet::color::{Color, RGBColor};
use scarlet::colors::HSVColor;

use crate::engine::animation::{Animated, interpolations, Keyframe};
use crate::engine::players::{PlayerData, PlayerId};
use crate::engine::sound::Playback;
use crate::engine::state::{State, World};
use crate::games::Game;
use crate::games::meta::{PlayerColor, Winner};
use crate::keyframes;

pub struct Player {
    alive: bool,

    accel: HistoryBuffer<f32, 4>,

    hue: f64,
}

impl PlayerColor for Player {
    fn color(&self) -> RGBColor {
        let accel = self.accel.recent().copied()
            .unwrap_or(0.0);

        return HSVColor {
            h: self.hue,
            s: 1.0,
            v: 1.0 - f32::sqrt(accel) as f64,
        }.convert::<RGBColor>();
    }
}

#[derive(Debug, Copy, Clone)]
enum Speed {
    NORMAL,
    FAST,
    SLOW,
}

impl Speed {
    pub fn music(self) -> f32 {
        return match self {
            Speed::NORMAL => 1.0,
            Speed::FAST => 1.5,
            Speed::SLOW => 0.5,
        };
    }

    pub fn threshold(self) -> f32 {
        return match self {
            Speed::NORMAL => 0.6,
            Speed::FAST => 0.9,
            Speed::SLOW => 0.3,
        };
    }
}

pub struct Joust {
    data: PlayerData<Player>,

    speed: (Speed, Instant),

    music: Playback,
    music_speed: Animated<f32>,

    threshold: Animated<f32>,
}

impl Joust {
    const CHANGE_SPEED_MUSIC: Duration = Duration::from_millis(500);

    // Change threshold slower than music to give some players time to adapt
    const CHANGE_SPEED_THRESHOLD: Duration = Self::CHANGE_SPEED_MUSIC.saturating_mul(3);

    const MUSIC_TIME_MIN: Duration = Duration::from_secs(15);
    const MUSIC_TIME_MAX: Duration = Duration::from_secs(30);
}

impl State for Joust {
    fn update(mut self: Box<Self>, world: &mut World, duration: Duration) -> Box<dyn State> {
        self.music_speed.update(duration);
        self.threshold.update(duration);

        // Check if speed is about to change
        if self.speed.1 < world.now {
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

            self.music_speed.animate([
                Keyframe { duration: Self::CHANGE_SPEED_MUSIC, value: speed.music(), interpolation: interpolations::elastic_in_out }
            ]);
            self.threshold.animate([
                Keyframe { duration: Self::CHANGE_SPEED_THRESHOLD, value: speed.threshold(), interpolation: interpolations::linear }
            ]);

            self.speed = (speed, world.now + duration);
        }

        // Update music speed
        self.music.speed(self.music_speed.value());

        // Calculating alive players
        let alive = self.data.iter()
            .filter_map(|(id, data)| if data.alive { Some(data) } else { None })
            .collect::<Vec<_>>();


        // Update players
        for (player, data) in world.players.with_data(&mut self.data)
            .existing() {
            if data.alive {
                data.accel.write((1.0 - player.input().accelerometer.magnitude()).abs());
                let accel = data.accel.iter().sum::<f32>() / data.accel.len() as f32;
                let accel = accel / self.threshold.value();

                if accel >= 1.0 {
                    data.alive = false;

                    player.color.set(RGBColor { r: 0.0, g: 0.0, b: 0.0 });
                    player.rumble.animate(keyframes![
                        0.0 => 255,
                        1.0 => 0 @ linear,
                    ]);
                } else {
                    // TODO: Slowly move color around and re-balance if player is out
                    player.color.set(HSVColor {
                        h: data.hue,
                        s: 1.0,
                        v: 1.0 - f32::sqrt(accel) as f64,
                    }.convert::<RGBColor>());
                }
            }
        }

        // Check if at least one player is alive
        let alive = self.data.iter()
            .filter_map(|(id, data)| if data.alive { Some(id) } else { None })
            .collect::<HashSet<_>>();

        if alive.len() <= 1 {
            return Box::new(Winner::new(alive, world));
        }

        return self;
    }
}

impl Game for Joust {
    type Data = Player;

    fn create(players: HashSet<PlayerId>, world: &mut World) -> Self {
        let music = world.assets.music.random();
        let music = world.sound.music(music);

        // Create players and assign colors
        let hue_base: f64 = rand::random();
        let hue_step: f64 = 1.0 / world.players.count() as f64;

        let players = PlayerData::init_with(players.into_iter()
            .enumerate()
            .map(|(i, id)| (id, Player {
                alive: true,
                accel: HistoryBuffer::new(),
                hue: ((hue_base + hue_step * i as f64) * 360.0) % 360.0,
            }))
            .collect());

        return Self {
            data: players,
            speed: (Speed::NORMAL, Instant::now()),
            music,
            music_speed: Animated::idle(Speed::NORMAL.music()),
            threshold: Animated::idle(Speed::NORMAL.threshold()),
        };
    }

    fn data(&mut self) -> &mut PlayerData<Self::Data> {
        return &mut self.data;
    }
}
