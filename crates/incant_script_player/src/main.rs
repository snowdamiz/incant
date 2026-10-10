//! Native build-produced script player. No source parsing or executable cache
//! loading at startup. Scene bytes remain independently checked untrusted data.
use std::io::Read;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/behavior.rs"));
}

fn read_bounded(path: &str, limit: u64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("input file exceeds player limit".into());
    }
    Ok(bytes)
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
    let replay_paths = match args.as_slice() {
        [_, _] => None,
        [_, _, clip_flag, clip, actions_flag, actions]
            if clip_flag == "--input-recording" && actions_flag == "--input-actions" => Some((clip, actions)),
        _ => return Err("usage: incant_script_player <cooked-scene> <ticks> [--input-recording <clip.json> --input-actions <actions.json>]; duplicate, conflicting and manual input arguments are unsupported".into()),
    };
    // Keep file acquisition bounded before the scene decoder checks its own
    // format/count limits. This is a player input cap, not an expanded format.
    const MAX_SCENE_BYTES: u64 = 128 * 1024 * 1024;
    let scene = incant_runtime::CookedScene::from_bytes(&read_bounded(&args[0], MAX_SCENE_BYTES)?)?;
    let ticks: u64 = args[1].parse()?;
    if ticks > 10_000_000 {
        return Err("tick count exceeds player limit".into());
    }
    // JSON is only a bounded startup exchange format. The session consumes
    // typed event slices and immutable bindings; neither file is a cooked asset.
    let (replay, actions) = if let Some((clip, actions)) = replay_paths {
        let bytes = read_bounded(clip, incant_input::MAX_RECORDING_BYTES as u64)?;
        // New sessions always start at tick zero. from_text enforces the cooked
        // rate and a clip that includes zero (therefore start_tick == zero).
        let replay = incant_input::InputReplay::from_text(
            std::str::from_utf8(&bytes)?,
            0,
            scene.tick_rate(),
        )?;
        if ticks > replay.end_tick() {
            return Err("input recording does not cover requested ticks".into());
        }
        const MAX_ACTION_BYTES: u64 = 256 * 1024;
        let actions = serde_json::from_slice(&read_bounded(actions, MAX_ACTION_BYTES)?)?;
        (Some(replay), actions)
    } else {
        (None, incant_input::InputActions::default())
    };
    let mut session = incant_script::NativeSession::new(scene, embedded::load()?, actions)?;
    for tick in 1..=ticks {
        let events = match &replay {
            Some(replay) => replay.events_at(tick)?,
            None => &[],
        };
        session.tick(events)?;
    }
    let world = session.world()?;
    println!("tick={} entities={}", world.tick(), world.entity_count());
    for entity in world.snapshot() {
        println!("{:?} {:?}", entity.id, entity.transform.translation);
    }
    Ok(())
}
