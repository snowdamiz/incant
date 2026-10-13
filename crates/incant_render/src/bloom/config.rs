//! Admission is independent of a GPU and occurs before resource allocation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bloom {
    /// Scene-linear Rec.709 luminance threshold, in [0, 65504].
    pub threshold: f32,
    /// Nonnegative filtered radiance gain, in [0, 8]. Zero uses the original output.
    pub intensity: f32,
    /// Number of normalized pyramid levels, in [1, 6], truncated at 1x1.
    pub levels: u32,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BloomError {
    #[error("bloom requires finite threshold in [0, 65504], intensity in [0, 8], and 1..=6 levels")]
    Configuration,
    #[error("bloom exceeds the 4-megapixel, 8192-dimension, or 32-MiB pyramid budget")]
    TargetSize,
    #[error("bloom viewport is outside its scene source")]
    Viewport,
    #[error("bloom output has no approved cooked-output artifact")]
    CookedOutput,
    #[error("bloom target cache lock failed")]
    CacheLock,
}

impl Bloom {
    pub fn validate(self) -> Result<(), BloomError> {
        if !self.threshold.is_finite()
            || !(0.0..=65504.0).contains(&self.threshold)
            || !self.intensity.is_finite()
            || !(0.0..=8.0).contains(&self.intensity)
            || !(1..=6).contains(&self.levels)
        {
            return Err(BloomError::Configuration);
        }
        Ok(())
    }
}

pub(super) fn admission(config: Bloom, cooked_output: bool) -> Result<(), BloomError> {
    config.validate()?;
    if cooked_output {
        return Err(BloomError::CookedOutput);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Plan {
    pub output: [u32; 2],
    pub sizes: Vec<[u32; 2]>,
    pub bytes: u64,
}
impl Plan {
    pub fn new(source: [u32; 2], levels: u32, device_limit: u32) -> Result<Self, BloomError> {
        let [width, height] = source;
        if width == 0
            || height == 0
            || width > device_limit.min(8192)
            || height > device_limit.min(8192)
            || u64::from(width) * u64::from(height) > 4 * 1024 * 1024
            || !(1..=6).contains(&levels)
        {
            return Err(BloomError::TargetSize);
        }
        // Fixed stack planning precedes even the small heap allocation. One down
        // texture per level and one reconstruction per nonterminal level, RGBA16F.
        let mut sizes = [[0; 2]; 6];
        let mut size = source;
        let mut count = 0;
        let mut bytes = 0u64;
        for entry in sizes.iter_mut().take(levels as usize) {
            size = size.map(|n| n.div_ceil(2));
            *entry = size;
            count += 1;
            bytes = bytes
                .checked_add(u64::from(size[0]) * u64::from(size[1]) * 16)
                .ok_or(BloomError::TargetSize)?;
            if size == [1, 1] {
                break;
            }
        }
        bytes -= u64::from(size[0]) * u64::from(size[1]) * 8;
        if bytes > 32 * 1024 * 1024 {
            return Err(BloomError::TargetSize);
        }
        Ok(Self {
            output: source,
            sizes: sizes[..count].to_vec(),
            bytes,
        })
    }

    /// The first pass maps physical footprints through the exact DRS factor.
    /// Allocation rounding must not become an extra scale or phase adjustment.
    pub fn source_span(
        &self,
        size: [u32; 2],
        rect: [f32; 4],
        scale: crate::resolution::ResolutionScale,
    ) -> Result<[f32; 2], BloomError> {
        let span = self.output.map(|v| v as f32 * scale.fraction());
        if size != span.map(|v| v.ceil() as u32) {
            return Err(BloomError::TargetSize);
        }
        Self::viewport(size, rect)?;
        Ok(span)
    }

    pub fn viewport(source: [u32; 2], rect: [f32; 4]) -> Result<(), BloomError> {
        if rect.iter().any(|v| !v.is_finite())
            || rect[0] < 0.0
            || rect[1] < 0.0
            || rect[2] <= 0.0
            || rect[3] <= 0.0
            || rect[0] + rect[2] > source[0] as f32
            || rect[1] + rect[3] > source[1] as f32
        {
            return Err(BloomError::Viewport);
        }
        Ok(())
    }
}
