use std::io::Read;
use std::mem;
use std::time::Duration;

use crate::Source;

use claxon::FlacReader;

/// Decoder for the Flac format.
pub struct FlacDecoder<R>
where
    R: Read,
{
    reader: FlacReader<R>,
    current_block: Vec<i32>,
    current_block_channel_len: usize,
    current_block_off: usize,
    bits_per_sample: u32,
    sample_rate: u32,
    channels: u16,
    samples: Option<u64>,
}

impl<R> FlacDecoder<R>
where
    R: Read,
{
    /// Attempts to decode the data as Flac.
    pub fn new(data: R) -> claxon::Result<FlacDecoder<R>> {
        let reader = FlacReader::new(data)?;
        let spec = reader.streaminfo();

        Ok(FlacDecoder {
            reader,
            current_block: Vec::with_capacity(
                spec.max_block_size as usize * spec.channels as usize,
            ),
            current_block_channel_len: 1,
            current_block_off: 0,
            bits_per_sample: spec.bits_per_sample,
            sample_rate: spec.sample_rate,
            channels: spec.channels as u16,
            samples: spec.samples,
        })
    }
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }
}

impl<R> Source for FlacDecoder<R>
where
    R: Read,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.channels
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        // `samples` in FLAC means "inter-channel samples" aka frames
        // so we do not divide by `self.channels` here.
        self.samples
            .map(|s| Duration::from_micros(s * 1_000_000 / self.sample_rate as u64))
    }
}

impl<R> Iterator for FlacDecoder<R>
where
    R: Read,
{
    type Item = i32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.current_block_off < self.current_block.len() {
                // Read from current block.
                let real_offset = (self.current_block_off % self.channels as usize)
                    * self.current_block_channel_len
                    + self.current_block_off / self.channels as usize;
                let val = self.current_block[real_offset] << (32 - self.bits_per_sample);
                self.current_block_off += 1;
                return Some(val);
            }

            // Load the next block.
            self.current_block_off = 0;
            let buffer = mem::replace(&mut self.current_block, Vec::new());
            match self.reader.blocks().read_next_or_eof(buffer) {
                Ok(Some(block)) => {
                    self.current_block_channel_len = (block.len() / block.channels()) as usize;
                    self.current_block = block.into_buffer();
                }
                _ => return None,
            }
        }
    }
}
