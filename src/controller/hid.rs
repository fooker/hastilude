use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::{anyhow, Result};
use futures::{Stream, TryStreamExt};
use tokio::io::unix::AsyncFd;
use udev::EventType;

const BUS_USB: u8 = 0x03;
const BUS_BLUETOOTH: u8 = 0x05;

const PSMOVE_VID: u16 = 0x054c;
const PSMOVE_PS3_PID: u16 = 0x03d5;
const PSMOVE_PS4_PID: u16 = 0x0c5e;

#[derive(Debug, Clone, Copy)]
pub enum Bus {
    USB,
    BLUETOOTH,
    UNKNOWN,
}

#[derive(Debug)]
pub struct Device {
    pub path: PathBuf,

    pub bus: Bus,

    pub vendor_id: u16,
    pub product_id: u16,

    pub address: String,
    pub controller: String,
}

#[derive(Debug)]
pub enum Event {
    Added(Device),
    Removed(PathBuf),
}

pub type Events = impl Stream<Item=Result<Event>>;

fn is_controller(device: &Device) -> bool {
    return device.vendor_id == PSMOVE_VID && (device.product_id == PSMOVE_PS3_PID || device.product_id == PSMOVE_PS4_PID);
}

pub fn monitor() -> Result<(Vec<Device>, Events)> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("hidraw")?;
    let devices = enumerator.scan_devices()?;

    let initial = devices
        .map(|device| {
            let path = if let Some(path) = device.devnode() {
                path.to_path_buf()
            } else {
                return Ok(None);
            };

            let device = self::device(path, &device)?;

            if !is_controller(&device) {
                return Ok(None);
            }

            return Ok(Some(device));
        }).filter_map(Result::transpose)
        .collect::<Result<_>>()?;

    let monitor = Monitor::new()?
        .try_filter_map(|event| async move {
            let path = if let Some(path) = event.devnode() {
                path.to_path_buf()
            } else {
                return Ok(None);
            };

            match event.event_type() {
                EventType::Add => {
                    let device = self::device(path, &event)?;

                    if !is_controller(&device) {
                        return Ok(None);
                    }

                    return Ok(Some(Event::Added(device)));
                }

                EventType::Remove => {
                    dump(&event);
                    return Ok(Some(Event::Removed(path)));
                }

                _ => {
                    return Ok(None);
                }
            }
        });

    return Ok((initial, Box::pin(monitor)));
}

struct Monitor {
    fd: AsyncFd<udev::MonitorSocket>,
}

impl Monitor {
    pub fn new() -> Result<Self> {
        let socket = udev::MonitorBuilder::new()?
            .match_subsystem("hidraw")?
            .listen()?;

        return Ok(Self {
            fd: AsyncFd::new(socket)?,
        });
    }
}

fn dump(device: &udev::Device) {
    println!("Device {}", device.syspath().to_string_lossy());

    for e in device.attributes() {
        println!("  Attr {} = {}", e.name().to_string_lossy(), e.value().to_string_lossy());
    }
    for e in device.properties() {
        println!("  Prop {} = {}", e.name().to_string_lossy(), e.value().to_string_lossy());
    }
}

fn device(path: PathBuf, raw_device: &udev::Device) -> Result<Device> {
    let hid_device = raw_device.parent_with_subsystem("hid")?
        .ok_or_else(|| anyhow!("Not a HID device"))?;

    let id = hid_device.property_value("HID_ID")
        .ok_or_else(|| anyhow!("No HID_ID"))?
        .to_string_lossy();

    let (bus, id) = id.split_once(":")
        .ok_or_else(|| anyhow!("Illegal HID_ID format"))?;
    let (vendor_id, product_id) = id.split_once(":")
        .ok_or_else(|| anyhow!("Illegal HID_ID format"))?;

    let bus = match u8::from_str_radix(bus, 16) {
        Ok(self::BUS_USB) => Bus::USB,
        Ok(self::BUS_BLUETOOTH) => Bus::BLUETOOTH,
        _ => Bus::UNKNOWN,
    };

    let vendor_id = u16::from_str_radix(vendor_id, 16)?;
    let product_id = u16::from_str_radix(product_id, 16)?;

    let address = hid_device.property_value("HID_UNIQ")
        .ok_or_else(|| anyhow!("No HID_UNIQ"))?
        .to_string_lossy();
    let controller = hid_device.property_value("HID_PHYS")
        .ok_or_else(|| anyhow!("No HID_PHYS"))?
        .to_string_lossy();

    return Ok(Device {
        path,
        bus,
        vendor_id,
        product_id,
        address: address.to_string(),
        controller: controller.to_string(),
    });
}

impl Stream for Monitor {
    type Item = Result<udev::Event>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.fd.poll_read_ready_mut(cx) {
            Poll::Ready(Ok(mut ready_guard)) => {
                ready_guard.clear_ready();
                return Poll::Ready(ready_guard.get_inner_mut().next().map(Ok));
            }

            Poll::Ready(Err(err)) => {
                return Poll::Ready(Some(Err(err.into())));
            }

            Poll::Pending => {
                return Poll::Pending;
            }
        }
    }
}
