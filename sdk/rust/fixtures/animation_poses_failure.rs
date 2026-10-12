//! Caught SDK failures and late host failures discard every staged effect.
#![no_std]
use incant_module::{AnimationControl as A, AnimationGraphControl as G, AnimationGraphKey as Key,
    EntityId, IDENTITY, LogLevel, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::module!(simulate);
fn simulate(tick:&mut Tick<'_>,_:f64)->Result<()> {
    let entity=EntityId::from_u128(1);
    tick.control_animation_graph(&entity,G::SetParameter {parameter:Key::from_u128(10),value:0.75})?;
    tick.control_animation(&entity,A::Playing(false))?;
    tick.spawn(&EntityId::from_u128(77),None,&IDENTITY,None)?;
    let chunk=tick.chunk(0)?;let mut scratch=[IDENTITY;64];
    let rows=tick.read_transforms(chunk,&mut scratch)?;
    rows[0][0]=42.0;tick.write_transforms(chunk,rows)?;
    tick.log("must roll back",LogLevel::Warn)?;
    if cfg!(pose_capacity) { let _=tick.animation_poses(&mut []); }
    if cfg!(pose_legacy) { let _=tick.legacy_animations(&mut [0;2048]); }
    if cfg!(pose_v2) { let _=tick.transition_animations(&mut [0;2048]); }
    if cfg!(pose_v3) { let _=tick.animations(&mut [0;2048]); }
    if cfg!(pose_number) { let _=tick.control_animation(&entity,A::Speed(f64::NAN)); }
    if cfg!(pose_parameter) { tick.control_animation_graph(&entity,G::SetParameter {parameter:Key::from_u128(99),value:0.5})?; }
    if cfg!(pose_late) { tick.set_velocity(&EntityId::from_u128(999),Some(&[1.0,0.0,0.0]))?; }
    if cfg!(pose_callback) { return Err(incant_module::Error::Callback); }
    if cfg!(pose_trap) { core::arch::wasm32::unreachable(); }
    Ok(())
}
