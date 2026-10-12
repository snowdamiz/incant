//! Retained CPU input for the native renderer. Bind checked cooked versions once;
//! ordinary frames only copy committed f64 poses into existing typed slots.
mod text;
pub use text::{MAX_TEXT_CHAINS, MAX_TEXT_GLYPHS, MAX_TEXT_LINES, TextSettings};
mod animation;
mod binding;
mod tilemap;
pub use tilemap::TilemapAtlasBinding;
mod update;
use glam::DMat4;
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::SceneInput;
use incant_runtime::{SpatialView, StableId, WorldIdentity};
use std::{collections::BTreeMap, path::Path, sync::Arc};
use thiserror::Error;

/// Published decoded vertex/index/mip/raw-font payload, excluding metadata/allocator/GPU
/// overhead. Startup may stage one bounded asset beyond this aggregate before
/// rejecting. Retained prior adapters also contribute to process peak memory.
pub const MAX_PAYLOAD_BYTES: usize = 256 * 1024 * 1024;

/// Explicit cooked cache directories. Reads occur only during `load`.
pub struct VisualCaches<'a> {
    pub models: &'a Path,
    pub textures: &'a Path,
}
#[derive(Debug, Error)]
pub enum NativeRenderError {
    #[error(transparent)]
    Text(#[from] incant_text::TextError),
    #[error(transparent)]
    Font(#[from] incant_assets_runtime::AssetError),
    #[error("native text catalog/topology binding mismatch")]
    TextBinding,
    #[error("native text consumer limit: {0}")]
    TextLimit(&'static str),
    /// Text needs the composed binding with an explicit checked font catalog;
    /// the spatial-only adapter and syncs without a text view refuse it.
    #[error("native text requires the composed text binding with an explicit font catalog")]
    TextUnsupported,
    #[error("spatial input belongs to a different native world")]
    DifferentWorld,
    #[error("native animation {entity:?}: {reason}")]
    Animation {
        entity: StableId,
        reason: &'static str,
    },
    #[error("tilemap render binding failed: {0}")]
    TilemapRender(incant_render_data::SceneError),
    #[error("tilemap requires a matching checked color atlas descriptor and cache")]
    TilemapBinding,
    #[error("tilemap frame {0:?} is absent from the checked atlas")]
    TilemapFrame(StableId),
    #[error("retained spatial index layout is inconsistent")]
    Layout,
    #[error("visual asset {asset:?} failed to load: {source}")]
    Asset {
        asset: StableId,
        #[source]
        source: incant_assets_runtime::AssetError,
    },
    #[error("decoded visual payload exceeds {MAX_PAYLOAD_BYTES} bytes")]
    PayloadLimit,
    #[error("visual asset {0:?} is not bound")]
    MissingAsset(StableId),
    #[error("native camera {0:?} was not found or has no Camera declaration")]
    MissingCamera(StableId),
    #[error("normal textures cannot provide environment radiance")]
    EnvironmentUsage,
    #[error("native sprite {entity:?} has invalid texture usage, region or atlas frame")]
    Sprite { entity: StableId },
}
#[derive(Debug, Error)]
pub enum SyncError<E> {
    #[error(transparent)]
    Native(#[from] NativeRenderError),
    #[error("render consumer rejected the input: {0}")]
    Consumer(E),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Whether the consumer can reuse retained model/configuration expansion.
pub enum UpdateKind {
    Motion,
    Rebuilt,
}

/// Attached to exactly one world, with immutable startup assets and an explicitly
/// selected camera. Configuration and topology rebuilds publish atomically;
/// unchanged metadata retains every String/Arc/Vec allocation between frames.
/// Sprite bindings resolve checked atlas metadata at configuration changes; no
/// diagnostic cubes are synthesized by this adapter. Selection is
/// fixed for its lifetime: removing that camera rejects sync until it is restored
/// or the host explicitly constructs a new adapter. A published input can lag the
/// native world after consumer rejection; synchronize again before drawing it as
/// the current simulation frame.
#[derive(Clone, Copy)]
struct NumericUndo {
    world: DMat4,
    frame: u16,
    animation: Option<incant_runtime::AnimationSample>,
    animation_blend: Option<incant_runtime::AnimationBlendSample>,
}

pub struct NativeRenderInput {
    identity: WorldIdentity,
    assets: BTreeMap<StableId, Arc<RuntimeAsset>>,
    input: SceneInput,
    slots: Vec<Option<usize>>,
    undo: Vec<NumericUndo>,
    camera: StableId,
    camera_slot: usize,
    topology_generation: u64,
    binding_generation: u64,
    tick: u64,
    payload_bytes: usize,
    text: Option<text::Binding>,
    text_generation: u64,
    text_slots: Vec<Option<usize>>,
    text_undo: Vec<DMat4>,
}

impl NativeRenderInput {
    /// Startup only. No source files or project are consulted. A caller publishes
    /// this adapter only after its GPU consumer accepts input()/camera_label().
    /// Expanded model primitive/material/GPU limits belong to that consumer.
    pub fn load(
        view: &SpatialView<'_>,
        caches: VisualCaches<'_>,
        camera: StableId,
    ) -> Result<Self, NativeRenderError> {
        Self::load_inner(view, None, caches, camera)
    }
    /// Composed startup: the text view must come from this exact world. Checked
    /// font bytes/layouts/atlas enter the same canonical consumer transaction.
    pub fn load_with_text(
        view: &SpatialView<'_>,
        text: &incant_runtime::TextView<'_>,
        caches: VisualCaches<'_>,
        camera: StableId,
        settings: TextSettings,
    ) -> Result<Self, NativeRenderError> {
        if view.identity() != text.identity() {
            return Err(NativeRenderError::DifferentWorld);
        }
        Self::load_inner(view, Some((text, settings)), caches, camera)
    }
    fn load_inner(
        view: &SpatialView<'_>,
        text: Option<(&incant_runtime::TextView<'_>, TextSettings)>,
        caches: VisualCaches<'_>,
        camera: StableId,
    ) -> Result<Self, NativeRenderError> {
        if view.has_text() && text.is_none() {
            return Err(NativeRenderError::TextUnsupported);
        }
        // An empty immutable font catalog can never acquire a valid Text
        // declaration. Preserve the old spatial-only allocation path.
        let text = text.filter(|(text, _)| !text.fonts().is_empty());
        let (assets, mut payload_bytes) = binding::load(view.assets(), caches)?;
        let mut prepared = binding::prepare(view, &assets, camera)?;
        let mut text_generation = 0;
        let mut text_slots = Vec::new();
        let mut text_undo = Vec::new();
        let text = text
            .map(|(view_text, settings)| {
                let binding = text::Binding::load(view_text, settings)?;
                payload_bytes = payload_bytes
                    .checked_add(binding.bytes)
                    .filter(|n| *n <= MAX_PAYLOAD_BYTES)
                    .ok_or(NativeRenderError::PayloadLimit)?;
                let candidate = binding.prepare(view_text, view)?;
                prepared.input.text = candidate.input;
                text_slots = candidate.slots;
                text_undo = candidate.undo;
                text_generation = view_text.generation();
                Ok::<_, NativeRenderError>(binding)
            })
            .transpose()?;
        Ok(Self {
            identity: view.identity().clone(),
            assets,
            input: prepared.input,
            slots: prepared.slots,
            undo: prepared.undo,
            camera,
            camera_slot: prepared.camera_slot,
            topology_generation: view.topology_generation(),
            binding_generation: view.binding_generation(),
            tick: view.tick(),
            payload_bytes,
            text,
            text_generation,
            text_slots,
            text_undo,
        })
    }
    pub fn has_text_binding(&self) -> bool {
        self.text.is_some()
    }
    pub fn text_generation(&self) -> u64 {
        self.text_generation
    }
    pub fn input(&self) -> &SceneInput {
        &self.input
    }
    pub fn camera_id(&self) -> StableId {
        self.camera
    }
    /// Cached informational label understood by RenderScene::with_camera.
    pub fn camera_label(&self) -> &str {
        &self.input.nodes[self.camera_slot].id
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn topology_generation(&self) -> u64 {
        self.topology_generation
    }
    pub fn binding_generation(&self) -> u64 {
        self.binding_generation
    }
    pub fn payload_bytes(&self) -> usize {
        self.payload_bytes
    }
}
