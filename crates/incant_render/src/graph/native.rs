//! Native-only batch; normal graph API and encoder remain unchanged. This code
//! uses the same prepared draw packets and seven logical timestamp groups.
use super::*;

pub(crate) enum Commands {
    Single(wgpu::CommandBuffer),
    Native([wgpu::CommandBuffer; 3]),
}
impl IntoIterator for Commands {
    type Item = wgpu::CommandBuffer;
    type IntoIter = std::iter::Flatten<std::array::IntoIter<Option<wgpu::CommandBuffer>, 3>>;
    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Single(c) => [Some(c), None, None],
            Self::Native(c) => c.map(Some),
        }
        .into_iter()
        .flatten()
    }
}
impl FrameGraph {
    pub fn encode_native(
        &self,
        device: &wgpu::Device,
        frame: PreparedFrame,
        native: &crate::metalfx::Draw,
        encode_raw: impl FnOnce() -> Result<wgpu::CommandBuffer>,
        retain: impl FnOnce(&mut wgpu::CommandEncoder),
    ) -> Result<Commands> {
        let state = self
            .0
            .lock()
            .map_err(|_| ResourceError::CacheLock("frame graph"))?;
        let capture = state
            .timings
            .as_ref()
            .map(|p| p.begin())
            .transpose()?
            .flatten();
        let make = || {
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Native interop ordered frame segment"),
            })
        };
        let mut pre = Encoder(make(), capture);
        pre.timed(crate::timing::GpuPass::InitializeScene, |e, q| {
            frame.diagnostics.encode(e, q)
        });
        if let Some(p) = frame
            .models
            .as_ref()
            .and_then(|m| m.lighting.compute.as_ref())
        {
            pre.timed(crate::timing::GpuPass::AssignLights, |e, q| p.encode(e, q));
        }
        if let Some(m) = &frame.models
            && m.shadows.has_passes()
        {
            pre.timed(crate::timing::GpuPass::Shadows, |e, q| {
                m.shadows.encode(e, q)
            });
        }
        let up = frame
            .upscaling
            .as_ref()
            .ok_or(crate::metalfx::MetalFxError::Interop)?;
        let count = usize::from(frame.motion.is_some())
            + usize::from(frame.models.as_ref().is_some_and(|m| m.has_opaque()))
            + 5
            + usize::from(frame.models.as_ref().is_some_and(|m| m.has_transparent()));
        let queries = pre
            .1
            .as_mut()
            .map(|c| c.pass(crate::timing::GpuPass::Models));
        let mut index = 0;
        let mut timestamp = || {
            let r = queries.as_ref().and_then(|q| q.render_range(index, count));
            index += 1;
            r
        };
        if let Some(m) = &frame.motion {
            m.encode(&mut pre.0, timestamp());
        }
        if let Some(m) = &frame.models
            && m.has_opaque()
        {
            m.encode_opaque(&mut pre.0, timestamp());
        }
        native.pack(&mut pre.0, timestamp());
        native.clear(&mut pre.0, 4, timestamp());
        native.clear(&mut pre.0, 5, timestamp());
        native.transition_before(&mut pre.0);
        let Encoder(pre, capture) = pre;
        let pre = pre.finish();
        // Raw encoder is never used with ordinary wgpu encoding. Failure here
        // drops every prepared buffer/capture; caller publishes no scene state.
        let raw = encode_raw()?;
        let mut post = Encoder(make(), capture);
        native.normalize(&mut post.0, timestamp());
        up.clear_overlay(&mut post.0, timestamp());
        if let Some(m) = &frame.models
            && m.has_transparent()
        {
            m.encode_transparent(&mut post.0, timestamp());
        }
        debug_assert_eq!(index, count);
        if let Some(s) = &frame.sprites {
            post.timed(crate::timing::GpuPass::Sprites, |e, q| s.encode(e, q));
        }
        if let Some(t) = &frame.text
            && t.has_passes()
        {
            post.timed(crate::timing::GpuPass::Text, |e, q| t.encode(e, q));
        }
        post.timed(crate::timing::GpuPass::Display, |e, q| {
            up.composite(e, q.and_then(|q| q.render_range(0, 2)));
            frame
                .output
                .encode_part(e, q.and_then(|q| q.render_range(1, 2)));
        });
        retain(&mut post.0);
        if let Some(c) = post.1.take() {
            c.finish(&mut post.0);
        }
        Ok(Commands::Native([pre, raw, post.0.finish()]))
    }
}
