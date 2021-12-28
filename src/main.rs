use anyhow::Result;

use crate::psmove::{Controller, Feedback};

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

        fn color(v: f32) -> u8 {
            return (v.abs().clamp(0.0, 1.0) * 255.0) as u8;
        }

        controller.feedback(Feedback::new()
            .with_color(color(controller.input().accelerometer.x),
                        color(controller.input().accelerometer.y),
                        color(controller.input().accelerometer.z))
            .with_rumble((controller.input().buttons.trigger.1 * 255.0) as u8));
    }
}
