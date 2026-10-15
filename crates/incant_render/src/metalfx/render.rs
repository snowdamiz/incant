use super::{MetalFxError, Owner, Pending};
use crate::{Renderer, Result, RetainedTarget};
impl Renderer {
    pub(crate) fn draw_scene_metalfx(
        &self,
        scene: &crate::RenderScene,
        target: RetainedTarget<'_>,
        plan: &crate::resolution::FramePlan,
        motion: crate::motion::MotionDraw,
        owner: &mut Owner,
        pending: &mut Pending,
    ) -> Result<crate::graph::Commands> {
        self.check_device()?;
        crate::output::OutputPass::validate(target.format)?;
        crate::frame::FrameCache::validate(&self.device, target.width, target.height)?;
        if target
            .viewport
            .is_some_and(|v| v.corner_radii.iter().any(|r| !r.is_finite() || *r < 0.))
        {
            return Err(crate::SceneError::Input("invalid viewport corner radius").into());
        }
        // Native temporal reconstruction owns a single-sample input domain;
        // ordinary editor MSAA must never alter its depth or motion contract.
        let attachments = self.frames.get(
            &self.device,
            plan.size[0],
            plan.size[1],
            crate::SceneMsaa::One,
        )?;
        let diagnostics = crate::diagnostic_draw::DiagnosticDraw::prepare(
            self,
            scene,
            attachments.clone(),
            plan.rect,
        )?;
        let models = self.prepare_models(
            scene,
            crate::models::ModelTarget {
                color: attachments.color.clone(),
                depth: attachments.depth.clone(),
                format: crate::frame::HDR_FORMAT,
                rect: plan.rect,
                samples: crate::SceneMsaa::One,
            },
        )?;
        if models.as_ref().is_some_and(|m| m.has_transparent()) {
            return Err(MetalFxError::Unavailable(super::Fallback::Overlays).into());
        }
        let draw = self
            .metalfx
            .as_ref()
            .ok_or(MetalFxError::Disabled)?
            .prepare(
                &self.device,
                pending,
                &attachments.resolved,
                &motion,
                scene.camera,
            )?;
        let native_plan = pending.plan.ok_or(MetalFxError::Interop)?;
        let output = self.output.prepare(
            &self.device,
            &pending
                .targets
                .as_ref()
                .ok_or(MetalFxError::Interop)?
                .normalized,
            target.view,
            target.format,
            target.viewport,
            Some([
                native_plan.output[0] as f32,
                native_plan.output[1] as f32,
                1.,
                1.,
            ]),
        )?;
        let frame = crate::graph::PreparedFrame {
            diagnostics,
            scene_resolve: None,
            models,
            motion: Some(motion),
            temporal: None,
            sprites: None,
            text: None,
            bloom: None,
            output,
        };
        self.graph
            .encode_native(&self.device, frame, &draw, owner, pending)
    }
}
