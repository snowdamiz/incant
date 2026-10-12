//! Read-only extraction over retained native topology, with no frame allocation.
use crate::world::{Global, Hierarchy, Identity, Local};
use crate::{CookedSpatial, NativeWorld, RuntimeError, StableId, VisualAssetRef};
use incant_types::Transform;

/// Borrowed continuous values in stable parent-before-child order for this
/// topology generation. Indices are not ECS table rows or persistent entity IDs.
pub struct SpatialNode<'a> {
    pub index: usize,
    pub parent_index: Option<usize>,
    pub id: StableId,
    pub local: &'a Transform,
    pub global: &'a glam::DMat4,
    pub declaration: Option<&'a CookedSpatial>,
    /// Immutable cell placements/collision union prepared at binding changes.
    pub tilemap: Option<&'a crate::PreparedTilemap>,
    pub flipbook: Option<crate::FlipbookState>,
    pub animation: Option<crate::AnimationSample>,
    pub animation_ik: Option<&'a std::sync::Arc<[crate::AnimationIkSample]>>,
    /// Outgoing sample and destination weight from the same committed state.
    pub animation_blend: Option<incant_types::AnimationBlendSample>,
    pub animation_pose: Option<incant_types::AnimationPoseSamples>,
}

/// Scoped view at a committed boundary. Numeric/structural mutation cannot
/// overlap the borrow. No authoring projection, ancestor walks or copies occur.
/// Consumers may reuse their own typed buffers keyed by these generations;
/// generations belong to this world instance, not a newly constructed world.
pub struct SpatialView<'a> {
    world: &'a NativeWorld,
}
impl NativeWorld {
    /// Lend committed globals, never the hierarchy validation scratch buffer.
    /// A failed solver makes the world unusable; ordinary rejected ticks/batches
    /// retain the prior values and generations. Trusted immediate numeric writers
    /// remain responsible for valid values, as documented on their loan API.
    ///
    /// ```compile_fail
    /// # use incant_runtime::NativeWorld;
    /// fn overlap(world: &mut NativeWorld) {
    ///     let view = world.spatial_view().unwrap();
    ///     world.step().unwrap();
    ///     let _ = view.nodes().count();
    /// }
    /// ```
    pub fn spatial_view(&self) -> Result<SpatialView<'_>, RuntimeError> {
        self.ensure_usable()?;
        Ok(SpatialView { world: self })
    }
}
impl<'a> SpatialView<'a> {
    /// Check this identity before trusting retained generation/index mappings.
    pub fn identity(&self) -> &'a crate::WorldIdentity {
        &self.world.identity
    }
    pub fn scene_id(&self) -> StableId {
        self.world.scene_id()
    }
    pub fn tick(&self) -> u64 {
        self.world.tick()
    }
    pub fn topology_generation(&self) -> u64 {
        self.world.topology_generation
    }
    pub fn binding_generation(&self) -> u64 {
        self.world.binding_generation
    }
    /// Consumers without a text pipeline must refuse these declarations.
    pub fn has_text(&self) -> bool {
        !self.world.font_assets.is_empty() || !self.world.text_ids.is_empty()
    }
    pub fn assets(&self) -> &'a [VisualAssetRef] {
        &self.world.visual_assets
    }
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = SpatialNode<'a>> + 'a {
        let world = &self.world.world;
        world.resource::<Hierarchy>().nodes.iter().enumerate().map(
            move |(index, &(entity, parent_index))| SpatialNode {
                index,
                parent_index,
                id: world.get::<Identity>(entity).expect("spatial identity").0,
                local: &world.get::<Local>(entity).expect("spatial local").0,
                global: &world
                    .get::<Global>(entity)
                    .expect("committed spatial global")
                    .0,
                declaration: world.get::<CookedSpatial>(entity),
                animation_ik: world.get::<crate::ik::IkState>(entity).map(|s| &s.0),
                animation: world
                    .get::<crate::animation::Playback>(entity)
                    .map(|p| p.state.sample),
                animation_pose: world
                    .get::<crate::animation::Playback>(entity)
                    .and_then(|p| p.pose_sample()),
                animation_blend: world
                    .get::<crate::animation::Playback>(entity)
                    .and_then(|p| p.blend_sample()),
                flipbook: world
                    .get::<crate::flipbook::Playback>(entity)
                    .map(|p| p.state),
                tilemap: world
                    .get::<crate::tilemap::Prepared>(entity)
                    .map(|prepared| &prepared.0),
            },
        )
    }
}
