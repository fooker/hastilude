use std::os::unix::prelude::AsRawFd;

use anyhow::Result;
use async_trait::async_trait;
use packed_struct::prelude::{bits::ByteArray, PackedStruct, PackedStructSlice};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub mod zcm1;

#[async_trait]
pub trait Getter<R: Report> {
    async fn get(f: &mut File) -> Result<R>;
}

#[async_trait]
pub trait Setter<R: Report> {
    async fn set(f: &mut File, report: R) -> Result<()>;
}

pub trait Report: PackedStruct + Sized {
    const REPORT_ID: u8;

    fn parse(data: &[u8]) -> Result<Self> {
        return Ok(Self::unpack_from_slice(&data)?);
    }
}

#[async_trait]
pub trait Get: Report {
    type Getter: self::Getter<Self>;

    async fn get(f: &mut File) -> Result<Self> {
        return Self::Getter::get(f).await;
    }
}

#[async_trait]
pub trait Set: Report {
    type Setter: self::Setter<Self>;

    async fn set(f: &mut File, report: Self) -> Result<()> {
        return Self::Setter::set(f, report).await;
    }
}

pub struct Primary {}

#[async_trait]
impl<R: Report> Getter<R> for Primary {
    async fn get(f: &mut File) -> Result<R> {
        let mut buffer = vec![0u8; R::ByteArray::len() + 1];
        f.read_exact(&mut buffer).await?;

        assert_eq!(buffer[0], R::REPORT_ID);

        return Ok(R::unpack_from_slice(&buffer[1..])?);
    }
}

#[async_trait]
impl<R> Setter<R> for Primary
    where
        R: Report + Send + 'static
{
    async fn set(f: &mut File, report: R) -> Result<()> {
        let mut data = vec![0u8; R::ByteArray::len() + 1]; // Make this static allocate
        data[0] = R::REPORT_ID;
        report.pack_to_slice(&mut data[1..])?;

        f.write_all(&data).await?;

        return Ok(());
    }
}

pub struct Feature {}

impl Feature {
    const IOC_HIDRAW_MAGIC: char = 'H';
    const IOC_HIDRAW_SEND_FEATURE_REPORT: u8 = 0x06;
    const IOC_HIDRAW_GET_FEATURE_REPORT: u8 = 0x07;
}

#[async_trait]
impl<R: Report> Getter<R> for Feature {
    async fn get(f: &mut File) -> Result<R> {
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
}

#[async_trait]
impl<R> Setter<R> for Feature
    where
        R: Report + Send + 'static
{
    async fn set(f: &mut File, report: R) -> Result<()> {
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
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct Address {
    data: [u8; 6],
}

impl Address {
    pub fn as_string(&self) -> String {
        return format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                       self.data[5], self.data[4], self.data[3], self.data[2], self.data[1], self.data[0]);
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        return &self.data;
    }
}

impl AsRef<[u8;6]> for Address {
    fn as_ref(&self) -> &[u8;6] {
        return &self.data;
    }
}
