use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::{anyhow, Result};
use futures::{Stream, StreamExt};
use tokio::io::unix::AsyncFd;
use udev::EventType;

#[derive(Debug, Clone, Copy)]
pub enum Bus {
    USB,
    BLUETOOTH,
    UNKNOWN,
}

pub enum Model {
    ZCM1,
    ZCM2,
}

#[derive(Debug)]
pub struct Device {
    path: PathBuf,

    bus: Bus,

    vendor_id: u16,
    product_id: u16,

    address: String,
    controller: String,
}

#[derive(Debug)]
pub enum Event<'a> {
    Added(&'a Device),
    Removed(Device),
}

pub struct Monitor {
    socket: udev::MonitorSocket,
    devices: HashMap<PathBuf, Device>,
}

impl Monitor {
    const BUS_USB: u8 = 0x03;
    const BUS_BLUETOOTH: u8 = 0x05;

    const PSMOVE_VID: u16 = 0x054c;
    const PSMOVE_PS3_PID: u16 = 0x03d5;
    const PSMOVE_PS4_PID: u16 = 0x0c5e;

    pub fn new() -> Result<Self> {
        let mut enumerator = udev::Enumerator::new()?;
        enumerator.match_subsystem("hidraw")?;
        let devices = enumerator.scan_devices()?
            .filter_map(|raw_device| {
                let path = raw_device.devnode()?.to_path_buf();
                return Self::device(path, &raw_device)
                    .map(|device| if device.vendor_id == Self::PSMOVE_VID &&
                        (device.product_id == Self::PSMOVE_PS3_PID || device.product_id == Self::PSMOVE_PS4_PID) {
                        Some((device.path.clone(), device))
                    } else {
                        None
                    })
                    .transpose();
            })
            .collect::<Result<_>>()?;

        let socket = udev::MonitorBuilder::new()?
            .match_subsystem("hidraw")?
            .listen()?;

        return Ok(Self {
            socket,
            devices,
        });
    }

    pub fn update(&mut self) -> Result<Option<Event>> {
        while let Some(event) = self.socket.next() {
            let path = if let Some(path) = event.devnode() {
                path.to_path_buf()
            } else {
                continue;
            };

            match event.event_type() {
                EventType::Add => {
                    let device = Self::device(path, &event)?;
                    if device.vendor_id == Self::PSMOVE_VID &&
                        (device.product_id == Self::PSMOVE_PS3_PID || device.product_id == Self::PSMOVE_PS4_PID) {
                        let device = self.devices.entry(device.path.clone()).or_insert(device);
                        return Ok(Some(Event::Added(device)));
                    }
                }
                EventType::Remove => {
                    if let Some(device) = self.devices.remove(&path) {
                        return Ok(Some(Event::Removed(device)));
                    }
                }

                _ => {
                    continue;
                }
            }
        }

        return Ok(None);
    }

    pub fn devices(&self) -> &HashMap<PathBuf, Device> {
        return &self.devices;
    }

    fn device(path: PathBuf, raw_device: &udev::Device) -> Result<Device> {
        let hid_device = raw_device.parent_with_subsystem("hid")?
            .ok_or_else(|| anyhow!("Not a HID device"))?;
        // dump(&hid_device);

        let id = hid_device.property_value("HID_ID")
            .ok_or_else(|| anyhow!("No HID_ID"))?
            .to_string_lossy();

        let (bus, id) = id.split_once(":")
            .ok_or_else(|| anyhow!("Illegal HID_ID format"))?;
        let (vendor_id, product_id) = id.split_once(":")
            .ok_or_else(|| anyhow!("Illegal HID_ID format"))?;

        let bus = match u8::from_str_radix(bus, 16) {
            Ok(Self::BUS_USB) => Bus::USB,
            Ok(Self::BUS_BLUETOOTH) => Bus::BLUETOOTH,
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
