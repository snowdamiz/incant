//! World attachment selection and complete-frame preparation before encoding.
use crate::{
    RenderScene, Renderer, Result, RetainedTarget, SceneMsaa, Viewport, diagnostic_draw, frame,
    graph, models, motion, output, resolution, temporal,
};
impl Renderer {
    pub fn draw_scene(
        &self,
        scene: &RenderScene,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        viewport: Option<Viewport>,
    ) -> Result<wgpu::CommandBuffer> {
        self.draw_scene_sampled(
            scene,
            RetainedTarget {
                view,
                format,
                width,
                height,
                viewport,
            },
            resolution::ResolutionScale::FULL,
            None,
            None,
            self.scene_msaa
                .select(scene.camera.pixel_resolution.is_some()),
        )
    }
    pub(crate) fn draw_scene_scaled(
        &self,
        scene: &RenderScene,
        target: RetainedTarget<'_>,
        scale: resolution::ResolutionScale,
        motion: Option<motion::MotionDraw>,
        temporal: Option<&temporal::Pending>,
    ) -> Result<wgpu::CommandBuffer> {
        // Native retained, dynamic resolution, temporal and vendor-upscale
        // routes never inherit the ordinary editor's world-MSAA selection.
        self.draw_scene_sampled(scene, target, scale, motion, temporal, SceneMsaa::One)
    }
    fn draw_scene_sampled(
        &self,
        scene: &RenderScene,
        target: RetainedTarget<'_>,
        scale: resolution::ResolutionScale,
        mut motion: Option<motion::MotionDraw>,
        temporal: Option<&temporal::Pending>,
        samples: SceneMsaa,
    ) -> Result<wgpu::CommandBuffer> {
        let RetainedTarget {
            view,
            format,
            width,
            height,
            viewport,
        } = target;
        samples.validate_ao(self.ambient_occlusion.is_some())?;
        self.check_device()?;
        output::OutputPass::validate(format)?;
        frame::FrameCache::validate(&self.device, width, height)?;
        if viewport.is_some_and(|v| v.corner_radii.iter().any(|r| !r.is_finite() || *r < 0.)) {
            return Err("invalid viewport corner radius".into());
        }
        let plan = resolution::FramePlan::new(
            [width, height],
            viewport
                .map(|v| v.rect)
                .unwrap_or([0., 0., width as f32, height as f32]),
            scene.camera.pixel_resolution,
            scale,
        )?;
        let frame_size = plan.size;
        let scene_rect = plan.rect;
        let attachments = self
            .frames
            .get(&self.device, frame_size[0], frame_size[1], samples)?;
        let diagnostics =
            diagnostic_draw::DiagnosticDraw::prepare(self, scene, attachments.clone(), scene_rect)?;
        let models = self.prepare_models(
            scene,
            models::ModelTarget {
                color: attachments.color.clone(),
                depth: attachments.depth.clone(),
                format: frame::HDR_FORMAT,
                rect: scene_rect,
                samples,
            },
        )?;
        let sprites = self.sprites.prepare(
            &self.device,
            scene,
            models::ModelTarget {
                color: attachments.color.clone(),
                depth: attachments.depth.clone(),
                format: frame::HDR_FORMAT,
                rect: scene_rect,
                samples,
            },
        )?;
        let text = self.text.prepare(
            &self.device,
            scene,
            models::ModelTarget {
                color: attachments.color.clone(),
                depth: attachments.depth.clone(),
                format: frame::HDR_FORMAT,
                rect: scene_rect,
                samples,
            },
        )?;
        let overlays = models.as_ref().is_some_and(|m| m.has_transparent())
            || sprites.as_ref().is_some_and(|s| s.has_passes())
            || text.as_ref().is_some_and(|t| t.has_passes());
        if temporal.is_some_and(|p| p.active()) && overlays {
            motion
                .as_mut()
                .expect("temporal motion draw")
                .set_overlay_depth(attachments.depth.clone());
        }
        let temporal = temporal.and_then(|pending| {
            self.temporal.as_ref().and_then(|system| {
                system.prepare(&self.device, pending, &attachments.resolved, scene.camera)
            })
        });
        let bloom_system = self
            .bloom
            .as_ref()
            .filter(|b| scene.camera.pixel_resolution.is_none() && b.config.intensity > 0.);
        let bloom = bloom_system
            .map(|system| {
                system.prepare(
                    &self.device,
                    &attachments.resolved,
                    [width, height],
                    &plan,
                    scale,
                )
            })
            .transpose()?;
        let output = if let Some((draw, system)) = bloom.as_ref().zip(bloom_system) {
            system.output.prepare_optional_bloom(
                &self.device,
                &attachments.resolved,
                view,
                format,
                viewport,
                plan.output_mapping,
                Some((&draw.result, system.config.intensity, [width, height])),
            )?
        } else {
            self.output.prepare(
                &self.device,
                &attachments.resolved,
                view,
                format,
                viewport,
                plan.output_mapping,
            )?
        };
        self.graph.encode(
            &self.device,
            graph::PreparedFrame {
                diagnostics,
                scene_resolve: frame::SceneResolve::new(attachments),
                models,
                motion,
                temporal,
                sprites,
                text,
                bloom,
                output,
            },
        )
    }
}
