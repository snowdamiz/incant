//! Candidate uses one ordinary wgpu encoder/submission. No host pixel admission.
use super::*;
impl FrameGraph {
    pub(crate) fn encode_upscale(
        &self,
        device: &wgpu::Device,
        frame: PreparedFrame,
        draw: &crate::temporal_upscale::system::Draw,
        #[cfg(test)] visibility: Option<&crate::motion::visibility::Probe>,
    ) -> Result<wgpu::CommandBuffer> {
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
        let mut encoder = Encoder(
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GPU-only color TAAU"),
            }),
            capture,
        );
        encoder.timed(crate::timing::GpuPass::InitializeScene, |e, q| {
            frame.diagnostics.encode(e, q)
        });
        if let Some(p) = frame
            .models
            .as_ref()
            .and_then(|m| m.lighting.compute.as_ref())
        {
            encoder.timed(crate::timing::GpuPass::AssignLights, |e, q| p.encode(e, q));
        }
        if let Some(m) = &frame.models
            && m.shadows.has_passes()
        {
            encoder.timed(crate::timing::GpuPass::Shadows, |e, q| {
                m.shadows.encode(e, q)
            });
        }
        let opaque = frame.models.as_ref().is_some_and(|m| m.has_opaque());
        encoder.timed(crate::timing::GpuPass::Models, |e, q| {
            let count = 1 + usize::from(opaque);
            frame
                .motion
                .as_ref()
                .expect("TAAU motion")
                .encode(e, q.and_then(|q| q.render_range(0, count)));
            if opaque {
                frame
                    .models
                    .as_ref()
                    .unwrap()
                    .encode_opaque(e, q.and_then(|q| q.render_range(1, count)));
            }
        });
        #[cfg(test)]
        if draw.visibility_before {
            visibility.expect("reconstruction visibility").encode(
                device,
                &mut encoder.0,
                frame.motion.as_ref().expect("TAAU motion"),
            );
        }
        encoder.timed(crate::timing::GpuPass::SceneResolve, |e, q| {
            draw.encode(e, q.map(|q| q.render()))
        });
        encoder.timed(crate::timing::GpuPass::Display, |e, q| {
            frame.output.encode(e, q.map(|q| q.render()))
        });
        #[cfg(test)]
        if let Some(probe) = visibility.filter(|_| !draw.visibility_before) {
            probe.encode(
                device,
                &mut encoder.0,
                frame.motion.as_ref().expect("TAAU motion"),
            );
        }
        if let Some(c) = encoder.1.take() {
            c.finish(&mut encoder.0);
        }
        Ok(encoder.0.finish())
    }
}
