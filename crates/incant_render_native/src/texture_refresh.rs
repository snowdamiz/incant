//! Host-issued catalog authority. Cooked decoding happens before live admission.
//! A ticket is not a project capability and carries no mutable world or source IO.
use super::*;
use incant_assets_runtime::RuntimeAssetData;
use incant_runtime::{VisualAssetKind, VisualAssetRef};

#[derive(Clone)]
pub struct TextureRefreshTicket {
    identity: WorldIdentity,
    generation: u64,
    topology: u64,
    bindings: u64,
    text_generation: u64,
    assets: BTreeMap<StableId, Arc<RuntimeAsset>>,
    navigation_diverged_models: Vec<StableId>,
    retained_text_bytes: usize,
}
pub struct PreparedTextureRefresh {
    ticket: TextureRefreshTicket,
    assets: BTreeMap<StableId, Arc<RuntimeAsset>>,
    payload_bytes: usize,
    changed: usize,
    // Cold decoded classification, not authority supplied by project data.
    navigation_diverged_models: Vec<StableId>,
    // Only changed models participate in this publication's dependency check.
    navigation_changed_models: Vec<StableId>,
}
/// Fully allocated CPU candidate. Final publication only checks authority and
/// copies current accepted numeric poses into this existing storage.
pub struct StagedTextureRefresh {
    prepared: PreparedTextureRefresh,
    candidate: binding::Prepared,
    generation: u64,
    used: bool,
    // An intervening simulation update can replace a staged IK owner. Retain
    // that displaced payload until the publication is retired after unlocking.
    retired_ik: Vec<Option<Arc<[incant_runtime::AnimationIkSample]>>>,
}
impl StagedTextureRefresh {
    pub fn input(&self) -> &SceneInput {
        &self.candidate.input
    }
    pub fn camera_label(&self) -> &str {
        &self.candidate.input.nodes[self.candidate.camera_slot].id
    }
    pub fn changed(&self) -> usize {
        self.prepared.changed
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureRefreshReceipt {
    pub generation: u64,
    pub changed: usize,
}
fn refused(why: &'static str) -> NativeRenderError {
    NativeRenderError::TextureRefresh(why)
}
fn fingerprint(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
impl TextureRefreshTicket {
    /// Cold no-op detection against the currently accepted catalog, without IO.
    pub fn matches_recipe(&self, id: StableId, kind: VisualAssetKind, fingerprint: &str) -> bool {
        self.assets.get(&id).is_some_and(|asset| {
            asset.info().fingerprint == fingerprint
                && matches!(
                    (kind, asset.data()),
                    (VisualAssetKind::Texture, RuntimeAssetData::Texture(_))
                        | (VisualAssetKind::Model, RuntimeAssetData::Model(_))
                )
        })
    }
    /// Texture-only compatibility entry point. Models remain byte-identical;
    /// texture IDs and interpreted usage must remain unchanged. Readers validate
    /// actual cooked bytes, including every atlas binding during final admission.
    /// Published payload is bounded; staging plus retained old versions is extra.
    pub fn prepare(
        self,
        references: &[VisualAssetRef],
        texture_cache: &Path,
    ) -> Result<PreparedTextureRefresh, NativeRenderError> {
        self.prepare_with_budget(references, texture_cache, MAX_PAYLOAD_BYTES)
    }
    /// Trusted hosts may select a smaller decoded-payload budget, never a larger
    /// one. Staged and retained versions remain outside this published-set cap.
    pub fn prepare_with_budget(
        self,
        references: &[VisualAssetRef],
        texture_cache: &Path,
        budget: usize,
    ) -> Result<PreparedTextureRefresh, NativeRenderError> {
        self.prepare_catalog(references, texture_cache, None, budget)
    }
    /// Trusted-host admission for compatible existing model/texture content,
    /// including complete model-local material tables. Animated clips retain
    /// pose layout and exact runtime duration bindings.
    /// All cache IO/decoding remains outside publication locks.
    pub fn prepare_visuals(
        self,
        references: &[VisualAssetRef],
        caches: VisualCaches<'_>,
    ) -> Result<PreparedTextureRefresh, NativeRenderError> {
        self.prepare_visuals_with_budget(references, caches, MAX_PAYLOAD_BYTES)
    }
    pub fn prepare_visuals_with_budget(
        self,
        references: &[VisualAssetRef],
        caches: VisualCaches<'_>,
        budget: usize,
    ) -> Result<PreparedTextureRefresh, NativeRenderError> {
        self.prepare_catalog(references, caches.textures, Some(caches.models), budget)
    }
    fn prepare_catalog(
        self,
        references: &[VisualAssetRef],
        texture_cache: &Path,
        model_cache: Option<&Path>,
        budget: usize,
    ) -> Result<PreparedTextureRefresh, NativeRenderError> {
        if budget > MAX_PAYLOAD_BYTES || self.retained_text_bytes > budget {
            return Err(NativeRenderError::PayloadLimit);
        }
        if references.len() != self.assets.len()
            || !references.windows(2).all(|r| r[0].id < r[1].id)
        {
            return Err(refused("catalog IDs changed"));
        }
        let mut assets = BTreeMap::new();
        let mut payload_bytes = self.retained_text_bytes;
        let mut changed = 0;
        let mut navigation_diverged_models = Vec::with_capacity(self.assets.len());
        let mut navigation_changed_models = Vec::new();
        for reference in references {
            let mut navigation_changed = self
                .navigation_diverged_models
                .binary_search(&reference.id)
                .is_ok();
            let old = self
                .assets
                .get(&reference.id)
                .ok_or_else(|| refused("unknown asset ID"))?;
            let same = old.info().fingerprint == fingerprint(&reference.fingerprint);
            let next = match (reference.kind, old.data()) {
                (VisualAssetKind::Model, RuntimeAssetData::Model(_)) if same => Arc::clone(old),
                (VisualAssetKind::Model, RuntimeAssetData::Model(old_model)) => {
                    let cache = model_cache
                        .ok_or_else(|| refused("only existing texture content may change"))?;
                    let generation = self
                        .generation
                        .checked_add(1)
                        .ok_or_else(|| refused("catalog generation exhausted"))?;
                    let asset = RuntimeAsset::load_visual(reference, generation, cache).map_err(
                        |source| NativeRenderError::Asset {
                            asset: reference.id,
                            source,
                        },
                    )?;
                    let RuntimeAssetData::Model(next_model) = asset.data() else {
                        return Err(refused("model payload kind changed"));
                    };
                    let change = super::model_refresh::validate(old_model, next_model)?;
                    navigation_changed |= change.navigation_geometry_changed;
                    if navigation_changed {
                        navigation_changed_models.push(reference.id);
                    }
                    changed += 1;
                    Arc::new(asset)
                }
                (VisualAssetKind::Texture, RuntimeAssetData::Texture(_)) if same => Arc::clone(old),
                (VisualAssetKind::Texture, RuntimeAssetData::Texture(_)) => {
                    let generation = self
                        .generation
                        .checked_add(1)
                        .ok_or_else(|| refused("catalog generation exhausted"))?;
                    let asset = RuntimeAsset::load_visual(reference, generation, texture_cache)
                        .map_err(|source| NativeRenderError::Asset {
                            asset: reference.id,
                            source,
                        })?;
                    if asset.usage() != old.usage() {
                        return Err(refused("texture usage changed"));
                    }
                    changed += 1;
                    Arc::new(asset)
                }
                _ => return Err(refused("only existing texture content may change")),
            };
            payload_bytes = payload_bytes
                .checked_add(next.info().payload_bytes)
                .filter(|n| *n <= budget)
                .ok_or(NativeRenderError::PayloadLimit)?;
            if navigation_changed {
                navigation_diverged_models.push(reference.id);
            }
            assets.insert(reference.id, next);
        }
        Ok(PreparedTextureRefresh {
            ticket: self,
            assets,
            payload_bytes,
            changed,
            navigation_diverged_models,
            navigation_changed_models,
        })
    }
}
impl NativeRenderInput {
    pub fn asset_generation(&self) -> u64 {
        self.asset_generation
    }
    /// Clone cold immutable catalog handles only at an explicit host request.
    pub fn texture_refresh_ticket(&self) -> TextureRefreshTicket {
        TextureRefreshTicket {
            identity: self.identity.clone(),
            generation: self.asset_generation,
            topology: self.topology_generation,
            bindings: self.binding_generation,
            text_generation: self.text_generation,
            assets: self.assets.clone(),
            navigation_diverged_models: self.navigation_diverged_models.clone(),
            retained_text_bytes: self.text.as_ref().map_or(0, |b| b.bytes),
        }
    }
    fn validate_refresh(
        &self,
        view: &SpatialView<'_>,
        ticket: &TextureRefreshTicket,
    ) -> Result<(), NativeRenderError> {
        if self.identity != ticket.identity || &self.identity != view.identity() {
            return Err(NativeRenderError::DifferentWorld);
        }
        if self.asset_generation != ticket.generation
            || self.topology_generation != ticket.topology
            || self.binding_generation != ticket.bindings
            || self.text_generation != ticket.text_generation
            || view.topology_generation() != ticket.topology
            || view.binding_generation() != ticket.bindings
            || view.tick() != self.tick
            || self.assets.len() != ticket.assets.len()
            || self.navigation_diverged_models != ticket.navigation_diverged_models
            || self.assets.iter().any(|(id, asset)| {
                ticket
                    .assets
                    .get(id)
                    .is_none_or(|old| old.version_id() != asset.version_id())
            })
        {
            return Err(refused("stale catalog or changed topology/bindings"));
        }
        Ok(())
    }
    pub fn stage_texture_refresh(
        &self,
        view: &SpatialView<'_>,
        prepared: PreparedTextureRefresh,
    ) -> Result<StagedTextureRefresh, NativeRenderError> {
        self.validate_refresh(view, &prepared.ticket)?;
        Self::validate_model_dependencies(view, &prepared)?;
        let generation = if prepared.changed == 0 {
            self.asset_generation
        } else {
            self.asset_generation
                .checked_add(1)
                .ok_or_else(|| refused("catalog generation exhausted"))?
        };
        let mut candidate = binding::prepare(view, &prepared.assets, self.camera)?;
        candidate.input.text = self.input.text.clone();
        let retired_ik = vec![None; candidate.input.nodes.len()];
        Ok(StagedTextureRefresh {
            prepared,
            candidate,
            generation,
            used: false,
            retired_ik,
        })
    }
    pub fn validate_texture_publication(
        &self,
        view: &SpatialView<'_>,
        staged: &StagedTextureRefresh,
    ) -> Result<(), NativeRenderError> {
        if staged.used {
            return Err(refused("texture publication ticket was already used"));
        }
        self.validate_refresh(view, &staged.prepared.ticket)
            .and_then(|_| Self::validate_model_dependencies(view, &staged.prepared))
    }
    fn validate_model_dependencies(
        view: &SpatialView<'_>,
        prepared: &PreparedTextureRefresh,
    ) -> Result<(), NativeRenderError> {
        // Cold-decoded geometry divergence is latched relative to startup, even
        // across subsequent material changes or Undo. Original model payloads
        // need not be retained. Check current dependencies at both stage and
        // publication, including navigation added during GPU preparation.
        // Final validation remains allocation-free and visits navigation sources
        // once, rather than scanning all sources separately for every model.
        if !prepared.navigation_changed_models.is_empty()
            && view.navigation_model_assets().any(|id| {
                prepared
                    .navigation_changed_models
                    .binary_search(&id)
                    .is_ok()
            })
        {
            return Err(refused(
                "model geometry diverged from startup and supplies baked navigation; restart Play",
            ));
        }
        Ok(())
    }
    /// Final cold operation. No allocation/source IO occurs on the success path;
    /// the callback must publish an already-prepared consumer without GPU work.
    /// Its refusal/unwind preserves the complete accepted CPU catalog and input.
    /// The caller retains this single-use storage throughout its final lock, then
    /// drops the rejected candidate or retired owners after unlocking.
    pub fn try_publish_textures<E>(
        &mut self,
        view: &SpatialView<'_>,
        staged: &mut StagedTextureRefresh,
        admit: impl FnOnce() -> Result<(), E>,
    ) -> Result<TextureRefreshReceipt, SyncError<E>> {
        self.validate_texture_publication(view, staged)?;
        staged.used = true;
        if staged.prepared.changed == 0 {
            return Ok(TextureRefreshReceipt {
                generation: self.asset_generation,
                changed: 0,
            });
        }
        let candidate = &mut staged.candidate;
        // Simulation may move while resources are staged. Topology/configuration
        // and text generation were checked above; current numeric data needs no
        // allocation and the next ordinary submit sees this committed pose.
        for ((next, current), retired_ik) in candidate
            .input
            .nodes
            .iter_mut()
            .zip(&self.input.nodes)
            .zip(&mut staged.retired_ik)
        {
            next.world = current.world;
            if let (Some(next), Some(current)) = (&mut next.light, &current.light) {
                next.set_values(current.values());
            }
            if let (Some(next), Some(current)) = (&mut next.mesh, &current.mesh) {
                next.animation = current.animation;
                next.animation_blend = current.animation_blend;
                next.animation_pose = current.animation_pose;
                // This single-use storage is empty before publication. Moving
                // the staged owner first avoids disposing it under the lock.
                *retired_ik = next.animation_ik.take();
                next.animation_ik = current.animation_ik.clone();
                next.animation_root = current.animation_root;
            }
            if let (Some(next), Some(current)) = (&mut next.sprite, &current.sprite) {
                next.frame = current.frame;
            }
        }
        if let (Some(next), Some(current)) = (&mut candidate.input.text, &self.input.text) {
            for (next, current) in next.nodes.iter_mut().zip(&current.nodes) {
                next.world = current.world;
            }
        }
        admit().map_err(SyncError::Consumer)?;
        std::mem::swap(&mut self.assets, &mut staged.prepared.assets);
        std::mem::swap(
            &mut self.navigation_diverged_models,
            &mut staged.prepared.navigation_diverged_models,
        );
        std::mem::swap(&mut self.payload_bytes, &mut staged.prepared.payload_bytes);
        std::mem::swap(&mut self.input, &mut staged.candidate.input);
        std::mem::swap(&mut self.slots, &mut staged.candidate.slots);
        std::mem::swap(&mut self.undo, &mut staged.candidate.undo);
        std::mem::swap(&mut self.camera_slot, &mut staged.candidate.camera_slot);
        self.asset_generation = staged.generation;
        let receipt = TextureRefreshReceipt {
            generation: staged.generation,
            changed: staged.prepared.changed,
        };
        Ok(receipt)
    }
}
