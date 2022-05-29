use std::collections::HashSet;
use std::ops::Range;
use std::time::{Duration, Instant};

use rand::Rng;
use scarlet::color::{Color, RGBColor};
use scarlet::colors::HSVColor;

use crate::engine::animation::Animated;
use crate::engine::players::{PlayerData, PlayerId};
use crate::engine::sound::Playback;
use crate::games::{Game, GameData, Session};
use crate::keyframes;
use crate::meta::celebration::Celebration;
use crate::meta::countdown::PlayerColor;
use crate::state::{State, World};

pub struct Player {
    hue: f64,
}

impl PlayerColor for Player {
    fn color(&self) -> RGBColor {
        return HSVColor {
            h: self.hue * 360.0 % 360.0,
            s: 1.0,
            v: 1.0,
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

    hue_base: f64,
}

impl Joust {
    // Speed for changes in pacing
    const PACING_CHANGE_SPEED: Duration = Duration::from_millis(1200);

    // Slack for slowing down movement detection
    const PACING_CHANGE_SLACK: Duration = Duration::from_millis(3000);

    // Minimum / maximum duration of a regular pacing phase
    const PACING_REGULAR_DUR: Range<Duration> = (Duration::from_secs(10) .. Duration::from_secs(30));

    // Minimum / maximum duration of a changed pacing phase
    const PACING_CHANGED_DUR: Range<Duration> = (Duration::from_secs(5) .. Duration::from_secs(15));

    // Speed of hue rotation (time for a full rotation)
    const HUE_ROTATION_SPEED: f64 = 1.0 / 120.0;

    // Speed of hue adoption when hue must change
    const HUE_ADOPTION_SPEED: f64 = 1.0 / 10.0;
}

impl Game for Joust {
    fn update(&mut self, world: &mut World, duration: Duration, session: &Session) -> Option<State> {
        self.music_speed.update(duration);
        self.threshold.update(duration);

        // Check if speed is about to change
        if self.speed.1 < world.now {
            let (speed, slack) = match self.speed.0 {
                Speed::NORMAL => if rand::thread_rng().gen() {
                    (Speed::FAST, false)
                } else {
                    (Speed::SLOW, true)
                }
                Speed::FAST => (Speed::NORMAL, true),
                Speed::SLOW => (Speed::NORMAL, false),
            };

            self.music_speed.animate(keyframes![
                Self::PACING_CHANGE_SPEED => { speed.music() } @ linear,
            ]);

            // Apply slack in threshold
            if slack {
                self.threshold.animate(keyframes![
                    Self::PACING_CHANGE_SLACK => { speed.threshold() } @ linear,
                ]);
            } else {
                self.threshold.set(speed.threshold());
            }

            // Roll a dice for duration of the next phase
            let duration = rand::thread_rng().gen_range(match speed {
                Speed::NORMAL => Self::PACING_REGULAR_DUR,
                _ => Self::PACING_CHANGED_DUR,
            });

            self.speed = (speed, world.now + duration);
        }

        // Update music speed
        self.music.speed(self.music_speed.value());

        // Slowly rotate and re-balance player colors
        for (i, (_, data)) in self.data.iter_mut().enumerate() {
            let target_hue = self.hue_base
                + session.age(world.now).as_secs_f64() * Self::HUE_ROTATION_SPEED
                + (1.0 / world.players.count() as f64) * i as f64;
            let delta_hue = target_hue - data.hue;
            data.hue += delta_hue.signum() * (Self::HUE_ADOPTION_SPEED * duration.as_secs_f64()).min(delta_hue.abs());
        }

        // Update players
        world.players.with_data(&mut self.data).update(|player, data| {
            let accel = player.acceleration(true) / self.threshold.value();

            // Check if player has moved to much
            if accel >= 1.0 {
                player.color.set(RGBColor { r: 0.0, g: 0.0, b: 0.0 });
                player.rumble.animate(keyframes![
                    0.0 => 255,
                    1.0 => 0 @ linear,
                ]);

                return false;
            }

            // Update color reflecting players acceleration
            player.color.set(HSVColor {
                h: data.hue * 360.0 % 360.0,
                s: 1.0,
                v: 1.0 - f32::sqrt(accel) as f64,
            }.convert::<RGBColor>());

            return true;
        });

        if self.data.len() == 1 {
            return Some(State::Celebration(Celebration::new(self.data.keys().collect(), world)));
        }

        if self.data.len() == 0 {
            // Got a draw - everybody is winner
            return Some(State::Celebration(Celebration::new(world.players.keys().collect(), world)));
        }

        return None;
    }

    fn kick_player(&mut self, player: PlayerId, world: &mut World) -> bool {
        if self.data.remove(player) {
            // Reset player color
            if let Some(player) = world.players.get_mut(player) {
                player.color.set(RGBColor { r: 0.0, g: 0.0, b: 0.0 })
            }

            return true;
        }

        return false;
    }
}

impl GameData for Joust {
    type Data = Player;

    fn data(&mut self) -> &mut PlayerData<Player> {
        return &mut self.data;
    }

    fn create(players: HashSet<PlayerId>, world: &mut World) -> Self {
        let music = world.assets.music.random();
        let music = world.sound.music(music);

        // Create players and assign colors
        let hue_base: f64 = rand::random();
        let hue_step: f64 = 1.0 / world.players.count() as f64;

        let players = PlayerData::init_with(players.into_iter()
            .enumerate()
            .map(|(i, id)| (id, Player {
                hue: hue_base + hue_step * i as f64,
            }))
            .collect());

        return Self {
            data: players,
            speed: (Speed::NORMAL, Instant::now() + Self::PACING_REGULAR_DUR.end),
            music,
            music_speed: Animated::idle(Speed::NORMAL.music()),
            threshold: Animated::idle(Speed::NORMAL.threshold()),
            hue_base,
        };
    }
}
