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
    /// Ordinary motion reuses a bounded numeric undo buffer: no Vec allocations,
    /// String copies, Arc clones, source reads or model hierarchy expansion.
    pub fn try_sync<T, E>(
        &mut self,
        view: &SpatialView<'_>,
        accept: impl FnOnce(&SceneInput, &str, UpdateKind) -> Result<T, E>,
    ) -> Result<T, SyncError<E>> {
        if &self.identity != view.identity() {
            return Err(NativeRenderError::DifferentWorld.into());
        }
        if self.topology_generation != view.topology_generation()
            || self.binding_generation != view.binding_generation()
        {
            let candidate = binding::prepare(view, &self.assets, self.camera)?;
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
            self.topology_generation = view.topology_generation();
            self.binding_generation = view.binding_generation();
            self.tick = view.tick();
            return Ok(result);
        }
        if view.nodes().len() != self.slots.len() || self.undo.len() != self.input.nodes.len() {
            return Err(NativeRenderError::Layout.into());
        }
        // Save the entire old numeric set before arming the guard. No user code
        // runs during these copies. Configuration/index layout is private and
        // unchanged for this world identity + accepted generation pair.
        for (old, node) in self.undo.iter_mut().zip(&self.input.nodes) {
            *old = node.world;
        }
        let mut guard = MotionGuard {
            input: &mut self.input,
            undo: &self.undo,
            accepted: false,
        };
        for node in view.nodes() {
            if let Some(slot) = self.slots[node.index] {
                guard.input.nodes[slot].world = *node.global;
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
    undo: &'a [DMat4],
    accepted: bool,
}
impl Drop for MotionGuard<'_> {
    fn drop(&mut self) {
        if !self.accepted {
            for (node, old) in self.input.nodes.iter_mut().zip(self.undo) {
                node.world = *old;
            }
        }
    }
}
