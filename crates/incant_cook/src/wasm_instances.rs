//! Cold typed data only. Never called by a gameplay tick or a guest.
use crate::ProjectScriptError as Error;
use incant_doc::{Project, SceneProgram};
use incant_runtime::CookedScene;
use incant_types::properties::PropertyBudget;
use incant_wasm_abi as abi;
use sha2::{Digest, Sha256};

/// Empty legacy catalogs keep the original program/checkpoint identity. The
/// returned binary binds canonical property bytes to the exact cooked scene.
pub fn prepare_wasm_instances(
    project: &Project,
    scene_id: &str,
    scene: &CookedScene,
) -> Result<Option<Vec<u8>>, Error> {
    project.validate().map_err(crate::CookError::from)?;
    let authored = project
        .scenes
        .get(scene_id)
        .ok_or(Error::Invalid("Wasm scene absent"))?;
    let Some(SceneProgram::Wasm { instances, .. }) = &authored.program else {
        return Ok(None);
    };
    if instances.is_empty() {
        return Ok(None);
    }
    if instances.len() > abi::INSTANCE_MAX_ROWS {
        return Err(Error::Invalid("Wasm instance count"));
    }
    if scene.id() != crate::stable_id(scene_id)? {
        return Err(Error::Invalid("Wasm scene pairing"));
    }
    let mut frame = abi::INSTANCE_MAGIC.to_vec();
    frame.extend(1u32.to_le_bytes());
    frame.extend([0; 12]);
    let mut budget = PropertyBudget::default();
    for (owner, props) in instances {
        let mut bytes = Vec::new();
        budget
            .object(props, 0, &mut |value| bytes.extend_from_slice(value))
            .map_err(|_| Error::Invalid("Wasm initial properties"))?;
        if frame.len() + 24 + bytes.len() > abi::INSTANCE_MAX_BYTES {
            return Err(Error::Invalid("Wasm instance frame size"));
        }
        frame.extend(crate::stable_id(owner)?.0);
        frame.extend((bytes.len() as u32).to_le_bytes());
        frame.extend([0; 4]);
        frame.extend(bytes);
    }
    frame[12..16].copy_from_slice(&(instances.len() as u32).to_le_bytes());
    let len = (frame.len() - 24) as u32;
    frame[16..20].copy_from_slice(&len.to_le_bytes());
    abi::instances::Snapshot::parse(&frame)
        .map_err(|_| Error::Invalid("Wasm instance wire contract"))?;
    let mut blob = Sha256::digest(scene.to_bytes()).to_vec();
    blob.extend(frame);
    Ok(Some(blob))
}
