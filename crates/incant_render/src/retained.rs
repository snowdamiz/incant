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
/// This path accepts native camera/model/sprite/light/environment inputs.
/// Frame encoding still allocates bounded draw packets/uniform resources; this is
/// not a claim that the complete renderer has reached its shipping budgets.
pub struct RetainedScene {
    owner: Arc<()>,
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
        if input.text.is_some() {
            return Err(SceneError::Input("text drawing is not implemented").into());
        }
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
        let mut staging = self.upload_scene(staging_plan, true)?.with_camera(camera)?;
        staging.sprites = live
            .sprites
            .as_ref()
            .map(|sprites| Arc::new(sprites.staging(&self.device)));
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
    pub fn stats(&self) -> &SceneStats {
        &self.live.stats
    }

    pub(super) fn stage(&mut self, input: &SceneInput) -> Result<()> {
        if input.text.is_some() {
            return Err(SceneError::Input("text drawing is not implemented").into());
        }
        if input.nodes.len() != self.bindings.len() || environment(input) != self.environment {
            return Err(SceneError::Input(
                "retained scene bindings changed; prepare a replacement",
            )
            .into());
        }
        self.light_plan.clear();
        for (node, binding) in input.nodes.iter().zip(&self.bindings) {
            if node.mesh.as_ref().is_some_and(|mesh| (mesh.animation.is_some() || mesh.animation_blend.is_some())
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
        let commands = renderer.draw_scene(
            &self.staging,
            target.view,
            target.format,
            target.width,
            target.height,
            target.viewport,
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
        let submission = renderer.queue.submit([commands]);
        std::mem::swap(&mut self.live, &mut self.staging);
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
