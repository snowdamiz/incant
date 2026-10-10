//! Build-produced script + native input/physics/audio proof player. Setup exchange
//! JSON is bounded and parsed once; ticks contain no document or JSON projection.
//! Simulation failure creates no output. Output I/O itself is not atomic.
use incant_assets_runtime::sha256;
use incant_audio_runtime::{ClipBinding, MAX_SCENE_CLIPS};
use incant_game_runtime::OfflineGameSession;
use incant_input::InputReplay;
use incant_runtime::{CookedScene, StableId};
use std::{
    fmt::Write as _,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/behavior.rs"));
}
fn bounded(path: &str, limit: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("runtime input size limit".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.as_slice() == ["--build-info"] {
        let code = embedded::load()?;
        let m = code.metadata();
        println!(
            "engine={} target={} build={} source_parsing=disabled",
            m.engine, m.target, m.build_id
        );
        return Ok(());
    }
    if args.len() != 8 {
        return Err("expected SCENE CACHE CLIPS RECORDING ACTIONS TICKS SAMPLE_RATE OUTPUT; manual input/source paths are unsupported".into());
    }
    let ticks: u64 = args[5].parse()?;
    let rate: u32 = args[6].parse()?;
    if !(1..=3600).contains(&ticks) || !(8000..=192000).contains(&rate) {
        return Err("tick/sample-rate bounds".into());
    }
    // This core decodes only v1-v3; spatial v4 requires an explicit host and is
    // unsupported until that capability is deliberately connected.
    let scene = CookedScene::from_bytes(&bounded(&args[0], 128 * 1024 * 1024)?)?;
    let frames = u128::from(ticks) * u128::from(rate) / u128::from(scene.tick_rate());
    if frames > 1_000_000 {
        return Err("output frame bound".into());
    }
    let replay = InputReplay::from_text(
        std::str::from_utf8(&bounded(&args[3], incant_input::MAX_RECORDING_BYTES)?)?,
        0,
        scene.tick_rate(),
    )?;
    if ticks > replay.end_tick() {
        return Err("recording does not cover requested ticks".into());
    }
    let actions = serde_json::from_slice(&bounded(&args[4], 256 * 1024)?)?;
    let table = bounded(&args[2], 12 + MAX_SCENE_CLIPS * 48)?;
    if table.len() < 12 || &table[..8] != b"NCLIPS01" {
        return Err("clip table header".into());
    }
    let count = u32::from_le_bytes(table[8..12].try_into()?) as usize;
    if count > MAX_SCENE_CLIPS || table.len() != 12 + count * 48 {
        return Err("clip table length/count".into());
    }
    let mut clips = Vec::with_capacity(count);
    for row in table[12..].as_chunks::<48>().0 {
        let id = StableId(row[..16].try_into()?);
        let mut key = String::with_capacity(64);
        for byte in &row[16..] {
            write!(&mut key, "{byte:02x}")?;
        }
        clips.push(ClipBinding::load(id, Path::new(&args[1]), &key)?);
    }
    let mut game = OfflineGameSession::new(scene, embedded::load()?, actions, clips, rate)?;
    let mut output = vec![0.; frames as usize * 2];
    let mut offset = 0;
    for tick in 1..=ticks {
        let end = offset + game.next_output_frames()? * 2;
        game.tick(replay.events_at(tick)?, &mut output[offset..end])?;
        offset = end;
    }
    if offset != output.len() || game.rendered_frames()? != frames {
        return Err("sample clock invariant".into());
    }
    let world = game.world()?;
    // Bounded by the cooked 128-source cap, not the total world entity count.
    let sources:Vec<_>=world.audio_nodes().filter(|(node,_)|node.source.is_some()).map(|(node,pose)| {
        let physics=world.physics_state(node.entity).map(|s|serde_json::json!({"position":s.translation,"velocity":s.velocity,"sleeping":s.sleeping}));
        serde_json::json!({"id":node.entity.0,"position":pose[3][..3],"physics":physics})
    }).collect();
    let bytes: Vec<_> = output.iter().flat_map(|v| v.to_le_bytes()).collect();
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[7])?
        .write_all(&bytes)?;
    println!(
        "{}",
        serde_json::json!({"tick":world.tick(),"frames":frames as u64,"pcm_sha256":sha256(&bytes),"sources":sources})
    );
    Ok(())
}
