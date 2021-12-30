use anyhow::Result;

use crate::games::{Game, Lobby};
use crate::psmove::Controller;
use crate::sound::Sound;
use crate::state::StateMachine;

pub mod psmove;
pub mod state;
pub mod sound;
pub mod games;

#[tokio::main]
async fn main() -> Result<()> {
    // let mut monitor = psmove::hid::Monitor::new()?;
    //
    // while let Some(event) = monitor.update()? {
    //     println!("Event: {:?}", event);
    // }
    //
    // for dev in monitor.devices() {
    //     println!("Device: {:?}", dev);
    // }

    let sound = Sound::init()?;

    let controllers = vec![
        Controller::new("/dev/hidraw6").await?,
    ];

    let mut data = state::Data {
        game: Game::Joust,
        sound,
        controllers,
    };

    let mut state = StateMachine::new(Lobby::new(), &mut data);

    while state.is_running() {
        for controller in data.controllers.iter_mut() {
            controller.update().await?;
        }

        state.update(&mut data);
    }

    return Ok(());
}
