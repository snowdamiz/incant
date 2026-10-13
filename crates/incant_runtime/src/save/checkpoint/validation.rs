use super::WorldCheckpointError as Error;
use crate::{
    NativeWorld, StableId,
    physics::{Binding, Spin},
    physics_2d::{Binding2D, Spin2D},
};

/// Join immutable declarations and owner membership. Numeric ECS inputs are
/// intentionally separate: scripts may have changed them since the native step.
pub(super) fn bindings(world: &NativeWorld) -> Result<(), Error> {
    if world.physics_failed {
        return Err(Error::Owner("failed world"));
    }
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
        if binding.body != spec.body
            || binding.collider != spec.collider
            || world.world.get::<Spin>(handle).is_some() != binding.body.is_some()
        {
            return Err(Error::Owner("spatial configuration"));
        }
        spatial += 1;
    }
    let mut planar = 0;
    for (id, body, collider, _) in world.physics_2d.checkpoint_configurations()? {
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
            || binding.collider.as_ref() != Some(collider)
            || world.world.get::<Spin2D>(handle).is_some() != binding.body.is_some()
        {
            return Err(Error::Owner("planar configuration"));
        }
        planar += 1;
    }
    let mut expected_spatial = 0;
    let mut expected_planar = 0;
    for &handle in world.entities.values() {
        expected_spatial += usize::from(world.world.get::<Binding>(handle).is_some());
        expected_planar += usize::from(world.world.get::<Binding2D>(handle).is_some());
    }
    if spatial != expected_spatial || planar != expected_planar {
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
