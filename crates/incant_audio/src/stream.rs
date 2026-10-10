use incant_assets::{AUDIO_CHUNK_FRAMES, AssetError, CookedAudio};
use kira::{Frame, sound::streaming::Decoder};
use std::sync::Arc;

/// Bounded, seekable decoding of validated PCM. Kira invokes this on its decoder
/// worker, never on the realtime sample callback. Concurrent voices have their
/// own cursor and share an immutable clip version.
pub struct ClipDecoder {
    clip: Arc<CookedAudio>,
    position: usize,
}
impl ClipDecoder {
    pub fn new(clip: Arc<CookedAudio>) -> Self {
        Self { clip, position: 0 }
    }
}
impl Decoder for ClipDecoder {
    type Error = AssetError;
    fn sample_rate(&self) -> u32 {
        self.clip.metadata.sample_rate
    }
    fn num_frames(&self) -> usize {
        self.clip.metadata.frames as usize
    }
    fn decode(&mut self) -> Result<Vec<Frame>, Self::Error> {
        if self.position >= self.num_frames() {
            return Ok(vec![]);
        }
        let index = self.position / AUDIO_CHUNK_FRAMES;
        let frames = self.clip.read_chunk(index)?;
        let skip = self.position % AUDIO_CHUNK_FRAMES;
        let out = frames[skip..]
            .iter()
            .map(|f| Frame::new(f[0], f[1]))
            .collect();
        self.position = index * AUDIO_CHUNK_FRAMES + frames.len();
        Ok(out)
    }
    fn seek(&mut self, index: usize) -> Result<usize, Self::Error> {
        self.position = index.min(self.num_frames());
        Ok(self.position)
    }
}
