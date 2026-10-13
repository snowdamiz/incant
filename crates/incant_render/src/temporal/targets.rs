use super::TemporalError;
use std::sync::Arc;
pub const MAX_TEMPORAL_COLOR_BYTES: u64 = 16 * crate::motion::MAX_MOTION_PIXELS;
/// Old+candidate motion/color attachments and maximum motion instance payloads.
/// Excludes existing ordinary renderer targets, allocator/driver/in-flight work.
pub const MAX_TEMPORAL_TRANSITION_BYTES: u64 = 2
    * (MAX_TEMPORAL_COLOR_BYTES
        + crate::motion::MAX_MOTION_TARGET_BYTES
        + crate::motion::MAX_MOTION_INSTANCE_BYTES)
    + 2 * 4096;
pub(super) struct Targets {
    pub size: [u32; 2],
    pub bytes: u64,
    pub color: [wgpu::TextureView; 2],
}
pub(super) fn preflight(
    size: [u32; 2],
    limit: u32,
    old_color: u64,
    old_motion: u64,
) -> Result<u64, TemporalError> {
    let pixels = u64::from(size[0]) * u64::from(size[1]);
    if size.contains(&0)
        || size.iter().any(|&s| s > limit.min(8192))
        || pixels > crate::motion::MAX_MOTION_PIXELS
        || old_color > MAX_TEMPORAL_COLOR_BYTES
        || old_motion > crate::motion::MAX_MOTION_TARGET_BYTES
    {
        return Err(TemporalError::Capacity);
    }
    let color = pixels.checked_mul(16).ok_or(TemporalError::Capacity)?;
    let total = old_color
        .checked_add(old_motion)
        .and_then(|n| n.checked_add(pixels * 40))
        .and_then(|n| n.checked_add(2 * crate::motion::MAX_MOTION_INSTANCE_BYTES + 2 * 4096))
        .ok_or(TemporalError::Capacity)?;
    if total > MAX_TEMPORAL_TRANSITION_BYTES {
        return Err(TemporalError::Capacity);
    }
    Ok(color)
}
impl Targets {
    pub fn prepare(device: &wgpu::Device, size: [u32; 2], old: Option<&Arc<Self>>) -> Arc<Self> {
        // Caller has admitted the combined motion+color transition BEFORE either allocation.
        if let Some(old) = old.filter(|t| t.size == size) {
            return Arc::clone(old);
        }
        Arc::new(Self {
            size,
            bytes: u64::from(size[0]) * u64::from(size[1]) * 16,
            color: std::array::from_fn(|_| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("Opaque temporal color history"),
                        size: wgpu::Extent3d {
                            width: size[0],
                            height: size[1],
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: crate::frame::HDR_FORMAT,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            }),
        })
    }
}

const _: () = assert!(
    size_of::<super::TemporalState>()
        + size_of::<super::Pending>()
        + size_of::<super::TemporalDraw>()
        + 2 * (size_of::<Targets>() + 2 * size_of::<usize>())
        + 128
        < 4096
);
