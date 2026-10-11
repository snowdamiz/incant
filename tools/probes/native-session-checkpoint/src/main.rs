mod embedded {
    include!(concat!(env!("OUT_DIR"), "/behavior.rs"));
}
use incant_script::NativeSession;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("mode, input-or-dash, output required".into());
    }
    let code = embedded::load()?;
    let initial = incant_runtime::CookedScene::from_bytes(include_bytes!(concat!(
        env!("OUT_DIR"), "/world.scene"
    )))?;
    let mut session = if args[1] == "resume" {
        NativeSession::from_checkpoint_bytes(&std::fs::read(&args[2])?, &initial, code, Default::default())?
    } else {
        NativeSession::new(initial, code, Default::default())?
    };
    let ticks = match args[1].as_str() {
        "baseline" => 40,
        "save" => 13,
        "resume" => 27,
        _ => return Err("unknown mode".into()),
    };
    for _ in 0..ticks {
        session.tick(&[])?;
    }
    let final_tick = session.world()?.tick();
    for (n, speed) in [(1, 2), (2, 5), (3, 9)] {
        let mut id = [0u8; 16];
        id[15] = n;
        let value = session
            .world()?
            .inspect(incant_runtime::StableId(id))
            .ok_or("entity missing")?
            .transform
            .translation[0];
        let expected = (final_tick * speed + (final_tick - 1) / 3) as f64;
        if value != expected {
            return Err(format!("counter {n}: {value} != {expected}").into());
        }
    }
    let checkpoint = session.checkpoint_bytes()?;
    std::fs::write(&args[3], &checkpoint)?;
    println!(
        "tick={final_tick} bytes={} source_parser=false ordinary_load=true",
        checkpoint.len()
    );
    Ok(())
}
