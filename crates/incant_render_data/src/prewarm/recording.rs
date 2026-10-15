use super::{
    MaterialIdentity, MaterialManifest, MaterialProfile, Observation, PrewarmError,
    RecordingProvenance, selection::Selection,
};
use crate::{
    SceneInput,
    material_keys::{MATERIAL_KEY_COUNT, MaterialKey},
};

/// Trusted host identities computed once at cold startup. Actual loaded model
/// receipts are derived by the observer, never supplied as caller assertions.
#[derive(Clone)]
pub struct RecordingContext {
    pub scene: [u8; 16],
    pub cooked_scene_sha256: [u8; 32],
    pub profile: MaterialProfile,
    pub script_sha256: [u8; 32],
    pub input_sha256: [u8; 32],
    pub tick_rate: u32,
}
pub struct MaterialRecorder {
    identity: MaterialIdentity,
    provenance: RecordingProvenance,
    observations: [Option<Observation>; MATERIAL_KEY_COUNT],
    selection: Selection,
    failed: bool,
}
impl MaterialRecorder {
    pub fn new(
        input: &SceneInput,
        camera: &str,
        tick: u64,
        context: RecordingContext,
    ) -> Result<Self, PrewarmError> {
        if !(1..=1000).contains(&context.tick_rate) {
            return Err(PrewarmError::Contract("tick rate"));
        }
        let selection = Selection::new(input, camera)?;
        let models = selection.receipts().to_vec();
        let mut value = Self {
            identity: MaterialIdentity {
                scene: context.scene,
                cooked_scene_sha256: context.cooked_scene_sha256,
                catalog_sha256: super::MATERIAL_CATALOG_SHA256,
                profile: context.profile,
                initial_models: models.clone(),
            },
            provenance: RecordingProvenance {
                script_sha256: context.script_sha256,
                input_sha256: context.input_sha256,
                tick_rate: context.tick_rate,
                first_tick: tick,
                last_tick: tick,
                observed_models: models,
            },
            observations: [None; MATERIAL_KEY_COUNT],
            selection,
            failed: false,
        };
        value.accept(input, camera, tick)?;
        Ok(value)
    }
    /// A failed observation is terminal: no partial candidate manifest can be
    /// exported. NativeRenderHost also destroys its composition after refusal.
    /// Repeated refreshes at one committed tick union keys without double counts.
    pub fn accept(
        &mut self,
        input: &SceneInput,
        camera: &str,
        tick: u64,
    ) -> Result<(), PrewarmError> {
        if self.failed {
            return Err(PrewarmError::Contract("recording is terminal"));
        }
        let result = self.observe(input, camera, tick);
        self.failed = result.is_err();
        result
    }
    fn observe(&mut self, input: &SceneInput, camera: &str, tick: u64) -> Result<(), PrewarmError> {
        if tick < self.provenance.last_tick || tick - self.provenance.first_tick > 36_000 {
            return Err(PrewarmError::Contract("recording clock/horizon"));
        }
        let keys = if self.selection.matches(input, camera) {
            self.selection.keys(input, self.identity.profile.samples)?
        } else {
            let mut next = Selection::new(input, camera)?;
            let keys = next.keys(input, self.identity.profile.samples)?;
            // Only binding changes copy bounded receipt metadata. Moving ticks
            // never hash content, read files, rebuild hierarchy or allocate it.
            let mut receipts = self.provenance.observed_models.clone();
            for receipt in next.receipts() {
                if let Err(index) = receipts.binary_search(receipt) {
                    if receipts.len() == super::MAX_MODEL_RECEIPTS {
                        return Err(PrewarmError::Contract("model receipt union limit"));
                    }
                    receipts.insert(index, *receipt);
                }
            }
            self.provenance.observed_models = receipts;
            self.selection = next;
            keys
        };
        for index in 0..MATERIAL_KEY_COUNT {
            if keys & (1 << index) == 0 {
                continue;
            }
            match &mut self.observations[index] {
                Some(item) => {
                    if item.last_tick != tick {
                        item.ticks += 1;
                    }
                    item.last_tick = tick;
                }
                item @ None => {
                    *item = Some(Observation {
                        key: MaterialKey::try_from(index as u8).expect("closed key array"),
                        first_tick: tick,
                        last_tick: tick,
                        ticks: 1,
                    })
                }
            }
        }
        self.provenance.last_tick = tick;
        Ok(())
    }
    /// Cold explicit export only. The returned data cannot create native code or
    /// arbitrary PSOs; a renderer still admits exact scene/catalog/profile bytes.
    pub fn manifest(&self) -> Result<MaterialManifest, PrewarmError> {
        if self.failed {
            return Err(PrewarmError::Contract("recording is terminal"));
        }
        MaterialManifest::new(
            self.identity.clone(),
            self.provenance.clone(),
            self.observations.iter().flatten().copied().collect(),
        )
    }
}
