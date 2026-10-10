fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: run_scene <scene> <ticks>".into());
    }
    let ticks: u64 = args[1].parse()?;
    let mut world = incant_runtime::NativeWorld::from_bytes(&std::fs::read(&args[0])?)?;
    for _ in 0..ticks {
        world.step()?;
    }
    println!("tick={} entities={}", world.tick(), world.entity_count());
    for entity in world.snapshot() {
        println!("{:?} {:?}", entity.id, entity.transform.translation);
    }
    Ok(())
}
