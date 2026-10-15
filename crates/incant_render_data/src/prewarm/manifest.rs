use crate::material_keys::{MATERIAL_KEY_COUNT, MaterialKey, MaterialSamples};
use serde::{Deserialize, Serialize};

pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAX_MODEL_RECEIPTS: usize = 256;
#[derive(Debug, thiserror::Error)]
pub enum PrewarmError {
    #[error(transparent)]
    Scene(#[from] crate::SceneError),
    #[error("material manifest exceeds its byte bound")]
    Bytes,
    #[error("material manifest data: {0}")]
    Data(#[from] serde_json::Error),
    #[error("material manifest contract: {0}")]
    Contract(&'static str),
    #[error("material manifest does not match the loaded scene/catalog/profile/content")]
    Identity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialProgram {
    Wgsl,
    Cooked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialProfile {
    pub samples: MaterialSamples,
    pub program: MaterialProgram,
}
impl MaterialProfile {
    pub fn admits(self, key: MaterialKey) -> bool {
        self.samples == MaterialSamples::Four || key.samples() == MaterialSamples::One
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialIdentity {
    pub scene: [u8; 16],
    pub cooked_scene_sha256: [u8; 32],
    pub catalog_sha256: [u8; 32],
    pub profile: MaterialProfile,
    /// Sorted, distinct sealed complete cooked-content receipts at level load.
    /// Process-local AssetVersionId and recipe fingerprints are never persisted.
    pub initial_models: Vec<[u8; 32]>,
}
impl MaterialIdentity {
    /// Bind the compiled logical catalog to sealed actual content already loaded
    /// for this level. Called at cold admission, never from frame preparation.
    pub fn loaded(
        scene: [u8; 16],
        cooked_scene_sha256: [u8; 32],
        profile: MaterialProfile,
        input: &crate::SceneInput,
    ) -> Result<Self, PrewarmError> {
        let mut models = std::collections::BTreeSet::new();
        for binding in input.nodes.iter().filter_map(|node| node.mesh.as_ref()) {
            let receipt = binding
                .source
                .model_content_sha256()
                .ok_or(PrewarmError::Contract(
                    "model admission requires a sealed content receipt",
                ))?;
            models.insert(receipt);
            if models.len() > MAX_MODEL_RECEIPTS {
                return Err(PrewarmError::Contract("model receipt limit"));
            }
        }
        Ok(Self {
            scene,
            cooked_scene_sha256,
            profile,
            catalog_sha256: super::MATERIAL_CATALOG_SHA256,
            initial_models: models.into_iter().collect(),
        })
    }
    fn validate(&self) -> Result<(), PrewarmError> {
        validate_models(&self.initial_models)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingProvenance {
    pub script_sha256: [u8; 32],
    pub input_sha256: [u8; 32],
    pub first_tick: u64,
    pub last_tick: u64,
    pub tick_rate: u32,
    /// Distinct content versions encountered across accepted binding changes.
    pub observed_models: Vec<[u8; 32]>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub key: MaterialKey,
    pub first_tick: u64,
    pub last_tick: u64,
    /// Accepted observations containing this key, not primitive or GPU counts.
    pub ticks: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Scope {
    ModelMaterialV1,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialManifest {
    version: u32,
    scope: Scope,
    pub identity: MaterialIdentity,
    pub recording: RecordingProvenance,
    /// Only actually observed selections, never an inferred mirror superset.
    pub observed: Vec<Observation>,
}
/// Constructible only after bounded decoding and exact loaded-identity admission.
/// No caller can inject arbitrary descriptors through this value.
pub struct AdmittedMaterials {
    keys: [bool; MATERIAL_KEY_COUNT],
    profile: MaterialProfile,
}
impl AdmittedMaterials {
    pub fn keys(&self) -> impl Iterator<Item = MaterialKey> + '_ {
        self.keys
            .iter()
            .enumerate()
            .filter(|(_, present)| **present)
            .map(|(index, _)| MaterialKey::try_from(index as u8).expect("closed key array"))
    }
    pub fn profile(&self) -> MaterialProfile {
        self.profile
    }
}
fn validate_models(models: &[[u8; 32]]) -> Result<(), PrewarmError> {
    if models.len() > MAX_MODEL_RECEIPTS || models.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(PrewarmError::Contract(
            "model receipts must be bounded, sorted and distinct",
        ));
    }
    Ok(())
}
impl MaterialManifest {
    pub fn new(
        identity: MaterialIdentity,
        recording: RecordingProvenance,
        observed: Vec<Observation>,
    ) -> Result<Self, PrewarmError> {
        let value = Self {
            version: 1,
            scope: Scope::ModelMaterialV1,
            identity,
            recording,
            observed,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PrewarmError> {
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(PrewarmError::Bytes);
        }
        let value: Self = serde_json::from_slice(bytes)?;
        value.validate()?;
        Ok(value)
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, PrewarmError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)?;
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(PrewarmError::Bytes);
        }
        Ok(bytes)
    }
    pub fn admit(&self, expected: &MaterialIdentity) -> Result<AdmittedMaterials, PrewarmError> {
        self.validate()?;
        expected.validate()?;
        if &self.identity != expected {
            return Err(PrewarmError::Identity);
        }
        let mut keys = [false; MATERIAL_KEY_COUNT];
        for observation in &self.observed {
            keys[observation.key.index()] = true;
        }
        Ok(AdmittedMaterials {
            keys,
            profile: expected.profile,
        })
    }
    fn validate(&self) -> Result<(), PrewarmError> {
        if self.version != 1 || self.scope != Scope::ModelMaterialV1 {
            return Err(PrewarmError::Contract("unsupported version or family"));
        }
        self.identity.validate()?;
        let run = &self.recording;
        validate_models(&run.observed_models)?;
        if self
            .identity
            .initial_models
            .iter()
            .any(|digest| run.observed_models.binary_search(digest).is_err())
        {
            return Err(PrewarmError::Contract("missing initial model provenance"));
        }
        let observations = run
            .last_tick
            .checked_sub(run.first_tick)
            .and_then(|n| n.checked_add(1))
            .filter(|&n| n <= 36_001)
            .ok_or(PrewarmError::Contract("recording horizon"))?;
        if !(1..=1000).contains(&run.tick_rate) {
            return Err(PrewarmError::Contract("tick rate"));
        }
        if self.observed.len() > MATERIAL_KEY_COUNT
            || self
                .observed
                .windows(2)
                .any(|pair| pair[0].key >= pair[1].key)
        {
            return Err(PrewarmError::Contract("keys must be sorted and distinct"));
        }
        for item in &self.observed {
            if !self.identity.profile.admits(item.key)
                || item.first_tick < run.first_tick
                || item.last_tick > run.last_tick
                || item.first_tick > item.last_tick
                || item.ticks == 0
                || item.ticks > observations
                || item.ticks > item.last_tick - item.first_tick + 1
            {
                return Err(PrewarmError::Contract("observed key profile/tick bounds"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> MaterialManifest {
        MaterialManifest::new(
            MaterialIdentity {
                scene: [1; 16],
                cooked_scene_sha256: [2; 32],
                catalog_sha256: [3; 32],
                profile: MaterialProfile {
                    samples: MaterialSamples::One,
                    program: MaterialProgram::Wgsl,
                },
                initial_models: vec![[4; 32]],
            },
            RecordingProvenance {
                script_sha256: [5; 32],
                input_sha256: [6; 32],
                first_tick: 0,
                last_tick: 12,
                tick_rate: 60,
                observed_models: vec![[4; 32]],
            },
            vec![Observation {
                key: MaterialKey::try_from(1).unwrap(),
                first_tick: 4,
                last_tick: 9,
                ticks: 3,
            }],
        )
        .unwrap()
    }
    #[test]
    fn admission_preserves_only_observed_keys_and_refuses_every_stale_identity_field() {
        let value = fixture();
        let decoded = MaterialManifest::from_bytes(&value.to_bytes().unwrap()).unwrap();
        assert_eq!(
            decoded
                .admit(&value.identity)
                .unwrap()
                .keys()
                .map(u8::from)
                .collect::<Vec<_>>(),
            vec![1]
        );
        for change in 0..6 {
            let mut expected = value.identity.clone();
            match change {
                0 => expected.scene[0] ^= 1,
                1 => expected.cooked_scene_sha256[0] ^= 1,
                2 => expected.catalog_sha256[0] ^= 1,
                3 => expected.profile.samples = MaterialSamples::Four,
                4 => expected.profile.program = MaterialProgram::Cooked,
                _ => expected.initial_models[0][0] ^= 1,
            }
            assert!(matches!(
                decoded.admit(&expected),
                Err(PrewarmError::Identity)
            ));
        }
    }
    #[test]
    fn unknown_duplicate_oversize_or_inconsistent_records_refuse() {
        assert!(matches!(
            MaterialManifest::from_bytes(&vec![b' '; MAX_MANIFEST_BYTES + 1]),
            Err(PrewarmError::Bytes)
        ));
        for change in 0..8 {
            let mut value = serde_json::to_value(fixture()).unwrap();
            match change {
                0 => value["version"] = 2.into(),
                1 => value["unexpected"] = true.into(),
                2 => value["observed"][0]["key"] = 24.into(),
                3 => value["observed"][0]["key"] = 12.into(),
                4 => value["observed"][0]["ticks"] = 7.into(),
                5 => value["recording"]["last_tick"] = u64::MAX.into(),
                6 => value["recording"]["observed_models"] = serde_json::json!([]),
                _ => {
                    let duplicate = value["observed"][0].clone();
                    value["observed"].as_array_mut().unwrap().push(duplicate);
                }
            }
            assert!(
                MaterialManifest::from_bytes(&serde_json::to_vec(&value).unwrap()).is_err(),
                "case {change}"
            );
        }
    }
}
