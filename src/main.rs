use std::time::Instant;

use anyhow::{Context, Result};
use tracing::Level;

use crate::engine::assets::Assets;
use crate::engine::players::Players;
use crate::engine::sound::Sound;
use crate::engine::state::{StateMachine, World};
use crate::games::GameType;
use crate::lobby::Lobby;
use crate::psmove::Controller;

pub mod psmove;
pub mod engine;
pub mod games;
pub mod lobby;
pub mod debug;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .with_ansi(true)
        .pretty()
        .init();

    let mut players = Players::init().await
        .context("Failed to initialize players")?;

    players.register(Controller::new("/dev/hidraw13").await?);
    players.register(Controller::new("/dev/hidraw14").await?);

    // let mut monitor = psmove::hid::Monitor::new()?;
    //
    // while let Some(event) = monitor.update()? {
    //     println!("Event: {:?}", event);
    // }
    //
    // for dev in monitor.devices() {
    //     println!("Device: {:?}", dev);
    // }

    let mut sound = Sound::init()
        .context("Failed to initialize sound")?;

    let assets = Assets::init(std::env::current_dir()?.join("assets"))
        .context("Failed to initialize assets")?;

    let mut last = Instant::now();

    // Initialize fresh state machine
    let mut state = StateMachine::new(Lobby::new(&mut players));

    loop {
        let now = Instant::now();
        let duration = now - last;

        players.update(duration).await
            .context("Failed to update players")?;

        state.update(&mut World {
            game: GameType::Joust,
            now,
            players: &mut players,
            sound: &mut sound,
            assets: &assets,
        }, duration);

        last = now;
    }
}
