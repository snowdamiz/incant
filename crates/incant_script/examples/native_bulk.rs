//! Development runner: compile SWC output once, then execute the native script
//! phase against a checked binary scene. No project or authoring crates linked.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: native_bulk <cooked-scene> <compiled-commonjs> <ticks>".into());
    }
    let scene = incant_runtime::CookedScene::from_bytes(&std::fs::read(&args[0])?)?;
    let dt = 1. / f64::from(scene.tick_rate());
    let script = incant_script::CompiledScript::compile(&std::fs::read_to_string(&args[1])?)?;
    let mut host = incant_script::ScriptHost::new(script)?;
    let mut world = incant_runtime::NativeWorld::new(scene);
    for _ in 0..args[2].parse::<u64>()? {
        world.step()?;
        host.tick(&mut world, dt)?;
    }
    println!("tick={} entities={}", world.tick(), world.entity_count());
    for entity in world.snapshot() {
        println!("{:?} {:?}", entity.id, entity.transform.translation);
    }
    Ok(())
}
