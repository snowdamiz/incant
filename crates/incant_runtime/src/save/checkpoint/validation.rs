use super::WorldCheckpointError as Error;
use crate::{
    NativeWorld, StableId,
    physics::{Binding, Spin},
    physics_2d::{Binding2D, Spin2D},
};

/// Join immutable declarations and owner membership. Numeric ECS inputs are
/// intentionally separate: scripts may have changed them since the native step.
pub(super) fn bindings(
    world: &NativeWorld,
    planar_work: &mut super::PlanarWork,
    spatial_work: &mut super::SpatialWork,
) -> Result<(), Error> {
    if world.physics_failed {
        return Err(Error::Owner("failed world"));
    }
    // Charge the enclosing walks too: this join precedes native checkpoint
    // capture, and the tile/compound declaration comparisons can be deep.
    planar_work
        .charge(world.entities.len() as u64)
        .map_err(|_| Error::Owner("planar binding work budget"))?;
    spatial_work
        .charge(world.entities.len() as u64)
        .map_err(|_| Error::Owner("spatial binding work budget"))?;
    let mut compound_parts = 0usize;
    let mut spatial = 0;
    for (scene, id, spec) in world.physics.checkpoint_configurations()? {
        if scene != world.scene_id() || !world.physics_used {
            return Err(Error::Owner("spatial scene or history"));
        }
        let handle = *world
            .entities
            .get(&id)
            .ok_or(Error::Owner("spatial entity"))?;
        let binding = world
            .world
            .get::<Binding>(handle)
            .ok_or(Error::Owner("spatial binding"))?;
        if let incant_types::ColliderShape::Compound { parts } = &spec.collider.shape {
            compound_parts = compound_parts
                .checked_add(parts.len())
                .filter(|n| *n <= incant_physics_snapshot::MAX_ITEMS)
                .ok_or(Error::Owner("compound binding part limit"))?;
            if !(1..=64).contains(&parts.len()) || parts.iter().any(|p| p.id.len() != 26) {
                return Err(Error::Owner("compound binding identity limit"));
            }
            spatial_work
                .charge(parts.len() as u64 * 32)
                .map_err(|_| Error::Owner("spatial binding work budget"))?;
        }
        if binding.body != spec.body
            || binding.collider != spec.collider
            || world.world.get::<Spin>(handle).is_some() != binding.body.is_some()
        {
            return Err(Error::Owner("spatial configuration"));
        }
        spatial += 1;
    }
    let mut planar = 0;
    let mut tile_owners = 0;
    let mut shapes = 0usize;
    for row in world.physics_2d.checkpoint_geometry_configurations()? {
        let (id, body, collider) = (row.id, row.body, row.collider);
        if !world.physics_2d_used {
            return Err(Error::Owner("planar history"));
        }
        let handle = *world
            .entities
            .get(&id)
            .ok_or(Error::Owner("planar entity"))?;
        let binding = world
            .world
            .get::<Binding2D>(handle)
            .ok_or(Error::Owner("planar binding"))?;
        if &binding.body != body
            || world.world.get::<Spin2D>(handle).is_some() != binding.body.is_some()
        {
            return Err(Error::Owner("planar configuration"));
        }
        match row.geometry {
            incant_physics_2d_runtime::CheckpointGeometry::Primitive(shape) => {
                shapes = shapes
                    .checked_add(1)
                    .ok_or(Error::Owner("planar shape limit"))?;
                if binding.collider.as_ref() != Some(collider)
                    || shape != &collider.shape
                    || world.tilemap_collision_ids.contains(&id)
                {
                    return Err(Error::Owner("planar primitive configuration"));
                }
            }
            incant_physics_2d_runtime::CheckpointGeometry::Tiles(parts) => {
                shapes = shapes
                    .checked_add(parts.len())
                    .filter(|n| *n <= incant_physics_2d_runtime::MAX_SHAPES)
                    .ok_or(Error::Owner("planar shape limit"))?;
                planar_work
                    .charge(parts.len() as u64 * 4)
                    .map_err(|_| Error::Owner("planar binding work budget"))?;
                if body.is_some()
                    || binding.collider.is_some()
                    || collider != &incant_types::Collider2D::default()
                    || !world.tilemap_collision_ids.contains(&id)
                {
                    return Err(Error::Owner("tile binding configuration"));
                }
                let map = world
                    .world
                    .get::<crate::tilemap::Prepared>(handle)
                    .ok_or(Error::Owner("tile prepared map"))?;
                if parts.is_empty() || parts.len() != map.0.collision().len() {
                    return Err(Error::Owner("tile rectangle membership"));
                }
                let cell = map.0.cell_size();
                for ((center, half), rect) in parts.iter().zip(map.0.collision()) {
                    let expected_half = std::array::from_fn::<_, 2, _>(|axis| {
                        f64::from(rect.size[axis]) * cell[axis] / 2.
                    });
                    let expected_center = std::array::from_fn::<_, 2, _>(|axis| {
                        f64::from(rect.origin[axis]) * cell[axis] + expected_half[axis]
                    });
                    if half.map(f64::to_bits) != expected_half.map(f64::to_bits)
                        || center.map(f64::to_bits) != expected_center.map(f64::to_bits)
                    {
                        return Err(Error::Owner("tile rectangle declaration"));
                    }
                }
                tile_owners += 1;
            }
        }
        if shapes > incant_physics_2d_runtime::MAX_SHAPES {
            return Err(Error::Owner("planar shape limit"));
        }
        planar += 1;
    }
    let mut expected_spatial = 0;
    let mut expected_planar = 0;
    for &handle in world.entities.values() {
        expected_spatial += usize::from(world.world.get::<Binding>(handle).is_some());
        expected_planar += usize::from(world.world.get::<Binding2D>(handle).is_some());
    }
    if spatial != expected_spatial
        || planar != expected_planar
        || tile_owners != world.tilemap_collision_ids.len()
    {
        return Err(Error::Owner("native binding membership"));
    }
    Ok(())
}

pub(super) fn restore_planar_cache(
    world: &mut NativeWorld,
    rows: &[(StableId, [f64; 4])],
) -> Result<(), Error> {
    if !rows.windows(2).all(|w| w[0].0 < w[1].0) || rows.len() != world.physics_2d.states().count()
    {
        return Err(Error::Owner("planar rotation membership"));
    }
    for &(id, rotation) in rows {
        let handle = *world
            .entities
            .get(&id)
            .ok_or(Error::Owner("planar rotation entity"))?;
        let mut binding = world
            .world
            .get_mut::<Binding2D>(handle)
            .ok_or(Error::Owner("planar rotation binding"))?;
        let state = world
            .physics_2d
            .state(id)
            .ok_or(Error::Owner("planar rotation owner"))?;
        let expected = crate::physics_2d::rotation(state.rotation);
        if !rotation
            .iter()
            .zip(expected)
            .all(|(a, b)| a.to_bits() == b.to_bits())
        {
            return Err(Error::Owner("planar published rotation"));
        }
        binding.rotation = rotation;
    }
    Ok(())
}
