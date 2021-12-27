use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::prelude::{AsRawFd, OpenOptionsExt};
use std::path::Path;

use anyhow::{bail, Result};
use cgmath::{ElementWise, Zero};
use packed_struct::PackedStructSlice;
use packed_struct::prelude::bits::ByteArray;

use crate::psmove::proto::Report;
use crate::psmove::proto::zcm1::{GetCalibration, GetCalibrationInner};

pub mod hid;

#[derive(Debug, Default, Clone)]
pub struct Buttons {
    pub square: bool,
    pub triangle: bool,
    pub cross: bool,
    pub circle: bool,

    pub start: bool,
    pub select: bool,

    pub logo: bool,
    pub swoosh: bool,

    pub trigger: (bool, f32),
}

#[derive(Debug, Clone)]
pub struct Input {
    pub accelerometer: cgmath::Vector3<f32>,
    pub gyroscope: cgmath::Vector3<f32>,

    pub buttons: Buttons,
}

impl Default for Input {
    fn default() -> Self {
        return Self {
            accelerometer: cgmath::Vector3::zero(),
            gyroscope: cgmath::Vector3::zero(),
            buttons: Default::default(),
        };
    }
}

#[derive(Debug, Clone)]
pub struct Calibration {
    accelerometer_m: cgmath::Vector3<f32>,
    accelerometer_b: cgmath::Vector3<f32>,

    gyroscope: cgmath::Vector3<f32>,
}

impl From<GetCalibrationInner> for Calibration {
    fn from(report: GetCalibrationInner) -> Self {
        let accel_min = cgmath::Vector3 {
            x: report.accel[1].x(),
            y: report.accel[5].y(),
            z: report.accel[2].z(),
        };

        let accel_max = cgmath::Vector3 {
            x: report.accel[3].x(),
            y: report.accel[4].y(),
            z: report.accel[0].z(),
        };

        let accelerometer_m = 2.0 / (accel_max - accel_min);
        let accelerometer_b = -accelerometer_m.mul_element_wise(accel_min) + cgmath::Vector3::new(-1.0, -1.0, -1.0);

        const FACTOR: f32 = 80.0 * (2.0 * std::f32::consts::PI) / 60.0;

        let gyroscope = FACTOR / (cgmath::Vector3 {
            x: report.gyro_x.x(),
            y: report.gyro_y.y(),
            z: report.gyro_z.z(),
        } - cgmath::Vector3::from(report.gyro_bias));

        return Self {
            accelerometer_m,
            accelerometer_b,
            gyroscope,
        };
    }
}

pub struct Controller {
    f: File,

    pub input: Input,

    calibration: Calibration,
}

mod proto;

impl Controller {
    const IOC_HIDRAW_MAGIC: char = 'H';
    const IOC_HIDRAW_SEND_FEATURE_REPORT: u8 = 0x06;
    const IOC_HIDRAW_GET_FEATURE_REPORT: u8 = 0x07;

    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)?;

        // Collect calibration data from device
        let calibration = GetCalibration::stitch([
            &Self::get_feature_report(&f)?,
            &Self::get_feature_report(&f)?,
            &Self::get_feature_report(&f)?,
        ])?.into();

        return Ok(Self {
            f,
            input: Default::default(),
            calibration,
        });
    }

    fn send_feature_report<R: Report>(f: &impl AsRawFd, report: &R) -> Result<()> {
        let ioc = nix::ioc!(nix::sys::ioctl::READ | nix::sys::ioctl::WRITE,
            Self::IOC_HIDRAW_MAGIC,
            Self::IOC_HIDRAW_SEND_FEATURE_REPORT,
            R::ByteArray::len() + 1);

        let mut data = vec![0u8; R::ByteArray::len() + 1]; // Make this static allocate
        data[0] = R::REPORT_ID;
        report.pack_to_slice(&mut data[1..])?;

        nix::errno::Errno::result(unsafe {
            nix::libc::ioctl(f.as_raw_fd(), ioc, data.as_slice())
        })?;

        return Ok(());
    }

    fn get_feature_report<R: Report>(f: &impl AsRawFd) -> Result<R> {
        let ioc = nix::ioc!(nix::sys::ioctl::READ | nix::sys::ioctl::WRITE,
            Self::IOC_HIDRAW_MAGIC,
            Self::IOC_HIDRAW_GET_FEATURE_REPORT,
            R::ByteArray::len() + 1);

        let mut data = vec![0u8; R::ByteArray::len() + 1]; // Make this static allocate
        data[0] = R::REPORT_ID;

        nix::errno::Errno::result(unsafe {
            nix::libc::ioctl(f.as_raw_fd(), ioc, data.as_mut_slice())
        })?;

        return Ok(R::unpack_from_slice(&data[1..])?);
    }

    pub fn update(&mut self) -> Result<()> {
        let mut buffer = [0u8; 4096];

        // Read in reports from device
        while let Some(buffer) = self.f.read(&mut buffer)
            .map(|size| Some(&buffer[0..size]))
            .or_else(|err| if err.kind() == io::ErrorKind::WouldBlock {
                Ok(None)
            } else {
                Err(err)
            })? {
            let req = buffer[0];
            let data = &buffer[1..];

            match req {
                proto::zcm1::REPORT_GET_INPUT => {
                    let input = proto::zcm1::GetInput::parse(data)?;

                    fn avg(v1: cgmath::Vector3<f32>, v2: cgmath::Vector3<f32>) -> cgmath::Vector3<f32> {
                        return (v1 + v2) / 2.0;
                    }

                    self.input.accelerometer = avg(input.accel_1.into(), input.accel_2.into())
                        .mul_element_wise(self.calibration.accelerometer_m)
                        .add_element_wise(self.calibration.accelerometer_b);

                    self.input.gyroscope = avg(input.gyro_1.into(), input.gyro_2.into())
                        .mul_element_wise(self.calibration.gyroscope);

                    fn bit(buttons: impl Into<u32>, bit: usize) -> bool {
                        return buttons.into() & (1 << bit) != 0;
                    }

                    let trigger = ((input.trigger_1 as f32) / (0xFF as f32) + (input.trigger_1 as f32) / (0xFF as f32)) / 2.0;

                    self.input.buttons = Buttons {
                        square: bit(input.buttons, 15),
                        triangle: bit(input.buttons, 12),
                        cross: bit(input.buttons, 14),
                        circle: bit(input.buttons, 13),
                        start: bit(input.buttons, 3),
                        select: bit(input.buttons, 0),
                        logo: bit(input.buttons, 16),
                        swoosh: bit(input.buttons, 19),
                        trigger: (bit(input.buttons, 20), trigger),
                    };
                }

                _ => {
                    bail!("Unsupported request type received: {:02x}", req);
                }
            }

            // fn color(v: f32) -> u8 {
            //     return (v.abs().clamp(0.0, 1.0) * 255.0) as u8;
            // }
            //
            // let led = SetLED::withColor(color(self.input.accelerometer.x),
            //                             color(self.input.accelerometer.y),
            //                             color(self.input.accelerometer.z));
            //
            // {
            //     let mut data = vec![0u8; <SetLED as PackedStruct>::ByteArray::len() + 1]; // Make this static allocate
            //     data[0] = SetLED::ID;
            //     led.pack_to_slice(&mut data[1..])?;
            //
            //     self.f.write_all(&data)?;
            // }
        }

        return Ok(());
    }
}