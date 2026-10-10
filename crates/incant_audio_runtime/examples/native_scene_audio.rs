//! Source-free offline native audio host. Setup accepts a cooked scene and a
//! bounded package clip table (NCLIPS01, u32 count, repeated ID[16]+key[32]).
//! Every tick steps the native world, syncs audio, then renders its exact rational
//! share of 48 kHz samples. Simulation/host failure stops before output creation.
//! Output I/O is not atomic: a write failure may leave a partial PCM proof file.
//! No world, input, script VM or audio-cursor rollback/save is claimed.
use incant_assets_runtime::sha256;
use incant_audio_runtime::{ClipBinding, MAX_SCENE_CLIPS, SceneAudio};
use incant_runtime::{NativeWorld, StableId};
use std::{
    fmt::Write as _,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

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
    if args.len() != 6 {
        return Err("expected SCENE CACHE CLIPS TICKS BATCH OUTPUT".into());
    }
    let ticks: u64 = args[3].parse()?;
    let batch: usize = args[4].parse()?;
    if !(1..=3600).contains(&ticks) || !(1..=8192).contains(&batch) {
        return Err("tick/batch bound".into());
    }
    let mut world = NativeWorld::from_bytes(&bounded(&args[0], 128 * 1024 * 1024)?)?;
    let frames = ticks * 48000 / u64::from(world.tick_rate());
    if frames > 1_000_000 {
        return Err("output frame bound".into());
    }
    let table = bounded(&args[2], 12 + MAX_SCENE_CLIPS * 48)?;
    if table.len() < 12 || &table[..8] != b"NCLIPS01" {
        return Err("clip table header".into());
    }
    let count = u32::from_le_bytes(table[8..12].try_into()?) as usize;
    if count > MAX_SCENE_CLIPS || table.len() != 12 + count * 48 {
        return Err("clip table length/count".into());
    }
    let mut clips = Vec::with_capacity(count);
    for record in table[12..].as_chunks::<48>().0 {
        let id = StableId(record[..16].try_into()?);
        let mut key = String::with_capacity(64);
        for byte in &record[16..] {
            write!(&mut key, "{byte:02x}")?;
        }
        clips.push(ClipBinding::load(id, Path::new(&args[1]), &key)?);
    }
    let mut audio = SceneAudio::offline(&world, clips, 48000)?;
    let mut output = vec![0.; frames as usize * 2];
    let mut begin = 0;
    for tick in 1..=ticks {
        // Never catch a simulation/host error and continue with advanced state.
        world.step()?;
        audio.sync(&world)?;
        let end = (tick * 48000 / u64::from(world.tick_rate())) as usize * 2;
        for block in output[begin..end].chunks_mut(batch * 2) {
            audio.render(block)?;
        }
        begin = end;
    }
    let bytes: Vec<_> = output.iter().flat_map(|v| v.to_le_bytes()).collect();
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[5])?
        .write_all(&bytes)?;
    println!("{} tick={} frames={frames}", sha256(&bytes), world.tick());
    for (node, pose) in world.audio_nodes().filter(|(n, _)| n.source.is_some()) {
        println!("source={:?} position={:?}", node.entity, &pose[3][..3]);
    }
    Ok(())
}
