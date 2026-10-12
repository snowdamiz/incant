//! Caught SDK failures and late host failures discard every staged effect.
#![no_std]
use incant_module::{
    AnimationControl as A, AnimationGraphControl as G, AnimationGraphKey as Key, EntityId,
    IDENTITY, LogLevel, Result, Tick,
};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let entity = EntityId::from_u128(1);
    tick.control_animation_graph(
        &entity,
        G::SetParameter {
            parameter: Key::from_u128(10),
            value: 0.75,
        },
    )?;
    tick.control_animation(&entity, A::Playing(false))?;
    tick.spawn(&EntityId::from_u128(77), None, &IDENTITY, None)?;
    let chunk = tick.chunk(0)?;
    let mut scratch = [IDENTITY; 128];
    let rows = tick.read_transforms(chunk, &mut scratch)?;
    rows[0][0] = 42.0;
    tick.write_transforms(chunk, rows)?;
    tick.log("must roll back", LogLevel::Warn)?;
    tick.set_animation_ik(
        entity,
        EntityId::from_u128(9),
        [9., 1., 0.],
        [0., 1., 0.],
        0.25,
    )?;
    // Populate the complete-frame cache first. Old imports must still refuse.
    tick.animation_ik(&mut [0; 4096])?;
    if cfg!(ik_capacity) {
        let _ = tick.animation_ik(&mut []);
    }
    if cfg!(ik_legacy) {
        let _ = tick.legacy_animations(&mut [0; 4096]);
    }
    if cfg!(ik_v2) {
        let _ = tick.transition_animations(&mut [0; 4096]);
    }
    if cfg!(ik_v3) {
        let _ = tick.animations(&mut [0; 4096]);
    }
    if cfg!(ik_v4) {
        let _ = tick.animation_poses(&mut [0; 4096]);
    }
    if cfg!(ik_number) {
        let _ = tick.set_animation_ik(
            entity,
            EntityId::from_u128(9),
            [f64::NAN, 1., 0.],
            [0., 1., 0.],
            1.,
        );
    }
    if cfg!(ik_coordinate) {
        let _ = tick.set_animation_ik(
            entity,
            EntityId::from_u128(9),
            [1e9 + 1., 1., 0.],
            [0., 1., 0.],
            1.,
        );
    }
    if cfg!(ik_weight) {
        let _ = tick.set_animation_ik(
            entity,
            EntityId::from_u128(9),
            [1., 1., 0.],
            [0., 1., 0.],
            1.5,
        );
    }
    if cfg!(ik_constraint) {
        tick.set_animation_ik(
            entity,
            EntityId::from_u128(99),
            [1., 1., 0.],
            [0., 1., 0.],
            1.,
        )?;
    }
    if cfg!(ik_owner) {
        tick.set_animation_ik(
            EntityId::from_u128(3),
            EntityId::from_u128(9),
            [1., 1., 0.],
            [0., 1., 0.],
            1.,
        )?;
    }
    if cfg!(ik_quota) {
        for _ in 0..4097 {
            let _ = tick.set_animation_ik(
                entity,
                EntityId::from_u128(9),
                [1., 1., 0.],
                [0., 1., 0.],
                1.,
            );
        }
    }
    if cfg!(ik_late) {
        tick.set_velocity(&EntityId::from_u128(999), Some(&[1.0, 0.0, 0.0]))?;
    }
    if cfg!(ik_callback) {
        return Err(incant_module::Error::Callback);
    }
    if cfg!(ik_trap) {
        core::arch::wasm32::unreachable();
    }
    Ok(())
}
