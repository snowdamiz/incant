//! Constant working-memory PCM staging. Only a complete accepted run can publish.
use sha2::{Digest, Sha256};
use std::{
    io::{self, Write},
    path::Path,
};

const ENCODE_SAMPLES: usize = 2048;

pub struct PcmOutput {
    stream: Stream<tempfile::NamedTempFile>,
}
impl PcmOutput {
    pub fn new(path: &Path, max_frames: u64) -> io::Result<Self> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let file = tempfile::Builder::new()
            .prefix(".incant-pcm-")
            .tempfile_in(parent)?;
        Ok(Self {
            stream: Stream::new(file, max_frames),
        })
    }
    pub fn append(&mut self, samples: &[f32]) -> io::Result<()> {
        self.stream.append(samples)
    }
    pub fn frames(&self) -> u64 {
        self.stream.frames
    }
    pub fn sha256(&self) -> String {
        format!("{:x}", self.stream.hash.clone().finalize())
    }
    /// Sync the completed staging file and atomically publish without replacing
    /// an existing destination. Drop cleans staging on errors/unwind. A separate
    /// checkpoint publication is not part of this file transaction.
    pub fn publish(self, path: &Path) -> io::Result<()> {
        if self.stream.failed {
            return Err(io::Error::other("PCM staging failed"));
        }
        self.stream.writer.as_file().sync_all()?;
        self.stream
            .writer
            .persist_noclobber(path)
            .map_err(|e| e.error)?;
        Ok(())
    }
}

struct Stream<W> {
    writer: W,
    hash: Sha256,
    frames: u64,
    max_frames: u64,
    failed: bool,
}
impl<W: Write> Stream<W> {
    fn new(writer: W, max_frames: u64) -> Self {
        Self {
            writer,
            hash: Sha256::new(),
            frames: 0,
            max_frames,
            failed: false,
        }
    }
    fn append(&mut self, samples: &[f32]) -> io::Result<()> {
        if self.failed {
            return Err(io::Error::other("PCM staging failed"));
        }
        let result = self.append_inner(samples);
        self.failed |= result.is_err();
        result
    }
    fn append_inner(&mut self, samples: &[f32]) -> io::Result<()> {
        let frames = u64::try_from(samples.len() / 2)
            .ok()
            .and_then(|count| self.frames.checked_add(count))
            .filter(|count| *count <= self.max_frames && count.checked_mul(8).is_some());
        let frames = frames
            .filter(|_| samples.len().is_multiple_of(2))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "output frame bound"))?;
        let mut bytes = [0_u8; ENCODE_SAMPLES * 4];
        for samples in samples.chunks(ENCODE_SAMPLES) {
            for (sample, output) in samples.iter().zip(bytes.as_chunks_mut::<4>().0) {
                output.copy_from_slice(&sample.to_le_bytes());
            }
            let bytes = &bytes[..samples.len() * 4];
            self.writer.write_all(bytes)?;
            self.hash.update(bytes);
        }
        self.frames = frames;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
