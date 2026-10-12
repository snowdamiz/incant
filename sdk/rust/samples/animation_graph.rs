//! A graph's state/parameters and actual two-pose blend are copied observations.
#![no_std]
use incant_module::{AnimationControl as A, AnimationGraphControl as G, AnimationGraphKey as Key,
    EntityId, Error, IDENTITY, LogLevel, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::module!(simulate);
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let entity = EntityId([1; 16]);
    let mut graph_bytes = [0; 256];
    let mut pose_bytes = [0; 256];
    let graph = tick.animation_graphs(&mut graph_bytes)?.iter()
        .find(|g| g.entity == entity).ok_or(Error::Malformed)?;
    let pose = tick.animations(&mut pose_bytes)?.iter()
        .find(|p| p.entity == entity).ok_or(Error::Malformed)?;
    let parameter = graph.parameters().next().ok_or(Error::Malformed)?;
    if graph.state != Key([20; 16]) || parameter.id != Key([10; 16]) {
        return Err(Error::Malformed);
    }
    let chunk = tick.chunk(0)?;
    let mut scratch = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut scratch)?;
    let turn = rows[0][0] as u32;
    rows[0][0] += 1.;
    rows[0][1] = pose.pose_blend.map_or(-1., |b| b.seconds);
    rows[0][2] = pose.seconds;
    rows[1][0] = parameter.value;
    rows[1][1] = graph.seconds;
    rows[1][2] = pose.pose_blend.map_or(-1., |b| b.weight);
    tick.write_transforms(chunk, rows)?;
    match turn {
        0 | 4 | 5 | 6 => tick.control_animation_graph(&entity, G::SetParameter {
            parameter: parameter.id, value: match turn { 0 => 0.75, 4 => 0., 5 => 1., _ => 0.5 },
        })?,
        1 => tick.control_animation(&entity, A::Playing(false))?,
        2 | 9 => tick.control_animation_graph(&entity, G::SelectState(graph.state))?,
        3 => tick.control_animation(&entity, A::Playing(true))?,
        _ => {}
    }
    let mut after = [0; 256];
    tick.animation_graphs(&mut after)?;
    if after != graph_bytes { return Err(Error::Malformed); }
    after.fill(0);
    tick.animations(&mut after)?;
    if after != pose_bytes { return Err(Error::Malformed); }
    tick.log("animation graph accepted", LogLevel::Info)?;
    Ok(())
}
