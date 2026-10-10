//! Offline shipping-profile probe. Consumes only a cooked cache and fingerprint;
//! neither this executable nor its normal graph includes authoring/importer code.
use incant_assets_runtime::{load_audio, sha256};
use incant_audio_runtime::{Mixer, PlayOptions};
use std::{fs::OpenOptions, io::Write, path::Path, sync::Arc};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 7 {
        return Err("expected CACHE KEY static|stream RATE FRAMES BATCH OUTPUT".into());
    }
    let streaming = match args[2].as_str() {
        "static" => false,
        "stream" => true,
        _ => return Err("invalid mode".into()),
    };
    let rate: f64 = args[3].parse()?;
    let frames: usize = args[4].parse()?;
    let batch: usize = args[5].parse()?;
    if !(1..=1_000_000).contains(&frames) || !(1..=8192).contains(&batch) {
        return Err("invalid frame/batch bound".into());
    }
    let clip = Arc::new(load_audio(Path::new(&args[0]), &args[1])?);
    let mut mixer = Mixer::offline(44100, &[])?;
    mixer.play(
        clip,
        PlayOptions {
            streaming,
            looping: true,
            start_seconds: 0.17,
            rate,
            ..Default::default()
        },
    )?;
    let mut pcm = vec![0.; frames * 2];
    for block in pcm.chunks_mut(batch * 2) {
        mixer.render(block)?;
        if !mixer.stream_errors().is_empty() {
            return Err("cooked audio stream failed".into());
        }
    }
    if pcm.iter().any(|v| !v.is_finite()) {
        return Err("nonfinite mixed audio".into());
    }
    let bytes: Vec<_> = pcm.iter().flat_map(|v| v.to_le_bytes()).collect();
    // Never truncate an existing result or publish a report for a failed mix.
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[6])?
        .write_all(&bytes)?;
    println!("{}", sha256(&bytes));
    Ok(())
}
