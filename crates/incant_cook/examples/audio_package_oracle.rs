//! Independent typed native declarations, transport and actual PCM on public cooked input.
#[allow(dead_code)]
#[path = "../../incant_wasm_runtime/tests/support/audio_transport.rs"]
mod fixture;
use incant_audio_runtime::{ClipBinding, SceneAudio};
use incant_runtime::*;
use std::{fs, io::Write, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 {
        return Err("usage: audio_package_oracle PROJECT SCENE SCENE_OUT PCM_OUT TICKS".into());
    }
    let path = Path::new(&args[0]);
    let project = incant_doc::Project::from_text(&fs::read_to_string(path)?)?;
    let bytes = incant_cook::cook_scene(&project, &args[1])?;
    let scene = CookedScene::from_bytes(&bytes)?;
    let ticks: u32 = args[4].parse()?;
    if scene.id() != fixture::id(99) || scene.tick_rate() != 60 || ticks != 96 {
        return Err("oracle scene/clock bounds".into());
    }
    let root = path.parent().ok_or("project parent")?;
    let mut clips = Vec::new();
    for asset in project.assets.values().filter(|a| a.kind == "audio") {
        let id = StableId(asset.id.parse::<ulid::Ulid>()?.to_bytes());
        clips.push(ClipBinding::load(
            id,
            &root.join(".incant/cache/audio"),
            &asset.sha256,
        )?);
    }
    if clips.len() != 2 {
        return Err("oracle requires two actual captured audio clips".into());
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?
        .write_all(&bytes)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[3])?;
    let mut world = NativeWorld::new(scene);
    let mut audio = SceneAudio::offline(&world, clips, 48000)?;
    let mut pcm = vec![0.; 1600];
    for turn in 0..ticks {
        world.step()?;
        let values = fixture::observations(&world);
        fixture::oracle(&mut world, turn);
        audio.sync(&world)?;
        audio.render(&mut pcm)?;
        for value in &pcm {
            output.write_all(&value.to_le_bytes())?;
        }
        println!(
            "{}",
            serde_json::json!({"tick":turn+1,"values":values.as_slice(),"pcm_nonzero":pcm.iter().any(|x| x.abs()>0.001),"pcm_silent":pcm.iter().all(|x| *x==0.)})
        );
    }
    Ok(())
}
