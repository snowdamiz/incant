//! Reusable, fixed-size offscreen world + screen-UI readback. Sampling a frame
//! does not imply a presented surface or continuity between sampled frames.
use crate::{Renderer, Result, RetainedScene, RetainedTarget, UiLayer};
use incant_render_data::{SceneInput, UiInput};
use std::{io::Write, sync::Arc, time::Duration};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub const MAX_CAPTURE_DIMENSION: u32 = 4096;
pub const MAX_CAPTURE_FRAME_BYTES: u64 = 64 * 1024 * 1024;

/// Checked independently of a device, before any GPU or readback allocation.
pub fn capture_layout(width: u32, height: u32) -> Result<(u32, u64, u64)> {
    if width == 0 || height == 0 || width > MAX_CAPTURE_DIMENSION || height > MAX_CAPTURE_DIMENSION
    {
        return Err("capture dimensions must be within 1..=4096".into());
    }
    let row = width.checked_mul(4).ok_or("capture row overflow")?;
    let stride = row
        .div_ceil(256)
        .checked_mul(256)
        .ok_or("capture stride overflow")?;
    let raw = u64::from(row)
        .checked_mul(u64::from(height))
        .ok_or("capture size overflow")?;
    let padded = u64::from(stride)
        .checked_mul(u64::from(height))
        .ok_or("capture readback overflow")?;
    if padded > MAX_CAPTURE_FRAME_BYTES {
        return Err("capture readback exceeds 64 MiB".into());
    }
    Ok((stride, raw, padded))
}

/// One output texture, one mapped-readback buffer and one reusable RGBA vector.
/// This excludes retained world/UI resources and PNG encoder working memory.
pub struct FrameCapture {
    owner: Arc<()>,
    size: [u32; 2],
    stride: u32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    buffer: wgpu::Buffer,
    rgba: Vec<u8>,
    ready: bool,
}
impl FrameCapture {
    pub fn new(renderer: &Renderer, width: u32, height: u32) -> Result<Self> {
        let (stride, raw, padded) = capture_layout(width, height)?;
        crate::frame::FrameCache::validate(&renderer.device, width, height)?;
        let limits = renderer.device.limits();
        if padded > limits.max_buffer_size {
            return Err("capture readback exceeds device buffer limit".into());
        }
        renderer.check_device()?;
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Selected packaged-player frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reusable selected-frame readback"),
            size: padded,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Ok(Self {
            owner: Arc::clone(&renderer.identity),
            size: [width, height],
            stride,
            texture,
            view,
            buffer,
            rgba: vec![0; usize::try_from(raw)?],
            ready: false,
        })
    }

    /// Draw the exact accepted world input, then its accepted screen UI using
    /// ordinary logical-viewport scaling over the entire fixed physical target.
    /// Poll and callback waits are each bounded to ten seconds; any failure
    /// leaves PNG encoding shut.
    pub fn render(
        &mut self,
        renderer: &Renderer,
        scene: &mut RetainedScene,
        input: &SceneInput,
        ui: Option<(&mut UiLayer, &Arc<UiInput>)>,
    ) -> Result<&[u8]> {
        self.ready = false;
        if !Arc::ptr_eq(&self.owner, &renderer.identity) {
            return Err("capture target belongs to another renderer".into());
        }
        let stage = if let Some((layer, input)) = ui {
            Some((layer.stage(renderer, input)?, layer))
        } else {
            None
        };
        let target = || RetainedTarget {
            view: &self.view,
            format: FORMAT,
            width: self.size[0],
            height: self.size[1],
            viewport: None,
        };
        scene.submit(renderer, input, target())?;
        if let Some((stage, layer)) = stage {
            layer.submit(renderer, stage, &target())?;
        }
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.stride),
                    rows_per_image: Some(self.size[1]),
                },
            },
            wgpu::Extent3d {
                width: self.size[0],
                height: self.size[1],
                depth_or_array_layers: 1,
            },
        );
        let submission = renderer.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        // Unmap also on a timeout/device/map error. A late callback cannot leave
        // a buffer mapped for reuse or publish stale pixels.
        let mapped = (|| -> Result<()> {
            renderer.device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(10)),
            })?;
            rx.recv_timeout(Duration::from_secs(10))??;
            let bytes = self.buffer.slice(..).get_mapped_range();
            for (source, dest) in bytes
                .chunks_exact(self.stride as usize)
                .zip(self.rgba.chunks_exact_mut(self.size[0] as usize * 4))
            {
                dest.copy_from_slice(&source[..dest.len()]);
            }
            Ok(())
        })();
        self.buffer.unmap();
        mapped?;
        renderer.check_device()?;
        self.ready = true;
        Ok(&self.rgba)
    }

    pub fn write_png(&self, writer: impl Write) -> Result<()> {
        if !self.ready {
            return Err("capture pixels are not complete".into());
        }
        let mut encoder = png::Encoder::new(writer, self.size[0], self.size[1]);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&self.rgba)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn checked_layout_includes_padding_and_rejects_unbounded_allocation() {
        assert_eq!(super::capture_layout(1, 2).unwrap(), (256, 8, 512));
        assert_eq!(
            super::capture_layout(4096, 4096).unwrap(),
            (16384, 67108864, 67108864)
        );
        for size in [(0, 1), (1, 0), (4097, 1), (u32::MAX, u32::MAX)] {
            assert!(super::capture_layout(size.0, size.1).is_err());
        }
    }
}
