//! Native build-produced script player. No source parsing or executable cache
//! loading at startup. Scene bytes remain independently checked untrusted data.
use std::io::Read;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/behavior.rs"));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.as_slice() == ["--build-info"] {
        let code = embedded::load()?;
        let metadata = code.metadata();
        println!(
            "engine={} target={} build={} source_parsing=disabled",
            metadata.engine, metadata.target, metadata.build_id
        );
        return Ok(());
    }
    if args.len() != 2 {
        return Err("usage: incant_script_player <cooked-scene> <ticks>".into());
    }
    // Keep file acquisition bounded before the scene decoder checks its own
    // format/count limits. This is a player input cap, not an expanded format.
    const MAX_SCENE_BYTES: u64 = 128 * 1024 * 1024;
    let mut bytes = Vec::new();
    std::fs::File::open(&args[0])?
        .take(MAX_SCENE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SCENE_BYTES {
        return Err("scene file exceeds player limit".into());
    }
    let scene = incant_runtime::CookedScene::from_bytes(&bytes)?;
    let dt = 1. / f64::from(scene.tick_rate());
    let ticks: u64 = args[1].parse()?;
    if ticks > 10_000_000 {
        return Err("tick count exceeds player limit".into());
    }
    let mut host = incant_script::ScriptHost::new(embedded::load()?)?;
    let mut world = incant_runtime::NativeWorld::new(scene);
    for _ in 0..ticks {
        world.step()?;
        host.tick(&mut world, dt)?;
    }
    println!("tick={} entities={}", world.tick(), world.entity_count());
    for entity in world.snapshot() {
        println!("{:?} {:?}", entity.id, entity.transform.translation);
    }
    Ok(())
}
