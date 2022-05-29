use anyhow::Result;
use packed_struct::prelude::{Integer, packed_bits, PackedStruct};

// TODO: Check out https://crates.io/crates/deku for struct packing

use crate::controller::Feedback;
use crate::controller::proto::{Address, Feature, Get, Primary, Set};

use super::Report;

const REPORT_GET_INPUT: u8 = 0x01;
const REPORT_SET_LED: u8 = 0x06;
// const REPORT_SET_LED_PWM_FREQ: u8 = 0x03;
const REPORT_GET_BT_ADDR: u8 = 0x04;
// const REPORT_SET_BT_ADDR: u8 = 0x05;
const REPORT_GET_CALIBRATION: u8 = 0x10;
// const REPORT_SET_AUTH_CHALLENGE: u8 = 0xA0;
// const REPORT_GET_AUTH_RESPONSE: u8 = 0xA1;
// const REPORT_GET_EXT_DEVICE_INFO: u8 = 0xE0;
// const REPORT_SET_DFU_MODE: u8 = 0xF2;
// const REPORT_GET_FIRMWARE_INFO: u8 = 0xF9;

#[derive(PackedStruct, Debug, Copy, Clone)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct Vector {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

impl Vector {
    pub fn normalize(v: u16) -> f32 {
        return (v as f32) / (0x8000 as f32) - 1.0;
    }

    pub fn x(&self) -> f32 {
        return Self::normalize(self.x);
    }

    pub fn y(&self) -> f32 {
        return Self::normalize(self.y);
    }

    pub fn z(&self) -> f32 {
        return Self::normalize(self.z);
    }
}

impl From<Vector> for cgmath::Vector3<f32> {
    fn from(vec: Vector) -> Self {
        return cgmath::Vector3::new(vec.x(), vec.y(), vec.z());
    }
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct GetInput {
    pub buttons: Integer<u32, packed_bits::Bits<28>>,

    pub seq: Integer<u8, packed_bits::Bits<4>>,

    pub trigger_1: u8,
    pub trigger_2: u8,

    _reserved: [u8; 4],

    time_high: u8,

    pub battery: u8,

    #[packed_field(element_size_bytes = "6")]
    pub accel_1: Vector,

    #[packed_field(element_size_bytes = "6")]
    pub accel_2: Vector,

    #[packed_field(element_size_bytes = "6")]
    pub gyro_1: Vector,

    #[packed_field(element_size_bytes = "6")]
    pub gyro_2: Vector,

    temp: Integer<u16, packed_bits::Bits<12>>,

    magnet_x: Integer<u16, packed_bits::Bits<12>>,
    magnet_y: Integer<u16, packed_bits::Bits<12>>,
    magnet_z: Integer<u16, packed_bits::Bits<12>>,

    time_low: u8, // TODO: can this be a single field but split using packed_struct magic?

    pub extdata: [u8; 5],
}

impl Report for GetInput {
    const REPORT_ID: u8 = self::REPORT_GET_INPUT;
}

impl Get for GetInput { type Getter = Primary; }

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct SetLED {
    _reserved1: [u8; 1],

    pub r: u8,
    pub g: u8,
    pub b: u8,

    _reserved2: [u8; 1],

    pub rumble: u8,

    _reserved3: [u8; 2],
}

impl Report for SetLED {
    const REPORT_ID: u8 = self::REPORT_SET_LED;
}

impl Set for SetLED {
    type Setter = Primary;
}

impl SetLED {
    pub fn from(feedback: &Feedback) -> Self {
        return Self {
            _reserved1: [0],
            r: feedback.rgb.0,
            g: feedback.rgb.1,
            b: feedback.rgb.2,
            _reserved2: [0],
            rumble: feedback.rumble,
            _reserved3: [0, 0],
        };
    }
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct GetCalibration {
    pub index: u8,
    pub data: [u8; 47],
}

impl Report for GetCalibration {
    const REPORT_ID: u8 = self::REPORT_GET_CALIBRATION;
}

impl Get for GetCalibration {
    type Getter = Feature;
}

impl GetCalibration {
    pub fn stitch(data: [&Self; 3]) -> Result<GetCalibrationInner> {
        let data1 = data.iter().find(|report| report.index == 0x00);
        let data2 = data.iter().find(|report| report.index == 0x01);
        let data3 = data.iter().find(|report| report.index == 0x82);

        if let (Some(data1), Some(data2), Some(data3)) = (data1, data2, data3) {
            let mut data = [0; 141];
            data[0..47].copy_from_slice(&data1.data);
            data[47..94].copy_from_slice(&data2.data);
            data[94..141].copy_from_slice(&data3.data);

            return Ok(GetCalibrationInner::unpack(&data)?);
        } else {
            anyhow::bail!("Insufficient data");
        }
    }
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct GetCalibrationInner {
    _unknown01: [u8; 2],

    #[packed_field(element_size_bytes = "6")]
    pub accel: [Vector; 6],

    _unknown02: [u8; 2],
    #[packed_field(element_size_bytes = "6")]
    pub gyro_bias: Vector,

    _unknown04: [u8; 2],
    #[packed_field(element_size_bytes = "6")]
    _unknown05: Vector,

    _unknown06: [u8; 7],

    _unknown07: [u8; 1],
    _unknown08: [u8; 2],
    _unknown09: [u8; 2],
    _unknown10: [u8; 2],

    #[packed_field(element_size_bytes = "6")]
    pub gyro_x: Vector,

    _unknown11: [u8; 2],

    #[packed_field(element_size_bytes = "6")]
    pub gyro_y: Vector,

    _unknown12: [u8; 2],

    #[packed_field(element_size_bytes = "6")]
    pub gyro_z: Vector,

    _unknown13: [u8; 2],

    _unknown14: [u8; 12],
    _unknown15: [u8; 12],
    _unknown16: [u8; 4],
    _unknown17: [u8; 4],

    _unknown18: [u8; 17],
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct GetAddress {
    #[packed_field(element_size_bytes = "6")]
    pub controller: Address,

    #[packed_field(element_size_bytes = "6")]
    pub host: Address,
}

impl Report for GetAddress {
    const REPORT_ID: u8 = self::REPORT_GET_BT_ADDR;
}

impl Get for GetAddress {
    type Getter = Feature;
}
