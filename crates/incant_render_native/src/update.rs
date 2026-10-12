use super::*;

impl NativeRenderInput {
    /// Propose a committed native frame to a consumer. The callback receives the
    /// explicitly selected camera's cached label; no label formatting is needed.
    /// Rejection or Rust unwind preserves this adapter's prior input, tick and
    /// generations. Abort cannot run cleanup. Cross-world input is rejected
    /// before writes or callback execution, even if all IDs/generations match.
    ///
    /// The callback must preflight before mutating externally published state or
    /// make that external consumer terminal on failure. This guard cannot undo
    /// GPU writes/device side effects. Return a newly prepared resource to publish
    /// alongside the accepted adapter, or use a transactional retained GPU path.
    /// Ordinary motion, model animation and flipbook selection reuse a bounded numeric undo buffer: no Vec allocations,
    /// String copies, Arc clones, source reads or model hierarchy expansion.
    pub fn try_sync<T, E>(
        &mut self,
        view: &SpatialView<'_>,
        accept: impl FnOnce(&SceneInput, &str, UpdateKind) -> Result<T, E>,
    ) -> Result<T, SyncError<E>> {
        self.sync_inner(view, None, accept)
    }
    /// Synchronize text and spatial state together. No source IO is performed;
    /// all changed text is prepared before invoking the mandatory consumer.
    pub fn try_sync_with_text<T, E>(
        &mut self,
        view: &SpatialView<'_>,
        text: &incant_runtime::TextView<'_>,
        accept: impl FnOnce(&SceneInput, &str, UpdateKind) -> Result<T, E>,
    ) -> Result<T, SyncError<E>> {
        if view.identity() != text.identity() {
            return Err(NativeRenderError::DifferentWorld.into());
        }
        self.sync_inner(view, Some(text), accept)
    }
    fn sync_inner<T, E>(
        &mut self,
        view: &SpatialView<'_>,
        text: Option<&incant_runtime::TextView<'_>>,
        accept: impl FnOnce(&SceneInput, &str, UpdateKind) -> Result<T, E>,
    ) -> Result<T, SyncError<E>> {
        if &self.identity != view.identity() {
            return Err(NativeRenderError::DifferentWorld.into());
        }
        if (view.has_text() || self.text.is_some()) && (text.is_none() || self.text.is_none()) {
            return Err(NativeRenderError::TextUnsupported.into());
        }
        if let Some((binding, text)) = self.text.as_ref().zip(text) {
            binding.validate(text)?;
        }
        let text_generation = text.map_or(0, |v| v.generation());
        if self.text_generation != text_generation
            || self.topology_generation != view.topology_generation()
            || self.binding_generation != view.binding_generation()
        {
            let mut candidate = binding::prepare(view, &self.assets, self.camera)?;
            let mut prepared_text = if let Some((binding, text)) = self.text.as_ref().zip(text) {
                Some(if self.text_generation == text_generation {
                    crate::text::map(self.input.text.clone(), view)?
                } else {
                    binding.prepare(text, view)?
                })
            } else {
                None
            };
            if let Some(prepared) = &mut prepared_text {
                candidate.input.text = prepared.input.take();
            }
            let result = accept(
                &candidate.input,
                &candidate.input.nodes[candidate.camera_slot].id,
                UpdateKind::Rebuilt,
            )
            .map_err(SyncError::Consumer)?;
            self.input = candidate.input;
            self.slots = candidate.slots;
            self.undo = candidate.undo;
            self.camera_slot = candidate.camera_slot;
            if let Some(prepared) = prepared_text {
                self.text_slots = prepared.slots;
                self.text_undo = prepared.undo;
            }
            self.text_generation = text_generation;
            self.topology_generation = view.topology_generation();
            self.binding_generation = view.binding_generation();
            self.tick = view.tick();
            return Ok(result);
        }
        if view.nodes().len() != self.slots.len() || self.undo.len() != self.input.nodes.len() {
            return Err(NativeRenderError::Layout.into());
        }
        if self.text.is_some()
            && (view.nodes().len() != self.text_slots.len()
                || self.text_undo.len() != self.input.text.as_ref().map_or(0, |t| t.nodes.len()))
        {
            return Err(NativeRenderError::Layout.into());
        }
        if let Some(text) = &self.input.text {
            for (old, node) in self.text_undo.iter_mut().zip(&text.nodes) {
                *old = node.world;
            }
        }
        // Save the entire old numeric set before arming the guard. No user code
        // runs during these copies. Configuration/index layout is private and
        // unchanged for this world identity + accepted generation pair.
        for (old, node) in self.undo.iter_mut().zip(&self.input.nodes) {
            *old = NumericUndo {
                world: node.world,
                frame: node.sprite.as_ref().map_or(0, |s| s.frame),
                animation: node.mesh.as_ref().and_then(|m| m.animation),
                animation_blend: node.mesh.as_ref().and_then(|m| m.animation_blend),
                animation_pose: node.mesh.as_ref().and_then(|m| m.animation_pose),
                animation_ik: node.mesh.as_ref().and_then(|m| m.animation_ik.clone()),
            };
        }
        let mut guard = MotionGuard {
            input: &mut self.input,
            undo: &self.undo,
            text_undo: &self.text_undo,
            accepted: false,
        };
        for node in view.nodes() {
            if let Some(text) = &mut guard.input.text
                && let Some(slot) = self.text_slots[node.index]
            {
                text.nodes[slot].world = *node.global;
            }
            if let Some(slot) = self.slots[node.index] {
                guard.input.nodes[slot].world = *node.global;
                if let Some(mesh) = &mut guard.input.nodes[slot].mesh {
                    mesh.animation = node.animation;
                    mesh.animation_blend = node.animation_blend;
                    mesh.animation_pose = node.animation_pose;
                    mesh.animation_ik = node.animation_ik.cloned();
                }
                if let Some(sprite) = &mut guard.input.nodes[slot].sprite {
                    sprite.frame = node.flipbook.map_or(0, |p| p.frame());
                }
            }
        }
        let result = accept(
            guard.input,
            &guard.input.nodes[self.camera_slot].id,
            UpdateKind::Motion,
        )
        .map_err(SyncError::Consumer)?;
        guard.accepted = true;
        self.tick = view.tick();
        Ok(result)
    }
}
struct MotionGuard<'a> {
    input: &'a mut SceneInput,
    undo: &'a [NumericUndo],
    text_undo: &'a [DMat4],
    accepted: bool,
}
impl Drop for MotionGuard<'_> {
    fn drop(&mut self) {
        if !self.accepted {
            if let Some(text) = &mut self.input.text {
                for (node, old) in text.nodes.iter_mut().zip(self.text_undo) {
                    node.world = *old;
                }
            }
            for (node, old) in self.input.nodes.iter_mut().zip(self.undo) {
                node.world = old.world;
                if let Some(mesh) = &mut node.mesh {
                    mesh.animation = old.animation;
                    mesh.animation_blend = old.animation_blend;
                    mesh.animation_pose = old.animation_pose;
                    mesh.animation_ik.clone_from(&old.animation_ik);
                }
                if let Some(sprite) = &mut node.sprite {
                    sprite.frame = old.frame;
                }
            }
        }
    }
}
