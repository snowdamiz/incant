//! Version 1, little-endian checked data (not executable code). SHA-256 detects
//! damage, not authorship. No casts, alignment assumptions or native-layout ABI.
use crate::scene::MAX_ENTITIES;
use crate::{CookedEntity, CookedScene, SceneError, StableId};
use incant_types::{Transform, Velocity};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const MAGIC: &[u8; 8] = b"INCANTSC";
const VERSION: u32 = 1;
const PREFIX: usize = 40;
const HEADER: usize = 72;
const MIN_RECORD: usize = 104;
const MAX_RECORD: usize = 128;

impl CookedScene {
    pub fn to_bytes(&self) -> Vec<u8> {
        let indices: BTreeMap<_, _> = self
            .entities
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id, i as u32))
            .collect();
        let mut bytes = Vec::with_capacity(HEADER + self.entities.len() * MAX_RECORD);
        bytes.extend(MAGIC);
        bytes.extend(VERSION.to_le_bytes());
        bytes.extend(self.tick_rate.to_le_bytes());
        bytes.extend(self.id.0);
        bytes.extend((self.entities.len() as u32).to_le_bytes());
        bytes.extend(0_u32.to_le_bytes());
        bytes.resize(HEADER, 0);
        for entity in &self.entities {
            bytes.extend(entity.id.0);
            bytes.extend(
                entity
                    .parent
                    .map_or(u32::MAX, |p| indices[&p])
                    .to_le_bytes(),
            );
            bytes.extend(
                (if entity.velocity.is_some() {
                    3_u32
                } else {
                    1_u32
                })
                .to_le_bytes(),
            );
            for value in entity
                .transform
                .translation
                .iter()
                .chain(&entity.transform.rotation)
                .chain(&entity.transform.scale)
            {
                bytes.extend(value.to_le_bytes());
            }
            if let Some(velocity) = &entity.velocity {
                for value in velocity.linear {
                    bytes.extend(value.to_le_bytes());
                }
            }
        }
        let length = (bytes.len() - HEADER) as u32;
        bytes[36..40].copy_from_slice(&length.to_le_bytes());
        let checksum = checksum(&bytes);
        bytes[PREFIX..HEADER].copy_from_slice(&checksum);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SceneError> {
        if bytes.len() < HEADER || bytes.len() > HEADER + MAX_ENTITIES * MAX_RECORD {
            return Err(SceneError::Invalid("file size"));
        }
        let mut header = Reader(&bytes[..PREFIX]);
        if &header.array::<8>()? != MAGIC {
            return Err(SceneError::Invalid("magic"));
        }
        let version = header.u32()?;
        if version != VERSION {
            return Err(SceneError::Version(version));
        }
        let tick_rate = header.u32()?;
        let id = StableId(header.array()?);
        let count = header.u32()? as usize;
        let size = header.u32()? as usize;
        if count > MAX_ENTITIES
            || size != bytes.len() - HEADER
            || size < count * MIN_RECORD
            || size > count * MAX_RECORD
        {
            return Err(SceneError::Invalid("record count or payload size"));
        }
        if checksum(bytes) != bytes[PREFIX..HEADER] {
            return Err(SceneError::Integrity);
        }
        let mut reader = Reader(&bytes[HEADER..]);
        let mut entities = Vec::with_capacity(count);
        let mut parents = Vec::with_capacity(count);
        let mut previous = None;
        for _ in 0..count {
            let id = StableId(reader.array()?);
            parents.push(reader.u32()?);
            let mask = reader.u32()?;
            if mask != 1 && mask != 3 {
                return Err(SceneError::Invalid("unsupported component mask"));
            }
            let key = (mask, id);
            if previous.is_some_and(|p| p >= key) {
                return Err(SceneError::Invalid("noncanonical archetype order"));
            }
            previous = Some(key);
            let transform = Transform {
                translation: reader.floats()?,
                rotation: reader.floats()?,
                scale: reader.floats()?,
            };
            let velocity = if mask == 3 {
                Some(Velocity {
                    linear: reader.floats()?,
                })
            } else {
                None
            };
            entities.push(CookedEntity {
                id,
                parent: None,
                transform,
                velocity,
            });
        }
        if !reader.0.is_empty() {
            return Err(SceneError::Invalid("trailing payload"));
        }
        for (i, parent) in parents.into_iter().enumerate() {
            if parent != u32::MAX {
                let id = entities
                    .get(parent as usize)
                    .ok_or(SceneError::Parent(entities[i].id))?
                    .id;
                entities[i].parent = Some(id);
            }
        }
        Self::new(id, tick_rate, entities)
    }
}

fn checksum(bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(&bytes[..PREFIX]);
    hash.update(&bytes[HEADER..]);
    hash.finalize().into()
}

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn array<const N: usize>(&mut self) -> Result<[u8; N], SceneError> {
        let part = self
            .0
            .get(..N)
            .ok_or(SceneError::Invalid("truncated record"))?;
        let mut array = [0; N];
        array.copy_from_slice(part);
        self.0 = &self.0[N..];
        Ok(array)
    }
    fn u32(&mut self) -> Result<u32, SceneError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn floats<const N: usize>(&mut self) -> Result<[f64; N], SceneError> {
        let mut values = [0.; N];
        for value in &mut values {
            *value = f64::from_le_bytes(self.array()?);
        }
        Ok(values)
    }
}
