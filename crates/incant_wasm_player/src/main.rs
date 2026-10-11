//! Composed compiler-free fixture player. No module/source loading interface.
//! Simulation/module/audio failure publishes no PCM file. Output I/O itself is
//! not atomic: a failed write can leave a partial disposable proof artifact.
use incant_game_runtime::OfflineGameSession;
use incant_runtime::CookedScene;
use incant_wasm_player::{SAMPLE_RATE, TICKS, actions, id, load_clip, replay, write_fixture};
use incant_wasm_runtime::embedded;
use std::{
    error::Error,
    fs,
    io::{Read, Write},
    path::Path,
};
fn bounded(path: &Path, limit: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("startup input byte limit".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if matches!(args.first().map(String::as_str), Some("--make-fixture"))
        && (args.len() == 2 || (args.len() == 3 && args[2] == "--stream"))
    {
        println!("{}", write_fixture(Path::new(&args[1]), args.len() == 3)?);
        return Ok(());
    }
    if args.len() != 6 {
        return Err("expected LINKED_MODULE COOKED_SCENE PCM_CACHE CLIP_KEY INPUT_RECORDING OUTPUT; no source/module loading supported".into());
    }
    let module = embedded::linked(&args[0]).ok_or("unknown linked module")?;
    let scene = CookedScene::from_bytes(&bounded(Path::new(&args[1]), 16 * 1024 * 1024)?)?;
    let text = String::from_utf8(bounded(
        Path::new(&args[4]),
        incant_input::MAX_RECORDING_BYTES,
    )?)?;
    let replay = replay(&text, &scene)?;
    let clip = load_clip(Path::new(&args[2]), &args[3])?;
    let mut game = OfflineGameSession::new(scene, module, actions(), vec![clip], SAMPLE_RATE)?;
    let total_frames = TICKS as usize * SAMPLE_RATE as usize / game.world()?.tick_rate() as usize;
    let mut output = vec![0.; total_frames * 2];
    let mut offset = 0;
    let mut pressed = 0;
    let mut released = 0;
    for tick in 1..=TICKS {
        let end = offset + game.next_output_frames()? * 2;
        game.tick(replay.events_at(tick)?, &mut output[offset..end])?;
        pressed += u32::from(game.input()?.actions["jump"].pressed);
        released += u32::from(game.input()?.actions["jump"].released);
        offset = end;
    }
    if offset != output.len() || output.iter().any(|v| !v.is_finite()) {
        return Err("PCM clock/value invariant".into());
    }
    let position = game
        .world()?
        .inspect(id(1))
        .ok_or("fixture source disappeared")?
        .transform
        .translation;
    let spawned = game
        .world()?
        .inspect(id(3))
        .ok_or("module spawn missing")?
        .transform
        .translation[0];
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[5])?;
    for value in output {
        file.write_all(&value.to_le_bytes())?;
    }
    println!(
        "tick={} frames={} source={position:?} spawned={spawned} pressed={pressed} released={released}",
        game.world()?.tick(),
        game.rendered_frames()?
    );
    Ok(())
}
