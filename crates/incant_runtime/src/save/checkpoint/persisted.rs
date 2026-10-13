//! World v12 wraps unchanged component bytes and two typed native-owner sections.
//! Hashes detect corruption, not authority. Trusted runtime catalogs are rebound
//! from the receiving world; no pointer, callback, code or pool is serialized.
use super::*;
use sha2::{Digest, Sha256};
const MAGIC: &[u8; 8] = b"INCANTSV";
const HEADER: usize = 96;
const CACHE_ROW: usize = 48;
const PHYSICS_BYTES: usize = 32 * 1024 * 1024;
/// Existing ECS/component bound plus one aggregate physics envelope. Physics
/// budgets cover physics-owned payload/scratch only, not ECS bytes or RSS.
pub const MAX_PERSISTED_WORLD_BYTES: usize = codec::MAX_SAVE_BYTES + PHYSICS_BYTES;
fn bad(reason: &'static str) -> WorldCheckpointError {
    WorldCheckpointError::Owner(reason)
}
fn checked_sum(parts: &[usize]) -> Result<usize, WorldCheckpointError> {
    parts.iter().try_fold(0usize, |sum, &part| {
        sum.checked_add(part).ok_or(bad("world section overflow"))
    })
}
fn checksum(bytes: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(&bytes[..32]);
    digest.update(&bytes[64..]);
    digest.finalize().into()
}
fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}
struct Sections<'a> {
    components: &'a [u8],
    spatial: &'a [u8],
    planar: &'a [u8],
    cache: &'a [u8],
    flags: u32,
}
fn sections(bytes: &[u8]) -> Result<Sections<'_>, WorldCheckpointError> {
    if !(HEADER..=MAX_PERSISTED_WORLD_BYTES).contains(&bytes.len()) || &bytes[..8] != MAGIC {
        return Err(bad("world v12 framing"));
    }
    if u32::from_le_bytes(bytes[8..12].try_into().unwrap()) != 12 {
        return Err(bad("world v12 version"));
    }
    let flags = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    if flags & !127 != 0 || bytes[92..96] != [0; 4] {
        return Err(bad("world v12 flags"));
    }
    if u64_at(bytes, 24) != (bytes.len() - HEADER) as u64 || checksum(bytes) != bytes[32..64] {
        return Err(bad("world v12 integrity"));
    }
    let size = |at| usize::try_from(u64_at(bytes, at)).map_err(|_| bad("world section length"));
    let components = size(64)?;
    let spatial = size(72)?;
    let planar = size(80)?;
    let count = u32::from_le_bytes(bytes[88..92].try_into().unwrap()) as usize;
    if components > codec::MAX_SAVE_BYTES || count > incant_physics_2d_runtime::MAX_BODIES {
        return Err(bad("world section capacity"));
    }
    let cache = count
        .checked_mul(CACHE_ROW)
        .ok_or(bad("planar cache length"))?;
    if checked_sum(&[HEADER, spatial, planar, cache])? > PHYSICS_BYTES
        || checked_sum(&[HEADER, components, spatial, planar, cache])? != bytes.len()
    {
        return Err(bad("aggregate physics framing limit"));
    }
    let stepped = flags & 3 != 0;
    if (flags & 32 != 0) != (spatial != 0)
        || (flags & 64 != 0) != (planar != 0)
        || stepped != (spatial != 0 && planar != 0)
        || (!stepped && (spatial != 0 || planar != 0 || cache != 0))
    {
        return Err(bad("missing or extra persisted native owner"));
    }
    let (components, rest) = bytes[HEADER..].split_at(components);
    let decoded = codec::decode(components)?;
    if decoded.version < 3 || decoded.tick != u64_at(bytes, 16) {
        return Err(bad("nested component clock/binding"));
    }
    let (spatial, rest) = rest.split_at(spatial);
    let (planar, cache) = rest.split_at(planar);
    Ok(Sections {
        components,
        spatial,
        planar,
        cache,
        flags,
    })
}
impl WorldCheckpoint {
    /// Exact supported native file-domain encoding. This does not enable the
    /// conventional save API or omit session/VM/input/audio/UI owners.
    pub fn persisted_bytes(
        &self,
        payload: &mut Budget,
        planar_work: &mut PlanarWork,
        spatial_work: &mut SpatialWork,
    ) -> Result<Vec<u8>, WorldCheckpointError> {
        let decoded = codec::decode(&self.components)?;
        if decoded.version < 3 {
            return Err(bad("unbound component checkpoint"));
        }
        let count = self.planar_rotations.len();
        if count > incant_physics_2d_runtime::MAX_BODIES {
            return Err(bad("planar cache count"));
        }
        planar_work
            .charge(count as u64)
            .map_err(|_| bad("planar cache work"))?;
        if !self.planar_rotations.windows(2).all(|w| w[0].0 < w[1].0)
            || self
                .planar_rotations
                .iter()
                .any(|(_, r)| r.iter().any(|v| !v.is_finite()))
        {
            return Err(bad("planar cache order/value"));
        }
        let spatial = self
            .spatial
            .as_ref()
            .map(|p| p.persisted_bytes(payload, spatial_work))
            .transpose()?;
        let planar = self
            .planar
            .as_ref()
            .map(|p| p.persisted_bytes(payload, planar_work))
            .transpose()?;
        let spatial = spatial.as_deref().unwrap_or(&[]);
        let planar = planar.as_deref().unwrap_or(&[]);
        let physics = checked_sum(&[HEADER, spatial.len(), planar.len(), count * CACHE_ROW])?;
        if physics > PHYSICS_BYTES {
            return Err(bad("aggregate physics framing limit"));
        }
        let length = checked_sum(&[physics, self.components.len()])?;
        let flags = u32::from(self.physics_used)
            | (u32::from(self.physics_2d_used) << 1)
            | (u32::from(self.physics_save_required) << 2)
            | (u32::from(self.spatial_used) << 3)
            | (u32::from(self.navigation_used) << 4)
            | (u32::from(!spatial.is_empty()) << 5)
            | (u32::from(!planar.is_empty()) << 6);
        // The output ECS section has the existing separate component-byte bound.
        // Charge the physics share before reserving the combined fallible buffer.
        payload
            .charge(physics)
            .map_err(|_| bad("world physics output budget"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| bad("world output allocation"))?;
        bytes.extend(MAGIC);
        bytes.extend(12u32.to_le_bytes());
        bytes.extend(flags.to_le_bytes());
        bytes.extend(decoded.tick.to_le_bytes());
        bytes.extend(((length - HEADER) as u64).to_le_bytes());
        bytes.extend([0; 32]);
        for size in [self.components.len(), spatial.len(), planar.len()] {
            bytes.extend((size as u64).to_le_bytes());
        }
        bytes.extend((count as u32).to_le_bytes());
        bytes.extend([0; 4]);
        bytes.extend(&self.components);
        bytes.extend(spatial);
        bytes.extend(planar);
        for (id, rotation) in &self.planar_rotations {
            bytes.extend(id.0);
            for value in rotation {
                bytes.extend(value.to_le_bytes());
            }
        }
        let digest = checksum(&bytes);
        bytes[32..64].copy_from_slice(&digest);
        sections(&bytes)?;
        Ok(bytes)
    }
}
impl NativeWorld {
    /// Stage a world-v12 candidate against the receiving world's trusted cooked
    /// binding/catalogs. No changes to `current`, including on late join refusal.
    /// The caller must stage its other session owners before atomic publication.
    pub fn from_persisted_bytes_for(
        current: &Self,
        bytes: &[u8],
        payload: &mut Budget,
        planar_work: &mut PlanarWork,
        spatial_work: &mut SpatialWork,
        allocations: &mut AllocationBudget,
    ) -> Result<Self, WorldCheckpointError> {
        if current.physics_failed {
            return Err(bad("failed receiving world"));
        }
        let sections = sections(bytes)?;
        let decoded = codec::decode(sections.components)?;
        let scene = CookedScene::from_bytes(decoded.scene).map_err(SaveError::from)?;
        let stepped = sections.flags & 3 != 0;
        if !stepped
            && (!scene.physics().is_empty()
                || !scene.physics_2d().is_empty()
                || scene
                    .spatial()
                    .iter()
                    .any(|n| n.tilemap.as_ref().is_some_and(|m| m.has_collision())))
        {
            return Err(bad("missing native owner"));
        }
        let spatial = if stepped {
            incant_physics_runtime::PhysicsRuntime::from_persisted_bytes(
                sections.spatial,
                payload,
                spatial_work,
                allocations,
            )?
        } else {
            Default::default()
        };
        let planar = if stepped {
            incant_physics_2d_runtime::PhysicsWorld2D::from_persisted_bytes(
                sections.planar,
                payload,
                planar_work,
                allocations,
            )?
        } else {
            Default::default()
        };
        let mut candidate = Self::from_decoded_components(decoded, scene, Some((spatial, planar)))?;
        candidate.physics_used = sections.flags & 1 != 0;
        candidate.physics_2d_used = sections.flags & 2 != 0;
        candidate.physics_save_required = sections.flags & 4 != 0;
        candidate.spatial_used |= sections.flags & 8 != 0;
        candidate.navigation_used |= sections.flags & 16 != 0;
        let count = sections.cache.len() / CACHE_ROW;
        planar_work
            .charge(count as u64)
            .map_err(|_| bad("planar cache work"))?;
        let mut cache = payload
            .vector(count)
            .map_err(|_| bad("planar cache budget"))?;
        for row in sections.cache.as_chunks::<CACHE_ROW>().0 {
            let id = StableId(row[..16].try_into().unwrap());
            let rotation = std::array::from_fn(|i| {
                f64::from_le_bytes(row[16 + i * 8..24 + i * 8].try_into().unwrap())
            });
            cache.push((id, rotation));
        }
        validation::restore_planar_cache(&mut candidate, &cache)?;
        validation::bindings(&candidate, planar_work, spatial_work)?;
        candidate.check_save_binding(
            Some(current.source_scene_fingerprint),
            current.scene_id(),
            current.tick_rate,
            &current.localization,
            &current.font_assets,
        )?;
        candidate.validate_ik_catalog(&current.ik_catalog)?;
        candidate.validate_root_catalog(&current.root_catalog)?;
        candidate.ik_catalog = Arc::clone(&current.ik_catalog);
        candidate.root_catalog = Arc::clone(&current.root_catalog);
        candidate.physics_save_required |=
            current.physics_save_required || current.physics_used || current.physics_2d_used;
        candidate.spatial_used |= current.spatial_used;
        candidate.navigation_used |= current.navigation_used;
        candidate.ui_used |= current.ui_used;
        candidate.audio_used |= current.audio_used;
        Ok(candidate)
    }
}
