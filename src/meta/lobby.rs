use std::collections::HashSet;

use scarlet::color::RGBColor;
use tracing::debug;

use crate::{GAME_MODE, keyframes};
use crate::engine::players::{PlayerId, Players};
use crate::engine::World;
use crate::games::debug;
use crate::state::State;

pub struct Lobby {
    ready: HashSet<PlayerId>,
}

impl Lobby {
    pub fn new(players: &mut Players) -> Self {
        // Reset all controllers
        for player in players.iter_mut() {
            player.color.set(RGBColor { r: 0.0, g: 0.0, b: 0.0 });
            player.rumble.set(0);
        }

        return Self {
            ready: HashSet::new(),
        };
    }

    pub fn update(mut self, world: &mut World) -> State {
        // Players can start the game by pressing the start button. But only if more than one player
        // is ready. By this they will become ready themself.
        let mut start = false;

        for player in world.players.iter_mut() {
            if !self.ready.contains(&player.id()) && player.input().buttons.trigger.0 {
                self.ready.insert(player.id());

                debug!("Player {} ready ({})", player.id(), self.ready.len());

                player.rumble.animate(keyframes![
                    0.00 => 64,
                    0.05 => 0,
                ]);
            }

            if self.ready.len() >= 2 && player.input().buttons.start {
                self.ready.insert(player.id());
                start = true;
                debug!("Starting on player {} request", player.id());
            }

            if player.input().buttons.circle {
                player.color.set(debug::battery_to_color(player.battery()));
            } else if self.ready.contains(&player.id()) {
                player.color.set(RGBColor { r: 1.0, g: 1.0, b: 1.0 });
            } else {
                player.color.set(RGBColor { r: 0.0, g: 0.0, b: 0.0 });
            }
        }

        if self.ready.len() >= 2 && self.ready.len() >= world.players.count() {
            debug!("Starting as all players are ready");
            start = true;
        }

        if start {
            // Collect players and reset ready list for next game
            let players = std::mem::take(&mut self.ready);

            let game_mode = GAME_MODE.lock();
            debug!("Starting game {:?}", game_mode);
            return game_mode.create(players, world);
        }

        return State::Lobby(self);
    }
}