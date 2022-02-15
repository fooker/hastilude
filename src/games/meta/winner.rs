use std::collections::HashSet;
use std::time::Duration;

use rand::Rng;
use scarlet::color::{Color, RGBColor};
use scarlet::colors::HSVColor;
use tracing::debug;

use crate::{keyframe, keyframes};
use crate::engine::players::{PlayerData, PlayerId};
use crate::engine::state::{State, World};
use crate::games::meta::lobby::Lobby;

pub struct Winner {
    elapsed: Duration,
}

impl Winner {
    const TIME: Duration = Duration::from_secs(10);

    pub fn new(winners: HashSet<PlayerId>, world: &mut World) -> Self {
        debug!("Celebrating winners: {:?}", winners);

        // TODO: Get rid of this
        let mut winners = PlayerData::init(winners, || ());

        for (player, _) in world.players.with_data(&mut winners).existing() {
            player.rumble.animate(keyframes![
            0.0 => 0   @ quadratic_in_out,
            0.8 => 200 @ quadratic_in_out,
            0.2 => 0   @ quadratic_in_out,

            0.5 => 0   @ quadratic_in_out,
            0.8 => 200 @ quadratic_in_out,
            0.2 => 0   @ quadratic_in_out,

            0.5 => 0   @ quadratic_in_out,
            0.8 => 200 @ quadratic_in_out,
            0.2 => 0   @ quadratic_in_out,
        ]);

            // Generate fireworks animation
            let fireworks = std::iter::from_fn({
                let mut elapsed = Duration::ZERO;

                move || {
                    if elapsed >= Self::TIME {
                        return None;
                    }

                    let duration = Duration::from_millis(rand::thread_rng().gen_range(100..700));
                    let color = HSVColor {
                        h: rand::thread_rng().gen_range(0.0..360.0),
                        s: 1.0,
                        v: 1.0,
                    }.convert::<RGBColor>();

                    elapsed += duration;

                    return Some(keyframe!(duration => { color }));
                }
            }).intersperse(keyframe!(0.2 => { (0,0,0) } @ quadratic_out));

            player.color.animate(fireworks);
        }

        return Self {
            elapsed: Duration::ZERO,
        };
    }
}

impl State for Winner {
    fn update(mut self: Box<Self>, world: &mut World, duration: Duration) -> Box<dyn State> {
        self.elapsed += duration;

        if self.elapsed >= Duration::from_secs(10) {
            debug!("Enough partying - back to lobby");
            return Box::new(Lobby::new(world.players));
        }

        return self;
    }
}
