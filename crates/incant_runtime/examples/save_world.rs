//! Small source-independent native-world save/restore executable, not a full
//! game-session exporter. Input is checked scene/save data; no authoring code.
use incant_runtime::{MAX_SAVE_BYTES, NativeWorld};
use std::{fs::File, io::Read, io::Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 4 {
        return Err("usage: save_world <scene|save> <input> <ticks> <new-save-output>".into());
    }
    let ticks: u64 = args[2].to_str().ok_or("ticks must be UTF-8")?.parse()?;
    if ticks > 10_000_000 {
        return Err("tick limit is 10,000,000".into());
    }
    let mut bytes = Vec::new();
    File::open(&args[1])?
        .take(MAX_SAVE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SAVE_BYTES {
        return Err("input exceeds native world save limit".into());
    }
    let mut world = match args[0].to_str() {
        Some("scene") => NativeWorld::from_bytes(&bytes)?,
        Some("save") => NativeWorld::from_save_bytes(&bytes)?,
        _ => return Err("input kind must be scene or save".into()),
    };
    for _ in 0..ticks {
        world.step()?;
    }
    let save = world.save_bytes()?;
    // Refuse to overwrite input, existing saves or unrelated files. Production
    // slot selection/atomic durable file publication is a separate host concern.
    File::options()
        .write(true)
        .create_new(true)
        .open(&args[3])?
        .write_all(&save)?;
    println!(
        "tick={} rate={} entities={} bytes={}",
        world.tick(),
        world.tick_rate(),
        world.entity_count(),
        save.len()
    );
    Ok(())
}
