//! Versioned conservative wall-time intervals for successful simulation frames.
pub const FRAME_BUCKET_WIDTH_NS: u64 = 10_000;
pub const FRAME_MAX_NS: u64 = 100_000_000;
pub const FRAME_BUCKET_COUNT: usize = (FRAME_MAX_NS / FRAME_BUCKET_WIDTH_NS) as usize + 1;

#[derive(Debug)]
#[cfg_attr(feature = "summary", derive(serde::Serialize))]
pub struct FramePhaseReport {
    pub name: &'static str,
    pub samples: u64,
    pub missing_frames: u64,
    pub invalid_frames: u64,
    pub overflow_samples: u64,
    pub p95_lower_ns: Option<u64>,
    pub p95_upper_ns: Option<u64>,
    /// Bin 0 is exactly zero. Bin i > 0 is ((i-1)*width, i*width].
    pub counts: Vec<u64>,
}
#[derive(Debug)]
#[cfg_attr(feature = "summary", derive(serde::Serialize))]
pub struct FrameWallReport {
    pub version: u32,
    pub unit: &'static str,
    pub bucket_width_ns: u64,
    pub max_ns: u64,
    pub attempted_frames: u64,
    pub successful_frames: u64,
    pub failed_frames: u64,
    pub invalid_frames: u64,
    pub active_frames: u64,
    pub saturated: bool,
    pub phases: Vec<FramePhaseReport>,
}
