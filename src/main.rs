use anyhow::Result;

use crate::psmove::Controller;

pub mod psmove;

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

    let mut controller = Controller::new("/dev/hidraw0").await?;

    loop {
        controller.update().await?;

        // std::thread::sleep(Duration::from_millis(10));
    }

    return Ok(());
}
