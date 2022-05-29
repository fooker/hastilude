use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sample, Source};
use tracing::instrument;

use crate::engine::assets::{Asset, AssetLoader};

struct DynamicSource<I> {
    input: I,

    speed: Arc<AtomicI8>,
    stopped: Arc<AtomicBool>,
}

impl<I> DynamicSource<I>
    where
        I: Source,
        I::Item: Sample,
{
    const MAX_FRAME_LEN: usize = 1024;

    pub fn new(input: I) -> Self {
        return Self {
            input,
            speed: Arc::new(AtomicI8::new(0)),
            stopped: Arc::new(AtomicBool::new(false)),
        };
    }

    fn speed_handle(&self) -> Arc<AtomicI8> {
        return self.speed.clone();
    }

    fn stopped_handle(&self) -> Arc<AtomicBool> {
        return self.stopped.clone();
    }
}

impl<I> Iterator for DynamicSource<I>
    where
        I: Source,
        I::Item: Sample,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        return if self.stopped.load(Ordering::SeqCst) {
            None
        } else {
            self.input.next()
        };
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        return self.input.size_hint();
    }
}

impl<I> ExactSizeIterator for DynamicSource<I>
    where
        I: Source + ExactSizeIterator,
        I::Item: Sample,
{}

impl<I> Source for DynamicSource<I>
    where
        I: Source,
        I::Item: Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        return Some(self.input.current_frame_len()
            .map_or(Self::MAX_FRAME_LEN,
                    |frame_len| frame_len.max(Self::MAX_FRAME_LEN)));
    }

    fn channels(&self) -> u16 {
        return self.input.channels();
    }

    fn sample_rate(&self) -> u32 {
        let speed = self.speed.load(Ordering::Relaxed) as i32; // [-128, 127]: 0 => 0
        let speed = (speed + 256) as u32;                            // [128, 383]: 0 => 256
        return (self.input.sample_rate() * speed / 256) as u32;
    }

    fn total_duration(&self) -> Option<Duration> {
        return None;
    }
}

pub struct Sound {
    #[allow(unused)]
    output: OutputStream,
    handle: OutputStreamHandle,
}

pub struct Playback {
    speed: Arc<AtomicI8>,
    stopped: Arc<AtomicBool>,
}

impl Playback {
    pub fn speed(&mut self, speed: f32) {
        let speed = speed.clamp(0.5, 1.5) * 256.0 - 256.0;
        self.speed.store(speed as i8, Ordering::Relaxed);
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
}

pub type Music = Decoder<BufReader<File>>;

impl AssetLoader for Music {
    type Asset = Music;

    fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        return Ok(Decoder::new(BufReader::new(File::open(path)?))?);
    }
}

impl Sound {
    #[instrument(level = "debug")]
    pub fn init() -> Result<Self> {
        let (output, handle) = OutputStream::try_default()
            .context("Failed to open default sound output stream")?;

        return Ok(Self {
            output,
            handle,
        });
    }

    #[instrument(level = "debug", skip(self))]
    pub fn music(&self, asset: &Asset<Music>) -> Playback {
        let source = asset
            .load()
            .repeat_infinite()
            .fade_in(Duration::from_secs(1));

        let source = DynamicSource::new(source);
        let music = Playback {
            speed: source.speed_handle(),
            stopped: source.stopped_handle(),
        };

        self.handle.play_raw(source.convert_samples())
            .expect("Output dropped");

        return music;
    }
}
