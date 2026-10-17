use crate::{Renderer, Result, RetainedTarget};
impl Renderer {
    pub(crate) fn draw_scene_upscale(
        &self,
        scene: &crate::RenderScene,
        target: RetainedTarget<'_>,
        plan: &crate::resolution::FramePlan,
        motion: crate::motion::MotionDraw,
        pending: &super::Pending,
    ) -> Result<wgpu::CommandBuffer> {
        self.check_device()?;
        crate::output::OutputPass::validate(target.format)?;
        let system = self
            .temporal_upscale
            .as_ref()
            .ok_or(crate::temporal::TemporalError::Disabled)?;
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
        let mut models = self.prepare_models(
            scene,
            crate::models::ModelTarget {
                color: attachments.color.clone(),
                depth: attachments.depth.clone(),
                format: crate::frame::HDR_FORMAT,
                rect: plan.rect,
                samples: crate::SceneMsaa::One,
            },
        )?;
        if let Some(models) = &mut models {
            models.center_materials(system)?;
        }
        let draw = system.prepare(&self.device, pending, &attachments.resolved, scene.camera);
        let output = self.output.prepare(
            &self.device,
            &draw.color,
            target.view,
            target.format,
            None,
            Some([target.width as f32, target.height as f32, 1., 1.]),
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
        self.graph.encode_upscale(&self.device, frame, &draw)
    }
}
pub(crate) fn admission(
    renderer: &Renderer,
    scene: &crate::RenderScene,
    plan: &crate::resolution::FramePlan,
    target: &RetainedTarget<'_>,
) -> Result<()> {
    if renderer.temporal_upscale.is_none() {
        return Err(crate::temporal::TemporalError::Disabled.into());
    }
    if target.viewport.is_some()
        || scene.camera.pixel_resolution.is_some()
        || scene.sprites.is_some()
        || scene.text.is_some()
        || renderer.bloom.is_some()
        || renderer.ambient_occlusion.is_some()
        || scene.has_blended_models()
        || plan.rect != [0., 0., plan.size[0] as f32, plan.size[1] as f32]
    {
        return Err(crate::SceneError::Input("experimental TAAU supports full-viewport opaque/MASK models without overlays, SSAO or bloom").into());
    }
    Ok(())
}
