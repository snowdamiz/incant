//! Explicit saves of the currently migrated native world, never the tick path.
//! The envelope and nested scene are versioned checked data, not executable
//! bytecode. Their hashes detect corruption, not origin or authorization.
use crate::world::{Local, Motion};
use crate::{CookedEntity, CookedScene, NativeWorld, SceneError};
use sha2::{Digest, Sha256};
use thiserror::Error;

const MAGIC: &[u8; 8] = b"INCANTSV";
const VERSION: u32 = 1;
const PREFIX: usize = 32;
const HEADER: usize = 64;
pub const MAX_SAVE_BYTES: usize = HEADER + crate::binary::MAX_SCENE_BYTES;

#[derive(Debug, Error, PartialEq)]
pub enum SaveError {
    #[error("invalid native world save: {0}")]
    Invalid(&'static str),
    #[error("unsupported native world save version {0}")]
    Version(u32),
    #[error("native world save integrity check failed")]
    Integrity,
    #[error(transparent)]
    Scene(#[from] SceneError),
    #[error("native world save belongs to a different scene or tick rate")]
    WorldMismatch,
}

impl NativeWorld {
    /// Save committed Transform/Velocity/hierarchy state and the fixed-step
    /// clock. Does not include a script VM, input, timers or pending external
    /// FrameCommands. Those systems must gain explicit formats when migrated;
    /// this is not a complete game-session save API. Call at a committed boundary.
    /// The schema versions/component masks come from the nested cooked scene.
    /// Invalid numeric state fails without changing the running world.
    pub fn save_bytes(&self) -> Result<Vec<u8>, SaveError> {
        let entities = self
            .entities
            .iter()
            .map(|(&id, &handle)| CookedEntity {
                id,
                parent: self.parents[&id],
                transform: self
                    .world
                    .get::<Local>(handle)
                    .expect("live entity")
                    .0
                    .clone(),
                velocity: self.world.get::<Motion>(handle).map(|v| v.0.clone()),
            })
            .collect();
        let scene = CookedScene::new(self.scene_id(), self.tick_rate, entities)?.to_bytes();
        let mut bytes = Vec::with_capacity(HEADER + scene.len());
        bytes.extend(MAGIC);
        bytes.extend(VERSION.to_le_bytes());
        bytes.extend(0_u32.to_le_bytes()); // Reserved flags: unknown systems fail closed.
        bytes.extend(self.tick.to_le_bytes());
        bytes.extend((scene.len() as u64).to_le_bytes());
        bytes.resize(HEADER, 0);
        bytes.extend(scene);
        let hash = checksum(&bytes);
        bytes[PREFIX..HEADER].copy_from_slice(&hash);
        Ok(bytes)
    }

    /// Load a self-contained native world after checking the entire envelope,
    /// nested scene schema, bounds, hierarchy, component values and integrity.
    /// A scene source/project/journal is neither read nor needed.
    pub fn from_save_bytes(bytes: &[u8]) -> Result<Self, SaveError> {
        if !(HEADER..=MAX_SAVE_BYTES).contains(&bytes.len()) {
            return Err(SaveError::Invalid("file size"));
        }
        if &bytes[..8] != MAGIC {
            return Err(SaveError::Invalid("magic"));
        }
        let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        if version != VERSION {
            return Err(SaveError::Version(version));
        }
        if bytes[12..16] != [0; 4] {
            return Err(SaveError::Invalid("unsupported flags"));
        }
        let tick = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let length = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        if length != (bytes.len() - HEADER) as u64 {
            return Err(SaveError::Invalid("payload size"));
        }
        if checksum(bytes) != bytes[PREFIX..HEADER] {
            return Err(SaveError::Integrity);
        }
        let scene = CookedScene::from_bytes(&bytes[HEADER..])?;
        let mut world = Self::new(scene);
        world.tick = tick;
        Ok(world)
    }

    /// Replace this world's committed state only after a full successful load.
    /// A malformed, unsupported or different-world save leaves it untouched.
    /// External script hosts and command buffers are not restored by this API.
    pub fn restore_bytes(&mut self, bytes: &[u8]) -> Result<(), SaveError> {
        let candidate = Self::from_save_bytes(bytes)?;
        if candidate.scene_id() != self.scene_id() || candidate.tick_rate != self.tick_rate {
            return Err(SaveError::WorldMismatch);
        }
        *self = candidate;
        Ok(())
    }
}

fn checksum(bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(&bytes[..PREFIX]);
    hash.update(&bytes[HEADER..]);
    hash.finalize().into()
}
