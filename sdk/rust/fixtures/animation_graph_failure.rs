//! Ignored SDK errors and deferred graph membership validation are atomic.
#![no_std]
use incant_module::{AnimationControl as A, AnimationGraphControl as G, AnimationGraphKey as Key,
    EntityId, IDENTITY, LogLevel, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::module!(simulate);
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let entity = EntityId([1; 16]);
    let set = |value| G::SetParameter { parameter: Key([10; 16]), value };
    tick.control_animation_graph(&entity, set(0.75))?;
    tick.control_animation(&entity, A::Playing(false))?;
    tick.spawn(&EntityId::from_u128(77), None, &IDENTITY, None)?;
    let chunk = tick.chunk(0)?;
    let mut scratch = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut scratch)?;
    rows[0][0] = 42.; tick.write_transforms(chunk, rows)?;
    tick.log("must roll back", LogLevel::Warn)?;
    if cfg!(sdk_graph_capacity) { let _ = tick.animation_graphs(&mut []); }
    if cfg!(sdk_graph_pose_capacity) { let _ = tick.animations(&mut []); }
    if cfg!(sdk_graph_number) { let _ = tick.control_animation_graph(&entity, set(f64::NAN)); }
    if cfg!(sdk_graph_range) { let _ = tick.control_animation_graph(&entity, set(1_000_001.)); }
    if cfg!(sdk_graph_authored_range) { tick.control_animation_graph(&entity, set(1.1))?; }
    if cfg!(sdk_graph_parameter) { tick.control_animation_graph(&entity, G::SetParameter { parameter: Key([99; 16]), value: 0. })?; }
    if cfg!(sdk_graph_state) { tick.control_animation_graph(&entity, G::SelectState(Key([99; 16])))?; }
    if cfg!(sdk_graph_entity) { tick.control_animation_graph(&EntityId([99; 16]), set(0.5))?; }
    if cfg!(sdk_graph_quota) { for _ in 0..1024 { tick.control_animation_graph(&entity, set(0.5))?; } }
    if cfg!(sdk_graph_callback) { return Err(incant_module::Error::Callback); }
    if cfg!(sdk_graph_trap) { core::arch::wasm32::unreachable(); }
    Ok(())
}
