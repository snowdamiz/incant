//! Opt-in native GPU timestamps. These measure instrumented queue intervals,
//! not CPU frame latency, presentation, or a portable strict execution barrier.
//! Results are bounded and asynchronous; an unread/full pool skips measurement.
mod pool;
mod recorder;
pub(crate) use recorder::Profiler;
pub(crate) use recorder::{Capture, PassQueries};
use serde::Serialize;

pub const GPU_TIMING_CAPACITY: usize = 3;
pub const GPU_PASS_COUNT: usize = 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GpuPass {
    InitializeScene,
    AssignLights,
    Shadows,
    Models,
    Sprites,
    Text,
    SceneResolve,
    Display,
}
impl GpuPass {
    pub const ALL: [Self; GPU_PASS_COUNT] = [
        Self::InitializeScene,
        Self::AssignLights,
        Self::Shadows,
        Self::Models,
        Self::Sprites,
        Self::Text,
        Self::SceneResolve,
        Self::Display,
    ];
    pub(crate) fn index(self) -> usize {
        self as usize
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct GpuSpan {
    pub start_ticks: u64,
    pub end_ticks: u64,
    pub nanoseconds: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct GpuFrameTiming {
    /// Encoding sequence; completion can arrive out of order. Gaps are expected.
    pub sequence: u64,
    pub timestamp_period_ns: f32,
    /// Earliest start through latest end of all instrumented pass groups.
    /// Counts overlap/gaps once; does not attribute uninstrumented work or
    /// presentation. Models spans its real motion, opaque, fixed resolve/copy,
    /// overlay-depth and transparent subpasses with one first/last query pair.
    /// SceneResolve is a separate real pass after all world draws, absent at1x.
    pub frame: GpuSpan,
    /// Indexed in `GpuPass::ALL` order. Absent passes are None, never fake zeros.
    pub passes: [Option<GpuSpan>; GPU_PASS_COUNT],
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct GpuTimingStats {
    pub enabled: bool,
    pub pending: usize,
    pub ready: usize,
    pub skipped: u64,
    pub abandoned: u64,
    /// Failed mappings cannot safely reuse wgpu's public mapping state. Slots
    /// retire permanently, without allocating replacement GPU resources.
    pub retired: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GpuTimingError {
    #[error("native pass timestamp queries are unavailable on this adapter")]
    Unsupported,
    #[error("invalid GPU timestamp period")]
    Period,
    #[error("GPU timing state poisoned")]
    State,
    #[error("GPU timing sequence exhausted")]
    Sequence,
    #[error("GPU timestamp readback failed")]
    Readback,
    #[error("GPU timestamps reversed or wrapped; interval discarded")]
    Reversed,
    #[error("GPU timestamp payload has invalid length")]
    Payload,
    #[error("GPU timestamp queries returned only zero counters; measurement unavailable")]
    Unwritten,
}
pub(crate) fn required_features(
    enabled: bool,
    supported: wgpu::Features,
) -> Result<wgpu::Features, GpuTimingError> {
    if !enabled {
        return Ok(wgpu::Features::empty());
    }
    let required = wgpu::Features::TIMESTAMP_QUERY;
    if cfg!(target_arch = "wasm32") || !supported.contains(required) {
        return Err(GpuTimingError::Unsupported);
    }
    Ok(required)
}
fn span(start_ticks: u64, end_ticks: u64, period: f32) -> Result<GpuSpan, GpuTimingError> {
    if !period.is_finite() || period <= 0. {
        return Err(GpuTimingError::Period);
    }
    // Subtract integer ticks before floating conversion to preserve small deltas
    // even when the device's absolute counter is beyond f64 integer precision.
    let delta = end_ticks
        .checked_sub(start_ticks)
        .ok_or(GpuTimingError::Reversed)?;
    Ok(GpuSpan {
        start_ticks,
        end_ticks,
        nanoseconds: delta as f64 * f64::from(period),
    })
}
#[cfg(test)]
mod gpu_tests;
#[cfg(test)]
mod tests;
