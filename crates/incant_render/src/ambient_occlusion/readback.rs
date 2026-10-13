//! Optional numerical observer. No shader, project or scene mutation.
#[cfg(not(target_arch = "wasm32"))]
use super::math::{MAX_PIXELS, target_bytes};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use wgpu::util::DeviceExt;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AmbientOcclusionCaptureError {
    #[error("SSAO must be enabled before arming a capture")]
    Disabled,
    #[error("SSAO stage capture is already armed, ready or being read")]
    Busy,
    #[error("no SSAO stage capture is ready")]
    NotReady,
    #[error("the captured command has not completed submission")]
    NotSubmitted,
    #[error("SSAO capture dimensions or allocation exceed diagnostic limits")]
    Capacity,
    #[error("SSAO capture lock failed")]
    Lock,
    #[error("SSAO stage buffer mapping or device polling failed")]
    Mapping,
    #[error("blocking diagnostic stage readback is native-only")]
    NativeOnly,
}

/// Tight, top-left-origin R8 planes in internal render-target coordinates.
pub struct AmbientOcclusionStages {
    pub size: [u32; 2],
    pub raw: Vec<u8>,
    pub horizontal: Vec<u8>,
    pub vertical: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
struct Layout {
    size: [u32; 2],
    row: u32,
    plane: u64,
    bytes: u64,
}
#[cfg(not(target_arch = "wasm32"))]
impl Layout {
    fn new(size: [u32; 2], max_buffer: u64) -> Result<Self, AmbientOcclusionCaptureError> {
        target_bytes(size).map_err(|_| AmbientOcclusionCaptureError::Capacity)?;
        let row = size[0].div_ceil(256) * 256;
        let plane = u64::from(row) * u64::from(size[1]);
        let bytes = 256 + plane * 3;
        // Includes worst-case row padding; independent of the idle texture cache.
        if bytes > max_buffer || bytes > 256 + 3 * (MAX_PIXELS + 255 * 8192) {
            return Err(AmbientOcclusionCaptureError::Capacity);
        }
        Ok(Self {
            size,
            row,
            plane,
            bytes,
        })
    }
    fn unpack(&self, data: &[u8]) -> Result<AmbientOcclusionStages, AmbientOcclusionCaptureError> {
        if data.len() != self.bytes as usize || data[..256].iter().any(|b| *b != 0) {
            return Err(AmbientOcclusionCaptureError::NotSubmitted);
        }
        let mut planes: [Vec<u8>; 3] = Default::default();
        let [w, h] = self.size;
        for (stage, plane) in planes.iter_mut().enumerate() {
            plane
                .try_reserve_exact((w * h) as usize)
                .map_err(|_| AmbientOcclusionCaptureError::Capacity)?;
            for y in 0..h {
                let start = 256 + self.plane as usize * stage + (self.row * y) as usize;
                plane.extend_from_slice(&data[start..start + w as usize]);
            }
        }
        let [raw, horizontal, vertical] = planes;
        Ok(AmbientOcclusionStages {
            size: self.size,
            raw,
            horizontal,
            vertical,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
enum State<T> {
    #[default]
    Idle,
    Armed,
    Ready(T),
    Failed(AmbientOcclusionCaptureError),
    Reading,
}
#[cfg(not(target_arch = "wasm32"))]
impl<T> State<T> {
    fn arm(&mut self) -> Result<(), AmbientOcclusionCaptureError> {
        if !matches!(self, Self::Idle) {
            return Err(AmbientOcclusionCaptureError::Busy);
        }
        *self = Self::Armed;
        Ok(())
    }
    fn take(&mut self) -> Result<T, AmbientOcclusionCaptureError> {
        match std::mem::replace(self, Self::Reading) {
            Self::Ready(value) => Ok(value),
            Self::Failed(error) => {
                *self = Self::Idle;
                Err(error)
            }
            prior => {
                *self = prior;
                Err(AmbientOcclusionCaptureError::NotReady)
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
pub(super) struct Capture(Mutex<State<Frame>>);
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub(super) struct Frame {
    layout: Layout,
    buffer: wgpu::Buffer,
}
#[cfg(not(target_arch = "wasm32"))]
impl Capture {
    pub fn begin(&self, device: &wgpu::Device, size: [u32; 2]) -> Option<Frame> {
        let mut state = self.0.lock().ok()?;
        if !matches!(*state, State::Armed) {
            return None;
        }
        match Frame::new(device, size) {
            Ok(frame) => {
                *state = State::Ready(frame.clone());
                Some(frame)
            }
            Err(error) => {
                *state = State::Failed(error);
                None
            }
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl Frame {
    fn new(device: &wgpu::Device, size: [u32; 2]) -> Result<Self, AmbientOcclusionCaptureError> {
        let layout = Layout::new(size, device.limits().max_buffer_size)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(layout.bytes as usize)
            .map_err(|_| AmbientOcclusionCaptureError::Capacity)?;
        bytes.resize(layout.bytes as usize, 0);
        // Only a submitted final-stage copy clears this marker. Mapping an
        // encoded but discarded command must not return synthetic zero planes.
        bytes[..256].fill(0xA5);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("diagnostic SSAO stages and completion marker"),
            contents: &bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        });
        Ok(Self { layout, buffer })
    }
    pub fn copy(&self, encoder: &mut wgpu::CommandEncoder, source: &wgpu::Texture, stage: usize) {
        debug_assert!(stage < 3);
        encoder.copy_texture_to_buffer(
            source.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 256 + self.layout.plane * stage as u64,
                    bytes_per_row: Some(self.layout.row),
                    rows_per_image: Some(self.layout.size[1]),
                },
            },
            source.size(),
        );
        if stage == 2 {
            encoder.clear_buffer(&self.buffer, 0, Some(256));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn read(
        &self,
        device: &wgpu::Device,
    ) -> Result<AmbientOcclusionStages, AmbientOcclusionCaptureError> {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
        let mapped = device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .map_err(|_| AmbientOcclusionCaptureError::Mapping)
            .and_then(|_| {
                rx.recv_timeout(std::time::Duration::from_secs(10))
                    .map_err(|_| AmbientOcclusionCaptureError::Mapping)
            })
            .and_then(|r| r.map_err(|_| AmbientOcclusionCaptureError::Mapping));
        if let Err(error) = mapped {
            self.buffer.unmap();
            return Err(error);
        }
        let data = self.buffer.slice(..).get_mapped_range();
        let result = self.layout.unpack(&data);
        drop(data);
        self.buffer.unmap();
        result
    }
}

impl crate::Renderer {
    /// Arm exactly the next encoded SSAO execution. A model-free or failed draw
    /// leaves the request armed. A second arm refuses until the first is taken.
    /// Disabling or replacing SSAO abandons the old system's capture.
    pub fn arm_ambient_occlusion_stage_capture(&self) -> crate::Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            Err(AmbientOcclusionCaptureError::NativeOnly.into())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let system = self
                .ambient_occlusion
                .as_ref()
                .ok_or(AmbientOcclusionCaptureError::Disabled)?;
            system
                .capture
                .0
                .lock()
                .map_err(|_| AmbientOcclusionCaptureError::Lock)?
                .arm()?;
            Ok(())
        }
    }
    /// Take tight planes after submitting the captured command. This native-only
    /// diagnostic blocks for at most the mapping deadlines. An unsubmitted or
    /// discarded command refuses; take consumes a ready capture even on failure.
    pub fn take_ambient_occlusion_stages(&self) -> crate::Result<AmbientOcclusionStages> {
        #[cfg(target_arch = "wasm32")]
        {
            Err(AmbientOcclusionCaptureError::NativeOnly.into())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.check_device()?;
            let capture = &self
                .ambient_occlusion
                .as_ref()
                .ok_or(AmbientOcclusionCaptureError::Disabled)?
                .capture;
            let frame = capture
                .0
                .lock()
                .map_err(|_| AmbientOcclusionCaptureError::Lock)?
                .take()?;
            let result = frame.read(&self.device);
            *capture
                .0
                .lock()
                .map_err(|_| AmbientOcclusionCaptureError::Lock)? = State::Idle;
            self.check_device()?;
            Ok(result?)
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn capture_state_is_one_shot_and_failed_or_unencoded_attempts_are_explicit() {
        let mut state = State::<u32>::default();
        assert_eq!(state.take(), Err(AmbientOcclusionCaptureError::NotReady));
        state.arm().unwrap();
        assert_eq!(state.arm(), Err(AmbientOcclusionCaptureError::Busy));
        assert_eq!(state.take(), Err(AmbientOcclusionCaptureError::NotReady));
        assert!(matches!(state, State::Armed));
        state = State::Ready(42);
        assert_eq!(state.take(), Ok(42));
        assert_eq!(state.arm(), Err(AmbientOcclusionCaptureError::Busy));
        state = State::Failed(AmbientOcclusionCaptureError::Capacity);
        assert_eq!(state.take(), Err(AmbientOcclusionCaptureError::Capacity));
        state.arm().unwrap();
    }
    #[test]
    fn stage_rows_are_aligned_and_diagnostic_allocation_is_bounded_before_use() {
        let l = Layout::new([321, 193], u64::MAX).unwrap();
        assert_eq!(
            (l.row, l.plane, l.bytes),
            (512, 512 * 193, 256 + 3 * 512 * 193)
        );
        for size in [[0, 1], [1, 0], [8193, 1], [8192, 8192]] {
            assert!(matches!(
                Layout::new(size, u64::MAX),
                Err(AmbientOcclusionCaptureError::Capacity)
            ));
        }
        assert!(Layout::new([321, 193], l.bytes - 1).is_err());
        assert!(Layout::new([8192, 512], u64::MAX).is_ok());
    }
    #[test]
    fn unpack_requires_gpu_completion_and_strips_every_stage_row_padding() {
        let layout = Layout::new([3, 2], u64::MAX).unwrap();
        let mut bytes = vec![0xA5; layout.bytes as usize];
        assert!(matches!(
            layout.unpack(&bytes),
            Err(AmbientOcclusionCaptureError::NotSubmitted)
        ));
        bytes[..256].fill(0);
        for stage in 0..3 {
            for y in 0..2 {
                let start = 256 + stage * layout.plane as usize + y * layout.row as usize;
                bytes[start..start + 3].fill((stage * 10 + y) as u8);
            }
        }
        let result = layout.unpack(&bytes).unwrap();
        assert_eq!(result.size, [3, 2]);
        assert_eq!(result.raw, [0, 0, 0, 1, 1, 1]);
        assert_eq!(result.horizontal, [10, 10, 10, 11, 11, 11]);
        assert_eq!(result.vertical, [20, 20, 20, 21, 21, 21]);
        assert!(matches!(
            layout.unpack(&bytes[..bytes.len() - 1]),
            Err(AmbientOcclusionCaptureError::NotSubmitted)
        ));
        bytes[255] = 1;
        assert!(matches!(
            layout.unpack(&bytes),
            Err(AmbientOcclusionCaptureError::NotSubmitted)
        ));
    }
}
