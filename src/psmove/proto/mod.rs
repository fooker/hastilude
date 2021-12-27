use anyhow::Result;
use packed_struct::{PackedStruct, PackedStructSlice};

pub mod zcm1;

pub trait Accessor<R: Report> {}

pub trait Report: PackedStruct + Sized {
    const REPORT_ID: u8;

    fn parse(data: &[u8]) -> Result<Self> {
        return Ok(Self::unpack_from_slice(&data)?);
    }
}

pub struct InputAccessor {}

