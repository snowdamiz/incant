fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--build-info"] {
        println!(
            "panic={} debug_assertions={}",
            if cfg!(panic = "abort") {
                "abort"
            } else {
                "unwind"
            },
            cfg!(debug_assertions),
        );
        return Ok(());
    }
    if args.len() == 2 && args[0] == "--panic-probe" {
        // Deliberate subprocess-only failure used by the shipping packager.
        // A test harness overrides panic=abort, so it cannot verify this behavior.
        struct UnwindMarker<'a>(&'a str);
        impl Drop for UnwindMarker<'_> {
            fn drop(&mut self) {
                let _ = std::fs::write(self.0, "unwound");
            }
        }
        let _marker = UnwindMarker(&args[1]);
        panic!("incant shipping panic probe");
    }
    if args.len() != 2 {
        return Err(
            "usage: run_scene <scene> <ticks> | --build-info | --panic-probe <marker>".into(),
        );
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
