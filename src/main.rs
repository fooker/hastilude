pub mod psmove;

use anyhow::Result;
use crate::psmove::Controller;
use std::time::Duration;

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

    let mut controller = Controller::new("/dev/hidraw0")?;

    loop {
        controller.update()?;

        // std::thread::sleep(Duration::from_millis(10));
    }

    return Ok(());
}
