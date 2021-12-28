use std::ops::Deref;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Result;
use cgmath::{ElementWise, Zero};
use tokio::fs::{File, OpenOptions};

use crate::psmove::proto::{Get, Set};
use crate::psmove::proto::zcm1::{GetCalibration, GetCalibrationInner, GetInput, SetLED};

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

struct Limiter<T> {
    value: T,
    dirty: bool,
    updated: Instant,
}

impl<T> Limiter<T> {
    const MIN_UPDATE: Duration = Duration::from_millis(110);
    const MAX_UPDATE: Duration = Duration::from_millis(4000);

    pub fn new(initial: T) -> Self {
        return Self {
            value: initial,
            dirty: true,
            updated: Instant::now(),
        };
    }

    pub fn set(&mut self, value: T) {
        self.value = value;
        self.dirty = true;
    }

    pub(self) fn update(&mut self) -> Option<&T> {
        let now = Instant::now();

        // Check if value has change but rate limit will not exceed or if value needs resending
        if (now.duration_since(self.updated) >= Self::MIN_UPDATE && self.dirty) ||
            now.duration_since(self.updated) >= Self::MAX_UPDATE {
            self.updated = now;
            return Some(&self.value);
        }

        return None;
    }
}

impl<T> Deref for Limiter<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return &self.value;
    }
}

impl<T> Default for Limiter<T>
    where
        T: Default
{
    fn default() -> Self {
        return Self::new(T::default());
    }
}

#[derive(Debug, Clone)]
pub struct Feedback {
    pub r: u8,
    pub g: u8,
    pub b: u8,

    pub rumble: u8,
}

impl Feedback {
    pub fn new() -> Self {
        return Self {
            r: 0,
            g: 0,
            b: 0,
            rumble: 0,
        };
    }

    pub fn with_color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.r = r;
        self.g = g;
        self.b = b;
        return self;
    }

    pub fn with_rumble(mut self, rumble: u8) -> Self {
        self.rumble = rumble;
        return self;
    }
}

impl Default for Feedback {
    fn default() -> Self {
        return Self::new();
    }
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

    /// Calibration data received from the controller
    calibration: Calibration,

    input: Input,

    feedback: Limiter<Feedback>,
}

mod proto;

impl Controller {
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .await?;

        // Collect calibration data from device
        let calibration = GetCalibration::stitch([
            &GetCalibration::get(&mut f).await?,
            &GetCalibration::get(&mut f).await?,
            &GetCalibration::get(&mut f).await?,
        ])?.into();

        return Ok(Self {
            f,
            calibration,
            input: Default::default(),
            feedback: Default::default(),
        });
    }

    pub async fn update(&mut self) -> Result<()> {
        // Send updates if required
        if let Some(feedback) = self.feedback.update() {
            let led = SetLED::from(feedback);
            SetLED::set(&mut self.f, led).await?;
        }

        // Read input report from device
        let input = GetInput::get(&mut self.f).await?;

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

        return Ok(());
    }

    pub fn input(&self) -> &Input {
        return &self.input;
    }

    pub fn feedback(&mut self, feedback: Feedback) {
        self.feedback.set(feedback);
    }
}