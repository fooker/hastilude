#![feature(type_alias_impl_trait)]
#![feature(iter_intersperse)]
#![feature(result_flattening)]

use std::time::Instant;

use anyhow::{Context, Result};
use parking_lot::Mutex;

use crate::engine::assets::Assets;
use crate::engine::players::{Players};
use crate::engine::sound::Sound;
use crate::engine::World;
use crate::games::GameMode;
use crate::state::State;

pub mod controller;
pub mod engine;
pub mod games;
pub mod meta;
pub mod state;

static GAME_MODE: Mutex<GameMode> = parking_lot::const_mutex(GameMode::Joust);

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("hyper=INFO,DEBUG")
        .with_ansi(true)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .compact()
        .init();

    let mut players = Players::init().await
        .context("Failed to initialize players")?;

    let mut sound = Sound::init()
        .context("Failed to initialize sound")?;

    let assets = Assets::init(std::env::current_dir()?.join("assets"))
        .context("Failed to initialize assets")?;

    let mut last = Instant::now();

    // Initialize fresh state machine
    let mut state = State::lobby(&mut players);

    loop {
        // Calculate last frame duration
        let now = Instant::now();
        let duration = now - last;

        // Update controller information
        players.update(duration).await
            .context("Failed to update players")?;

        // Play the game
        state = state.update(&mut World {
            now,
            players: &mut players,
            sound: &mut sound,
            assets: &assets,
        }, duration);

        last = now;
    }
}
