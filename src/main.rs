#![feature(type_alias_impl_trait)]
#![feature(iter_intersperse)]
#![feature(result_flattening)]

use std::time::Instant;

use anyhow::{Context, Result};
use futures::task::Poll;

use crate::engine::assets::Assets;
use crate::engine::players::Players;
use crate::engine::sound::Sound;
use crate::engine::World;
use crate::state::State;
use crate::web::{Info, GameState};

pub mod controller;
pub mod engine;
pub mod games;
pub mod web;
pub mod meta;
pub mod state;

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

    // Start web interface
    let (web, mut requests, mut info) = web::serve()?;
    let mut web = tokio::spawn(web);

    loop {
        // Calculate last frame duration
        let now = Instant::now();
        let duration = now - last;

        // Handle failures from the web server
        if let Poll::Ready(result) = futures::poll!(&mut web) {
            return result.map_err(Into::into);
        };

        // Update controller information
        players.update(duration).await
            .context("Failed to update players")?;

        let mut world = World {
            now,
            players: &mut players,
            sound: &mut sound,
            assets: &assets,
        };

        // Handle requests
        state = state.handle(&mut requests, &mut world).await;

        // Play the game
        state = state.update(&mut world, duration);

        // Publish updated status info
        info.publish(Info {
            mode: *state::GAME_MODE.lock(),
            state: GameState::Waiting { ready: Vec::new() },
        });

        last = now;
    }
}
