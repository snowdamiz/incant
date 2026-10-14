//! Native moving-frame path. Topology/resources are prepared once; updates touch
//! only preallocated CPU staging and retained GPU instance/light buffers.
use super::{RenderScene, Renderer};
use crate::{Result, SceneError, SceneStats, scene::Instance};
use incant_assets_runtime::AssetVersionId;
use incant_render_data::{SceneInput, SceneLight};
use std::sync::Arc;

/// Output owned by the host for one immediately submitted native frame.
pub struct RetainedTarget<'a> {
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
    pub viewport: Option<crate::Viewport>,
}
struct NodeBinding {
    id: String,
    camera: Option<incant_types::Camera>,
    light: Option<SceneLight>,
    mesh: Option<(AssetVersionId, bool)>,
    sprite: Option<(AssetVersionId, incant_2d::SpriteRenderer)>,
    tilemap: Option<Arc<incant_render_data::TilemapInput>>,
    frames: Option<Arc<incant_render_data::SpriteFrames>>,
}
/// Private double staging prevents callers from keeping an unsubmitted command
/// buffer or a scene snapshot that aliases mutable instance/light resources.
/// Binding changes require preparing a replacement and accepting its first frame.
/// This path accepts native camera/model/sprite/text/light/environment inputs.
/// Frame encoding still allocates bounded draw packets/uniform resources; this is
/// not a claim that the complete renderer has reached its shipping budgets.
pub struct RetainedScene {
    owner: Arc<()>,
    motion: Option<super::motion::MotionState>,
    temporal: Option<crate::temporal::TemporalState>,
    #[cfg(feature = "native-metalfx")]
    metalfx: Option<crate::metalfx::Owner>,
    resolution: Option<crate::resolution::ResolutionOwner>,
    bindings: Vec<NodeBinding>,
    environment: Option<(AssetVersionId, f64, f64)>,
    camera_index: usize,
    live: RenderScene,
    staging: RenderScene,
    light_plan: crate::lighting::scene::LightPlan,
    packed_lights: Vec<crate::lighting::scene::Light>,
}
fn environment(input: &SceneInput) -> Option<(AssetVersionId, f64, f64)> {
    input
        .environment
        .as_ref()
        .map(|e| (e.source.version_id(), e.intensity, e.rotation_degrees))
}
impl Renderer {
    pub fn prepare_retained(&self, input: &SceneInput, camera: &str) -> Result<RetainedScene> {
        self.check_device()?;
        if input.nodes.iter().any(|n| n.diagnostic) {
            return Err(
                SceneError::Input("retained native diagnostic cubes are not implemented").into(),
            );
        }
        let camera_index = input
            .nodes
            .iter()
            .position(|n| n.id == camera && n.camera.is_some())
            .ok_or_else(|| SceneError::MissingCamera(camera.into()))?;
        // Immutable geometry/materials are shared; mutable frame buffers are not.
        let live = self
            .upload_scene(crate::scene::resolve(input)?, true)?
            .with_camera(camera)?;
        let mut staging_plan = crate::scene::resolve(input)?;
        staging_plan.sprites.clear();
        staging_plan.tilemaps.clear();
        staging_plan.text = None;
        let mut staging = self.upload_scene(staging_plan, true)?.with_camera(camera)?;
        staging.sprites = live
            .sprites
            .as_ref()
            .map(|sprites| Arc::new(sprites.staging(&self.device)));
        staging.text = live
            .text
            .as_ref()
            .map(|text| Arc::new(text.staging(&self.text, &self.device)));
        // Both frames report the same shared text resources after any swap.
        staging.stats.text_gpu_bytes = live.stats.text_gpu_bytes;
        let directional = input
            .nodes
            .iter()
            .filter(|n| matches!(n.light, Some(SceneLight::Directional(_))))
            .count();
        let local = input
            .nodes
            .iter()
            .filter(|n| matches!(n.light, Some(SceneLight::Point(_) | SceneLight::Spot(_))))
            .count();
        Ok(RetainedScene {
            owner: Arc::clone(&self.identity),
            motion: None,
            temporal: None,
            #[cfg(feature = "native-metalfx")]
            metalfx: None,
            resolution: None,
            bindings: input
                .nodes
                .iter()
                .map(|n| NodeBinding {
                    id: n.id.clone(),
                    camera: n.camera.clone(),
                    light: n.light.clone(),
                    sprite: n
                        .sprite
                        .as_ref()
                        .map(|s| (s.source.version_id(), s.sprite.clone())),
                    tilemap: n.tilemap.clone(),
                    frames: n.sprite.as_ref().and_then(|s| s.frames.clone()),
                    mesh: n
                        .mesh
                        .as_ref()
                        .map(|m| (m.source.version_id(), m.cast_shadows)),
                })
                .collect(),
            environment: environment(input),
            camera_index,
            live,
            staging,
            light_plan: crate::lighting::scene::LightPlan::with_capacity(
                directional,
                local,
                crate::shadows::MAX_SHADOW_LIGHTS,
            ),
            packed_lights: Vec::with_capacity((directional + local).max(1)),
        })
    }
}
impl RetainedScene {
    /// Explicit startup opt-in. Spatial fallback keeps the ordinary renderer;
    /// screen-space host UI remains after the world. No built-in upscale claim.
    #[cfg(feature = "native-metalfx")]
    pub fn enable_metalfx(
        &mut self,
        renderer: &Renderer,
        preference: crate::metalfx::Preference,
        config: crate::metalfx::Config,
    ) -> Result<()> {
        let owner = crate::metalfx::Owner::new(
            preference,
            config,
            renderer
                .metalfx_admission
                .ok_or(crate::metalfx::MetalFxError::Disabled)?,
        )?;
        if !Arc::ptr_eq(&self.owner, &renderer.identity) {
            return Err(crate::motion::MotionError::Owner.into());
        }
        if renderer.metalfx_admission.is_some_and(|r| r.is_ok()) {
            self.enable_motion_history(renderer)?;
        } else {
            self.disable_motion_history();
        }
        self.metalfx = Some(owner);
        Ok(())
    }
    /// Prepare a future output extent outside ordinary frame submission. This
    /// may synchronously initialize the vendor scaler; call at startup/resize.
    #[cfg(feature = "native-metalfx")]
    pub fn prepare_metalfx_extent(&mut self, renderer: &Renderer, extent: [u32; 2]) -> Result<()> {
        if !Arc::ptr_eq(&self.owner, &renderer.identity) {
            return Err(crate::motion::MotionError::Owner.into());
        }
        self.metalfx
            .as_mut()
            .ok_or(crate::metalfx::MetalFxError::Disabled)?
            .prepare_extent(renderer, extent)?;
        Ok(())
    }
    #[cfg(feature = "native-metalfx")]
    pub fn metalfx_selection(&self) -> Option<crate::metalfx::Selection> {
        self.metalfx.as_ref().and_then(|o| o.selection())
    }

    /// Explicit owner attachment reset. Toggling creates a fresh motion owner
    /// and drops accepted fixed color history atomically. No color consumer
    /// reads the attachment yet; upscale/native history owners are not present.
    pub fn set_motion_owner_attachment(
        &mut self,
        renderer: &Renderer,
        enabled: bool,
    ) -> Result<()> {
        let old = self
            .motion
            .as_ref()
            .ok_or(crate::motion::MotionError::Disabled)?;
        if !Arc::ptr_eq(&self.owner, &renderer.identity) {
            return Err(crate::motion::MotionError::Owner.into());
        }
        if enabled && !renderer.motion.as_ref().is_some_and(|m| m.owner_enabled()) {
            return Err(crate::motion::MotionError::OwnerDisabled.into());
        }
        if old.owner_attachment == enabled {
            return Ok(());
        }
        self.enable_motion_history(renderer)?;
        self.motion
            .as_mut()
            .expect("enabled history")
            .owner_attachment = enabled;
        Ok(())
    }
    pub fn motion_owner_attachment_enabled(&self) -> bool {
        self.motion.as_ref().is_some_and(|m| m.owner_attachment)
    }
    /// Pipelines must already be enabled on this renderer. Publication is atomic.
    pub fn enable_motion_history(&mut self, renderer: &Renderer) -> Result<()> {
        if !Arc::ptr_eq(&self.owner, &renderer.identity) {
            return Err(crate::motion::MotionError::Owner.into());
        }
        let candidate = crate::motion::MotionState::new(
            renderer,
            &self.live,
            &self.staging,
            self.motion.as_ref(),
        )?;
        self.motion = Some(candidate);
        self.temporal = None;
        #[cfg(feature = "native-metalfx")]
        {
            self.metalfx = None;
        }
        Ok(())
    }
    pub fn disable_motion_history(&mut self) {
        self.motion = None;
        self.temporal = None;
        #[cfg(feature = "native-metalfx")]
        {
            self.metalfx = None;
        }
    }
    pub fn invalidate_motion_history(&mut self) {
        if let Some(motion) = &mut self.motion {
            motion.invalidate();
        }
    }
    /// Borrow accepted placement changes. Candidate staging and failed frames
    /// never publish here; IDs are scoped to the returned frame owner/epoch.
    pub fn motion_changes(&self) -> Option<crate::motion::MotionChanges<'_>> {
        self.motion.as_ref().and_then(|m| m.changes())
    }
    pub fn motion_frame(&self) -> Option<crate::motion::MotionFrame> {
        self.motion.as_ref().and_then(|m| m.frame())
    }

    /// Atomic opt-in/reset. Color, depth, exact model history and jitter are one
    /// submitted owner. No authored camera or world state is changed.
    pub fn enable_temporal_history(
        &mut self,
        renderer: &Renderer,
        config: crate::temporal::TemporalConfig,
    ) -> Result<()> {
        if self.motion_owner_attachment_enabled() {
            return Err(crate::motion::MotionError::OwnerCombination.into());
        }
        let temporal = crate::temporal::TemporalState::new(config)?;
        if renderer.temporal.is_none() {
            return Err(crate::temporal::TemporalError::Disabled.into());
        }
        self.enable_motion_history(renderer)?;
        self.temporal = Some(temporal);
        Ok(())
    }
    pub fn temporal_frame(&self) -> Option<crate::temporal::TemporalFrame> {
        self.temporal.as_ref().and_then(|t| t.frame())
    }
    /// Explicit opt-in, replacing any previous telemetry ownership. Invalid config
    /// leaves the existing policy unchanged. None restores output-sized rendering.
    pub fn set_dynamic_resolution(
        &mut self,
        config: Option<crate::resolution::ResolutionConfig>,
    ) -> std::result::Result<(), crate::resolution::ResolutionError> {
        self.resolution = config
            .map(crate::resolution::ResolutionOwner::new)
            .transpose()?;
        Ok(())
    }
    pub fn resolution_frame(&self) -> Option<crate::resolution::ResolutionFrame> {
        self.resolution.as_ref().and_then(|r| r.last())
    }
    pub fn record_frame_time(
        &mut self,
        frame: &crate::resolution::ResolutionFrame,
        milliseconds: f64,
    ) -> std::result::Result<bool, crate::resolution::ResolutionError> {
        self.resolution
            .as_mut()
            .map_or(Ok(false), |r| r.observe(frame, milliseconds))
    }
    pub fn stats(&self) -> &SceneStats {
        &self.live.stats
    }

    pub(super) fn stage(&mut self, input: &SceneInput) -> Result<()> {
        crate::models::animation::validate_ik_capacity(input)?;
        if input.nodes.len() != self.bindings.len() || environment(input) != self.environment {
            return Err(SceneError::Input(
                "retained scene bindings changed; prepare a replacement",
            )
            .into());
        }
        self.light_plan.clear();
        for (node, binding) in input.nodes.iter().zip(&self.bindings) {
            if node.mesh.as_ref().is_some_and(|mesh| (mesh.animation.is_some() || mesh.animation_blend.is_some() || mesh.animation_pose.is_some() || mesh.animation_ik.is_some() || mesh.animation_root.is_some())
                && matches!(mesh.source.data(), incant_assets_runtime::RuntimeAssetData::Model(model) if model.animation.is_none())) {
                return Err(SceneError::Input("animation sample requires animated model").into());
            }
            if node.id != binding.id
                || node.camera != binding.camera
                || node.light != binding.light
                || node
                    .mesh
                    .as_ref()
                    .map(|m| (m.source.version_id(), m.cast_shadows))
                    != binding.mesh
                || node
                    .sprite
                    .as_ref()
                    .map(|s| (s.source.version_id(), &s.sprite))
                    != binding.sprite.as_ref().map(|(id, s)| (*id, s))
                || match (
                    node.sprite.as_ref().and_then(|s| s.frames.as_ref()),
                    &binding.frames,
                ) {
                    (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
                    (None, None) => false,
                    _ => true,
                }
                || node.sprite.as_ref().is_some_and(|s| {
                    usize::from(s.frame) >= s.frames.as_ref().map_or(1, |f| f.regions().len())
                })
                || match (&node.tilemap, &binding.tilemap) {
                    (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
                    (None, None) => false,
                    _ => true,
                }
                || node.diagnostic
            {
                return Err(SceneError::Input(
                    "retained scene bindings changed; prepare a replacement",
                )
                .into());
            }
            if !node.world.is_finite() || node.world.row(3) != glam::DVec4::W {
                return Err(SceneError::TransformRange.into());
            }
            Instance::new(node.world.as_mat4())?;
            if let Some(light) = &node.light {
                self.light_plan.add_typed(light, node.world)?;
            }
        }
        let node = &input.nodes[self.camera_index];
        self.staging.camera = crate::camera::CameraView::authored(
            node.camera.as_ref().expect("frozen camera binding"),
            node.world,
        )?;
        self.staging
            .lights
            .stage(&mut self.light_plan, &mut self.packed_lights);
        self.staging
            .lights
            .validate_view(self.staging.camera.view)?;
        super::animation::stage(&mut self.staging.animations, input)?;
        let mut draw_calls = 0;
        for batch in &mut self.staging.batches {
            // Only prepare_models/shadows borrow Arc copies, and their packets
            // are consumed/dropped before submit returns. No public scene escape.
            let batch = Arc::get_mut(batch).expect("private retained batch ownership");
            batch.updates.clear();
            batch.centers.clear();
            if !super::animation::stage_batch(batch, &self.staging.animations, input)? {
                let mesh = &batch.model.primitives[batch.primitive];
                let mut minimum = glam::Vec3::splat(f32::INFINITY);
                let mut maximum = glam::Vec3::splat(f32::NEG_INFINITY);
                for placement in batch.placements.iter() {
                    let world = (input.nodes[placement.node].world * placement.local).as_mat4();
                    let instance = Instance::new(world)?;
                    if (instance.normal[0][3] < 0.) != batch.mirrored {
                        continue;
                    }
                    for x in [mesh.minimum.x, mesh.maximum.x] {
                        for y in [mesh.minimum.y, mesh.maximum.y] {
                            for z in [mesh.minimum.z, mesh.maximum.z] {
                                let point = world.transform_point3(glam::Vec3::new(x, y, z));
                                if !point.is_finite() {
                                    return Err(crate::ResourceError::GeometryRange.into());
                                }
                                minimum = minimum.min(point);
                                maximum = maximum.max(point);
                            }
                        }
                    }
                    batch.centers.push(world.transform_point3(mesh.center));
                    batch.updates.push(instance);
                }
                batch.count = batch.updates.len() as u32;
                batch.bounds = crate::shadows::cascade::Bounds { minimum, maximum };
            }
            if batch.count > 0 {
                let mesh = &batch.model.primitives[batch.primitive];
                draw_calls += if batch.model.materials.materials[mesh.material].alpha
                    == incant_assets_runtime::AlphaMode::Blend
                {
                    batch.count as usize
                } else {
                    1
                };
            }
        }
        if let Some(sprites) = &mut self.staging.sprites {
            Arc::get_mut(sprites)
                .expect("private retained sprite ownership")
                .stage(input)?;
        }
        match (
            &mut self.staging.text,
            input.text.as_ref().filter(|text| !text.nodes.is_empty()),
        ) {
            (Some(text), Some(input)) => Arc::get_mut(text)
                .expect("private retained text ownership")
                .stage(input)?,
            (None, None) => {}
            _ => {
                return Err(SceneError::Input(
                    "retained text bindings changed; prepare a replacement",
                )
                .into());
            }
        }
        self.staging.stats.model_draw_calls = draw_calls;
        Ok(())
    }

    /// Validate/stage/encode before any frame buffer writes. An ordinary error
    /// leaves the accepted scene and its GPU buffers unchanged and is retryable.
    /// Successful calls write and immediately submit on the owning renderer's
    /// queue, then accept CPU metadata. Device loss/driver failure is terminal to
    /// the host, as in the ordinary renderer; it is not a recoverable transaction.
    pub fn submit(
        &mut self,
        renderer: &Renderer,
        input: &SceneInput,
        target: RetainedTarget<'_>,
    ) -> Result<wgpu::SubmissionIndex> {
        if !Arc::ptr_eq(&self.owner, &renderer.identity) {
            return Err(SceneError::Input("retained scene belongs to another renderer").into());
        }
        self.stage(input)?;
        let resolution = self
            .resolution
            .as_ref()
            .map(|r| {
                r.prepare(
                    [target.width, target.height],
                    target.viewport.map(|v| v.rect).unwrap_or([
                        0.,
                        0.,
                        target.width as f32,
                        target.height as f32,
                    ]),
                    self.staging.camera.pixel_resolution,
                )
            })
            .transpose()?;
        let scale = self
            .resolution
            .as_ref()
            .map_or(crate::resolution::ResolutionScale::FULL, |r| r.scale());
        let plan = crate::resolution::FramePlan::new(
            [target.width, target.height],
            target.viewport.map(|v| v.rect).unwrap_or([
                0.,
                0.,
                target.width as f32,
                target.height as f32,
            ]),
            self.staging.camera.pixel_resolution,
            scale,
        )?;
        #[cfg(feature = "native-metalfx")]
        let mut native_pending = self
            .metalfx
            .as_ref()
            .map(|owner| {
                let overlays = self.staging.sprites.is_some()
                    || self.staging.text.is_some()
                    || self.staging.batches.iter().any(|b| {
                        b.model.materials.materials[b.model.primitives[b.primitive].material].alpha
                            == incant_assets_runtime::AlphaMode::Blend
                    });
                owner.prepare(
                    renderer,
                    &self.staging,
                    overlays,
                    &plan,
                    target.viewport.map(|v| v.rect).unwrap_or([
                        0.,
                        0.,
                        target.width as f32,
                        target.height as f32,
                    ]),
                )
            })
            .transpose()?;
        #[cfg(feature = "native-metalfx")]
        let native_active = native_pending.as_ref().is_some_and(|p| p.native());
        #[cfg(not(feature = "native-metalfx"))]
        let native_active = false;
        if let Some(temporal) = &self.temporal
            && self.staging.camera.pixel_resolution.is_none()
        {
            temporal.preflight(
                &renderer.device,
                plan.size,
                self.motion
                    .as_ref()
                    .ok_or(crate::temporal::TemporalError::History)?
                    .target_bytes(),
            )?;
        }
        let motion = self
            .motion
            .as_mut()
            .map(|m| {
                m.prepare(
                    renderer,
                    &self.live,
                    &self.staging,
                    input,
                    crate::motion::MotionView {
                        output: [target.width, target.height],
                        plan: &plan,
                        temporal: self.temporal.is_some() || native_active,
                    },
                )
            })
            .transpose()?;
        let (motion_draw, motion_pending) = match motion {
            Some((draw, pending)) => (draw, Some(pending)),
            None => (None, None),
        };
        let temporal_pending = self
            .temporal
            .as_ref()
            .map(|temporal| {
                let pending = motion_pending
                    .as_ref()
                    .ok_or(crate::temporal::TemporalError::History)?;
                temporal.prepare(&renderer.device, pending.temporal_input(plan.rect))
            })
            .transpose()?;
        if let Some(phase) = motion_pending.as_ref().and_then(|p| p.jitter_phase()) {
            self.staging.camera.jitter = crate::temporal::jitter::ndc(phase, plan.rect);
            self.staging.camera.jitter_guard = crate::temporal::jitter::guard(plan.rect);
        }
        #[cfg(feature = "native-metalfx")]
        let commands = if let Some(pending) = native_pending.as_mut().filter(|p| p.native()) {
            let owner = self
                .metalfx
                .as_mut()
                .ok_or(crate::metalfx::MetalFxError::Interop)?;
            let frame = motion_pending
                .as_ref()
                .ok_or(crate::metalfx::MetalFxError::Interop)?
                .temporal_input(plan.rect)
                .frame;
            owner.bind_motion(pending, &frame)?;
            renderer.draw_scene_metalfx(
                &self.staging,
                target,
                &plan,
                motion_draw.ok_or(crate::metalfx::MetalFxError::Interop)?,
                owner,
                pending,
            )?
        } else {
            crate::graph::Commands::Single(renderer.draw_scene_scaled(
                &self.staging,
                target,
                scale,
                motion_draw,
                temporal_pending.as_ref(),
            )?)
        };
        #[cfg(not(feature = "native-metalfx"))]
        let commands = renderer.draw_scene_scaled(
            &self.staging,
            target,
            scale,
            motion_draw,
            temporal_pending.as_ref(),
        )?;
        for batch in &self.staging.batches {
            if !batch.updates.is_empty() {
                renderer.queue.write_buffer(
                    &batch.instances,
                    0,
                    bytemuck::cast_slice(&batch.updates),
                );
            }
        }
        self.staging
            .lights
            .write(&renderer.queue, &self.packed_lights);
        super::animation::write(&self.staging.animations, &renderer.queue);
        if let Some(sprites) = &self.staging.sprites {
            sprites.write(&renderer.queue);
        }
        if let Some(text) = &self.staging.text {
            text.write(&renderer.queue);
        }
        if let (Some(motion), Some(pending)) = (&self.motion, &motion_pending) {
            motion.write(&renderer.queue, pending);
        }
        #[cfg(feature = "native-metalfx")]
        let submission = renderer.queue.submit(commands);
        #[cfg(not(feature = "native-metalfx"))]
        let submission = renderer.queue.submit([commands]);
        #[cfg(feature = "native-metalfx")]
        if let (Some(owner), Some(pending)) = (&mut self.metalfx, native_pending) {
            owner.accept(pending);
        }
        std::mem::swap(&mut self.live, &mut self.staging);
        if let (Some(motion), Some(pending)) = (&mut self.motion, motion_pending) {
            motion.accept(pending, input);
        }
        if let (Some(temporal), Some(pending)) = (&mut self.temporal, temporal_pending) {
            temporal.accept(pending);
        }
        if let (Some(owner), Some(frame)) = (&mut self.resolution, resolution) {
            owner.accept(frame);
        }
        Ok(submission)
    }
}

#[cfg(test)]
#[path = "retained_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "retained_native_tests.rs"]
mod native_tests;

#[cfg(test)]
#[path = "retained_sprite_tests.rs"]
mod sprite_tests;

#[cfg(test)]
#[path = "retained_tilemap_tests.rs"]
mod tilemap_tests;

#[cfg(test)]
#[path = "flipbook_tests.rs"]
mod flipbook_tests;

#[cfg(test)]
#[path = "text_retained_tests.rs"]
mod text_tests;

#[cfg(test)]
#[path = "text_native_tests.rs"]
mod text_native_tests;

#[cfg(test)]
#[path = "motion_device_tests.rs"]
mod motion_tests;

#[cfg(test)]
#[path = "temporal_retained_tests.rs"]
mod temporal_tests;

#[cfg(test)]
#[path = "motion_owner_device_tests.rs"]
mod motion_owner_tests;

#[cfg(all(test, feature = "native-metalfx"))]
#[path = "metalfx_retained_tests.rs"]
mod metalfx_tests;
