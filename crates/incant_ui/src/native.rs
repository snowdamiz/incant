//! Data-only UI checkpoint authority bound to one exact native world. No project
//! state, font discovery, drawing or guest initialization occurs here.
use crate::{TextMeasure, UiCheckpointError, UiError, UiPending, UiRuntime, Widget};
use incant_runtime::{NativeWorld, UiView, WorldIdentity};
use std::sync::Arc;

type Result<T> = std::result::Result<T, UiCheckpointError>;
#[derive(Clone)]
pub(crate) struct Binding {
    identity: WorldIdentity,
    generation: u64,
    tick: u64,
}
impl Binding {
    fn new(view: &UiView<'_>, tick: u64) -> Self {
        Self {
            identity: view.identity().clone(),
            generation: view.generation(),
            tick,
        }
    }
    fn validate(&self, view: &UiView<'_>, tick: u64) -> Result<()> {
        if &self.identity != view.identity()
            || self.generation != view.generation()
            || self.tick != tick
        {
            return Err(UiCheckpointError::Invalid("native UI world or generation"));
        }
        Ok(())
    }
}
/// Accepted widget data captured from its actual native owner. Private fields
/// prevent unrelated binary attachments or a caller-selected capability flag.
pub struct NativeUiCheckpoint {
    binding: Binding,
    bytes: Vec<u8>,
}
impl NativeUiCheckpoint {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn validate_world(&self, world: &NativeWorld) -> Result<()> {
        self.binding.validate(&view(world)?, world.tick())
    }
}
/// Fully bounded widget records paired to the decoded native world, before
/// layout/font preparation or a render consumer can run. Focus visibility is
/// checked while restoring the real layout, before the host may publish itself.
pub struct PendingUiCheckpoint {
    binding: Binding,
    bytes: Vec<u8>,
}
impl PendingUiCheckpoint {
    pub fn new(world: &NativeWorld, bytes: &[u8]) -> Result<Self> {
        let view = view(world)?;
        // Decode and validate all text/selection/membership before owning a copy.
        UiRuntime::checkpoint_state(widgets(&view), bytes)?;
        Ok(Self {
            binding: Binding::new(&view, world.tick()),
            bytes: bytes.to_vec(),
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn restore(
        &self,
        world: &NativeWorld,
        viewport: [f32; 2],
        measure: &mut impl TextMeasure,
    ) -> Result<UiRuntime> {
        let view = view(world)?;
        self.binding.validate(&view, world.tick())?;
        let mut runtime =
            UiRuntime::from_checkpoint_bytes(widgets(&view), &self.bytes, viewport, measure)?;
        runtime.native = Some(self.binding.clone());
        Ok(runtime)
    }
    pub fn validate_restored(&self, world: &NativeWorld, runtime: &UiRuntime) -> Result<()> {
        self.binding.validate(&view(world)?, world.tick())?;
        let restored = runtime.checkpoint_for_world(world)?;
        if restored.bytes != self.bytes {
            return Err(UiCheckpointError::Invalid("restored UI state differs"));
        }
        Ok(())
    }
}
impl UiRuntime {
    pub fn for_world(world: &NativeWorld) -> Result<Self> {
        let view = view(world)?;
        let mut runtime = Self::new(widgets(&view))?;
        runtime.native = Some(Binding::new(&view, world.tick()));
        Ok(runtime)
    }
    /// Publish an already prepared candidate for the same world. Unchanged
    /// generations retain the normal constant-work commit; structural changes
    /// validate the candidate's exact declaration/incarnation pairing first.
    pub fn commit_for_world(&mut self, pending: UiPending, world: &NativeWorld) -> Result<()> {
        let view = view(world)?;
        let binding = self
            .native
            .as_ref()
            .ok_or(UiCheckpointError::Invalid("unbound UI owner"))?;
        if &binding.identity != view.identity() {
            return Err(UiCheckpointError::Invalid("native UI world"));
        }
        if binding.generation != view.generation() || pending.replacement.is_some() {
            matches(pending.prepared(self)?.0, &view)?;
        }
        self.commit(pending)?;
        self.native = Some(Binding::new(&view, world.tick()));
        Ok(())
    }
    pub fn checkpoint_for_world(&self, world: &NativeWorld) -> Result<NativeUiCheckpoint> {
        let view = view(world)?;
        let binding = self
            .native
            .as_ref()
            .ok_or(UiCheckpointError::Invalid("unbound UI owner"))?;
        binding.validate(&view, world.tick())?;
        matches(self.widgets(), &view)?;
        Ok(NativeUiCheckpoint {
            binding: binding.clone(),
            bytes: self.checkpoint_bytes()?,
        })
    }
}
fn view(world: &NativeWorld) -> Result<UiView<'_>> {
    world
        .ui_view()
        .map_err(|_| UiError::Invalid("unusable native world").into())
}
fn widgets(view: &UiView<'_>) -> Vec<Widget> {
    view.nodes()
        .map(|node| Widget {
            id: node.id,
            incarnation: node.incarnation,
            parent: node.parent,
            node: Arc::clone(node.node),
        })
        .collect()
}
fn matches(widgets: &[Widget], view: &UiView<'_>) -> Result<()> {
    if widgets.len() != view.nodes().len() {
        return Err(UiCheckpointError::Invalid("native UI membership"));
    }
    // Both independently validated native/UI trees have unique IDs. Storage
    // order is not authority; checkpoints are rare and this bounded scan also
    // accepts a canonical host that preserves layout order in its widget vector.
    for node in view.nodes() {
        if !widgets.iter().any(|w| {
            w.id == node.id
                && w.incarnation == node.incarnation
                && w.parent == node.parent
                && w.node == *node.node
        }) {
            return Err(UiCheckpointError::Invalid("native UI widget binding"));
        }
    }
    Ok(())
}
