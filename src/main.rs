use anyhow::{Result, Context};

use crate::games::{Game, Lobby};
use crate::psmove::Controller;
use crate::sound::Sound;
use crate::state::StateMachine;
use crate::assets::Assets;
use tracing::Level;

pub mod psmove;
pub mod state;
pub mod sound;
pub mod games;
pub mod assets;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .with_ansi(true)
        .pretty()
        .init();

    // let mut monitor = psmove::hid::Monitor::new()?;
    //
    // while let Some(event) = monitor.update()? {
    //     println!("Event: {:?}", event);
    // }
    //
    // for dev in monitor.devices() {
    //     println!("Device: {:?}", dev);
    // }

    let sound = Sound::init()
        .context("Failed to initialize sound")?;

    let controllers = vec![
        Controller::new("/dev/hidraw6").await?,
    ];

    let assets = Assets::init(std::env::current_dir()?)
        .context("Failed to initialize assets")?;

    let mut data = state::Data {
        game: Game::Joust,
        sound,
        controllers,
        assets,
    };

    let mut state = StateMachine::new(Lobby::new(), &mut data);

    while state.is_running() {
        for controller in data.controllers.iter_mut() {
            controller.update().await
                .with_context(|| format!("Failed to update controller: {}", controller.serial()))?;
        }

        state.update(&mut data);
    }

    return Ok(());
}
