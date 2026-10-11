//! Versions 1/2/3/4/5/6/7/8, little-endian checked data (not executable code). SHA-256 detects
//! damage, not authorship. No casts, alignment assumptions or native-layout ABI.
use crate::scene::MAX_ENTITIES;
use crate::{CookedEntity, CookedScene, SceneError, StableId};
use incant_types::{Transform, Velocity};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const MAGIC: &[u8; 8] = b"INCANTSC";
const VERSION: u32 = 8;
const PREFIX: usize = 40;
const HEADER: usize = 72;
const MIN_RECORD: usize = 104;
const MAX_RECORD: usize = 128;
pub(crate) const MAX_SCENE_BYTES: usize = 128 * 1024 * 1024;

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
        let version = if self.spatial.iter().any(|n| n.flipbook.is_some()) {
            VERSION
        } else if !self.physics_2d.is_empty() {
            7
        } else if self.spatial.iter().any(|n| n.tilemap.is_some()) {
            6
        } else if self.spatial.iter().any(|n| n.sprite.is_some()) {
            5
        } else if !self.spatial.is_empty() || !self.visual_assets.is_empty() {
            4
        } else if !self.audio.is_empty() {
            3
        } else if !self.physics.is_empty() {
            2
        } else {
            1
        };
        bytes.extend(version.to_le_bytes());
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
        if version >= 2 {
            bytes.extend((self.physics.len() as u32).to_le_bytes());
            for declaration in &self.physics {
                crate::physics_binary::write(&mut bytes, declaration);
            }
        }
        if version >= 3 {
            bytes.extend((self.audio.len() as u32).to_le_bytes());
            for node in &self.audio {
                crate::audio_binary::write(&mut bytes, node);
            }
        }
        if version >= 4 {
            bytes.extend((self.visual_assets.len() as u32).to_le_bytes());
            for asset in &self.visual_assets {
                crate::spatial_binary::write_asset(&mut bytes, asset);
            }
            bytes.extend((self.spatial.len() as u32).to_le_bytes());
            for node in &self.spatial {
                crate::spatial_binary::write(&mut bytes, node);
            }
        }
        if version >= 7 {
            bytes.extend((self.physics_2d.len() as u32).to_le_bytes());
            for record in &self.physics_2d {
                crate::physics_2d_binary::write(&mut bytes, record);
            }
        }
        if version >= 8 {
            bytes.extend(
                (self.spatial.iter().filter(|n| n.flipbook.is_some()).count() as u32).to_le_bytes(),
            );
            for node in &self.spatial {
                if let Some(f) = &node.flipbook {
                    crate::flipbook_binary::write(&mut bytes, node.entity, f);
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
        if bytes.len() < HEADER || bytes.len() > MAX_SCENE_BYTES {
            return Err(SceneError::Invalid("file size"));
        }
        let mut header = Reader(&bytes[..PREFIX]);
        if &header.array::<8>()? != MAGIC {
            return Err(SceneError::Invalid("magic"));
        }
        let version = header.u32()?;
        if !(1..=VERSION).contains(&version) {
            return Err(SceneError::Version(version));
        }
        let tick_rate = header.u32()?;
        let id = StableId(header.array()?);
        let count = header.u32()? as usize;
        let size = header.u32()? as usize;
        if count > MAX_ENTITIES
            || size != bytes.len() - HEADER
            || size < count * MIN_RECORD
            || (version == 1 && size > count * MAX_RECORD)
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
        let mut physics = Vec::new();
        if version >= 2 {
            let physics_count = reader.u32()? as usize;
            if (version == 2 && physics_count == 0)
                || physics_count > count
                || physics_count > reader.0.len() / 52
            {
                return Err(SceneError::Invalid("physics record count"));
            }
            physics.reserve(physics_count);
            for _ in 0..physics_count {
                let record = crate::physics_binary::read(&mut reader)?;
                if physics
                    .last()
                    .is_some_and(|p: &crate::CookedPhysics| p.entity >= record.entity)
                {
                    return Err(SceneError::Invalid("noncanonical physics order"));
                }
                physics.push(record);
            }
        }
        let mut audio = Vec::new();
        if version >= 3 {
            let count = reader.u32()? as usize;
            if (version == 3 && count == 0)
                || count > crate::MAX_AUDIO_NODES
                || count > reader.0.len() / 17
            {
                return Err(SceneError::Invalid("audio record count"));
            }
            audio.reserve(count);
            for _ in 0..count {
                let node = crate::audio_binary::read(&mut reader)?;
                if audio
                    .last()
                    .is_some_and(|prior: &crate::CookedAudio| prior.entity >= node.entity)
                {
                    return Err(SceneError::Invalid("noncanonical audio order"));
                }
                audio.push(node);
            }
        }
        let mut assets = Vec::new();
        let mut spatial = Vec::new();
        if version >= 4 {
            let asset_count = reader.u32()? as usize;
            if asset_count > crate::MAX_VISUAL_ASSETS
                || asset_count > reader.0.len() / crate::spatial_binary::ASSET_BYTES
            {
                return Err(SceneError::Invalid("visual asset count"));
            }
            assets.reserve(asset_count);
            for _ in 0..asset_count {
                let asset = crate::spatial_binary::read_asset(&mut reader)?;
                if assets
                    .last()
                    .is_some_and(|p: &crate::VisualAssetRef| p.id >= asset.id)
                {
                    return Err(SceneError::Invalid("noncanonical visual asset order"));
                }
                assets.push(asset);
            }
            let spatial_count = reader.u32()? as usize;
            if spatial_count > count
                || spatial_count > reader.0.len() / 17
                || (version < 7 && spatial_count == 0 && asset_count == 0)
            {
                return Err(SceneError::Invalid("spatial record count"));
            }
            spatial.reserve(spatial_count);
            let mut tile_budget = crate::tilemap_binary::Budget::default();
            for _ in 0..spatial_count {
                let node = crate::spatial_binary::read(&mut reader, version, &mut tile_budget)?;
                if spatial
                    .last()
                    .is_some_and(|p: &crate::CookedSpatial| p.entity >= node.entity)
                {
                    return Err(SceneError::Invalid("noncanonical spatial order"));
                }
                spatial.push(node);
            }
        }
        if version == 5 && !spatial.iter().any(|n| n.sprite.is_some()) {
            return Err(SceneError::Invalid(
                "version 5 requires a sprite declaration",
            ));
        }
        if version == 6 && !spatial.iter().any(|n| n.tilemap.is_some()) {
            return Err(SceneError::Invalid(
                "version 6 requires a tilemap declaration",
            ));
        }
        let mut physics_2d = Vec::new();
        if version >= 7 {
            let n = reader.u32()? as usize;
            if (version == 7 && n == 0)
                || n > count
                || n > incant_physics_2d_runtime::MAX_BODIES
                || n > reader.remaining_len() / 60
            {
                return Err(SceneError::Invalid("2D record count"));
            }
            physics_2d.reserve(n);
            for _ in 0..n {
                let record = crate::physics_2d_binary::read(&mut reader)?;
                if physics_2d
                    .last()
                    .is_some_and(|p: &crate::CookedPhysics2D| p.entity >= record.entity)
                {
                    return Err(SceneError::Invalid("noncanonical 2D order"));
                }
                physics_2d.push(record);
            }
        }
        if version >= 8 {
            let n = reader.u32()? as usize;
            if n == 0 || n > spatial.len() || n > reader.remaining_len() / 46 {
                return Err(SceneError::Invalid("flipbook record count"));
            }
            let mut previous = None;
            let mut budget = 0;
            for _ in 0..n {
                let id = StableId(reader.array()?);
                if previous.is_some_and(|p| p >= id) {
                    return Err(SceneError::Invalid("noncanonical flipbook order"));
                }
                previous = Some(id);
                let index = spatial
                    .binary_search_by_key(&id, |n| n.entity)
                    .map_err(|_| SceneError::Invalid("flipbook entity"))?;
                spatial[index].flipbook =
                    Some(crate::flipbook_binary::read(&mut reader, &mut budget)?);
            }
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
        Self::new(id, tick_rate, entities)?
            .with_physics(physics)?
            .with_audio(audio)?
            .with_spatial(assets, spatial)?
            .with_physics_2d(physics_2d)
    }
}

fn checksum(bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(&bytes[..PREFIX]);
    hash.update(&bytes[HEADER..]);
    hash.finalize().into()
}

pub(crate) struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    pub(crate) fn remaining_len(&self) -> usize {
        self.0.len()
    }
    pub(crate) fn byte(&mut self) -> Result<u8, SceneError> {
        Ok(self.array::<1>()?[0])
    }
    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], SceneError> {
        let part = self
            .0
            .get(..N)
            .ok_or(SceneError::Invalid("truncated record"))?;
        let mut array = [0; N];
        array.copy_from_slice(part);
        self.0 = &self.0[N..];
        Ok(array)
    }
    pub(crate) fn u32(&mut self) -> Result<u32, SceneError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    pub(crate) fn floats<const N: usize>(&mut self) -> Result<[f64; N], SceneError> {
        let mut values = [0.; N];
        for value in &mut values {
            *value = f64::from_le_bytes(self.array()?);
        }
        Ok(values)
    }
}
