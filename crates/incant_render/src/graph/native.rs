//! Raw Metal encoding has its own wgpu command encoder. The queue still receives
//! one ordered batch; no external queue, commit, wait or CPU texture copy.
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
    pub(crate) fn encode_native(
        &self,
        device: &wgpu::Device,
        frame: PreparedFrame,
        native: &crate::metalfx::Draw,
        owner: &mut crate::metalfx::Owner,
        pending: &mut crate::metalfx::Pending,
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
                label: Some("Native interop ordered segment"),
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
        let ao = frame
            .models
            .as_ref()
            .is_some_and(|m| m.has_ambient_occlusion());
        let opaque = frame.models.as_ref().is_some_and(|m| m.has_opaque());
        // Motion + optional AO/opaque + pack + output/exposure clears + normalize.
        let count = 5 + 4 * usize::from(ao) + usize::from(opaque);
        let queries = pre
            .1
            .as_mut()
            .map(|c| c.pass(crate::timing::GpuPass::Models));
        let mut index = 0;
        let mut timestamp = || {
            let q = queries.as_ref().and_then(|q| q.render_range(index, count));
            index += 1;
            q
        };
        frame
            .motion
            .as_ref()
            .ok_or(crate::metalfx::MetalFxError::Interop)?
            .encode(&mut pre.0, timestamp());
        if ao {
            frame.models.as_ref().unwrap().encode_ambient_occlusion(
                &mut pre.0,
                [timestamp(), timestamp(), timestamp(), timestamp()],
            );
        }
        if opaque {
            frame
                .models
                .as_ref()
                .unwrap()
                .encode_opaque(&mut pre.0, timestamp());
        }
        native.pack(&mut pre.0, timestamp());
        native.clear(&mut pre.0, 4, timestamp());
        native.clear(&mut pre.0, 5, timestamp());
        native.transition_before(&mut pre.0);
        let Encoder(pre, capture) = pre;
        let pre = pre.finish();
        let raw = owner.encode(device, pending)?;
        let mut post = Encoder(make(), capture);
        native.normalize(&mut post.0, timestamp());
        debug_assert_eq!(index, count);
        post.timed(crate::timing::GpuPass::Display, |e, q| {
            frame.output.encode(e, q.map(|q| q.render()))
        });
        pending.completion(&mut post.0);
        if let Some(c) = post.1.take() {
            c.finish(&mut post.0);
        }
        Ok(Commands::Native([pre, raw, post.0.finish()]))
    }
}
