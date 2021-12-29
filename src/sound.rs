use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};
use std::time::Duration;

use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sample, Source};

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

    pub fn speed_handle(&self) -> Arc<AtomicI8> {
        return self.speed.clone();
    }

    pub fn stopped_handle(&self) -> Arc<AtomicBool> {
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
        let speed = self.speed.load(Ordering::Relaxed) as i64 + 255;
        return (self.input.sample_rate() as i64 * speed / 255) as u32;
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

pub struct Music {
    speed: Arc<AtomicI8>,
    stopped: Arc<AtomicBool>,
}

impl Music {
    pub fn speed(&mut self, speed: i8) {
        self.speed.store(speed, Ordering::SeqCst);
    }
}

impl Drop for Music {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
}

impl Sound {
    pub fn init() -> Result<Self> {
        let (output, handle) = OutputStream::try_default()?;

        return Ok(Self {
            output,
            handle,
        });
    }

    pub fn music(&self, path: impl AsRef<Path>) -> Result<Music> {
        let source = Decoder::new(BufReader::new(File::open(path)?))?
            .repeat_infinite()
            .fade_in(Duration::from_secs(1));

        let source = DynamicSource::new(source);
        let music = Music {
            speed: source.speed_handle(),
            stopped: source.stopped_handle(),
        };

        self.handle.play_raw(source.convert_samples())?;

        return Ok(music);
    }
}
