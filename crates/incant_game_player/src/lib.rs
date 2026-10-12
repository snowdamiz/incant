//! Build-produced script + native input/physics/audio proof player. Setup exchange
//! JSON is bounded and parsed once; ticks contain no document or JSON projection.
//! Simulation failure creates no output. Output I/O itself is not atomic.
use incant_assets_runtime::sha256;
mod checkpoint;
#[cfg(feature = "render-gpu")]
mod gpu;
#[cfg(feature = "mobile-ffi")]
pub mod mobile;
mod observe;
mod package;
mod program;
mod session;
mod startup;
mod ui;
#[cfg(all(
    feature = "native-window",
    any(target_os = "macos", target_os = "windows", target_os = "linux")
))]
mod window;
use incant_input::InputReplay;
use incant_runtime::CookedScene;
use session::Player;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
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
/// Run the ordinary packaged player from a trusted platform host.
/// Reports are emitted only at completion (or a completed failed assertion),
/// never as a per-tick world projection. CLI and embedded hosts share this path.
pub fn run(
    args: Vec<String>,
    mut emit: impl FnMut(&str),
) -> Result<(), Box<dyn std::error::Error>> {
    let (args, checkpoint) = checkpoint::Options::split(args)?;
    let args = package::expand(args)?;
    if args.as_slice() == ["--build-info"] {
        let code = embedded::load()?;
        let m = code.metadata();
        emit(&format!(
            "engine={} target={} build={} source_parsing=disabled",
            m.engine, m.target, m.build_id
        ));
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "--window") {
        #[cfg(all(
            feature = "native-window",
            any(target_os = "macos", target_os = "windows", target_os = "linux")
        ))]
        return window::run(&args[1..]);
        #[cfg(not(all(
            feature = "native-window",
            any(target_os = "macos", target_os = "windows", target_os = "linux")
        )))]
        return Err("native window mode requires the native-window feature on desktop".into());
    }
    let (args, module) = program::split_args(&args)?;
    let (args, observation_path) = observe::split_args(args);
    let spatial = session::settings(args)?;
    let ticks: u64 = args[5].parse()?;
    let rate: u32 = args[6].parse()?;
    if !(1..=3600).contains(&ticks) || !(8000..=192000).contains(&rate) {
        return Err("tick/sample-rate bounds".into());
    }
    let prepare_observations = |end_tick| {
        observation_path
            .map(|path| -> Result<_, Box<dyn std::error::Error>> {
                Ok(observe::Plan::parse(
                    &bounded(path, observe::MAX_PLAN_BYTES)?,
                    end_tick,
                )?)
            })
            .transpose()
    };
    // Fresh runs know their absolute end tick before scene/device setup. Resumed
    // runs must first restore the world so plans retain absolute tick bounds.
    let mut observations = if checkpoint.load.is_none() {
        prepare_observations(ticks)?
    } else {
        None
    };
    let scene = CookedScene::from_bytes(&bounded(&args[0], 128 * 1024 * 1024)?)?;
    if spatial.is_none() && (!scene.spatial().is_empty() || !scene.visual_assets().is_empty()) {
        return Err("this game player has no spatial host".into());
    }
    let replay = InputReplay::from_text(
        std::str::from_utf8(&bounded(&args[3], incant_input::MAX_RECORDING_BYTES)?)?,
        0,
        scene.tick_rate(),
    )?;
    session::validate_ui_settings(&spatial, replay.has_ui())?;
    if ticks > replay.end_tick() {
        return Err("recording does not cover requested ticks".into());
    }
    let actions = serde_json::from_slice(&bounded(&args[4], 256 * 1024)?)?;
    let clips = startup::clips(&args[1], &args[2])?;
    let mut game = if let Some(path) = &checkpoint.load {
        Player::from_checkpoint(
            &bounded(path, incant_script::MAX_CHECKPOINT_BYTES)?,
            &scene,
            program::load(module)?,
            actions,
            clips,
            rate,
            spatial,
        )?
    } else {
        Player::new(
            scene,
            program::load(module)?,
            actions,
            clips,
            rate,
            spatial,
            ticks,
        )?
    };
    game.validate_ui_recording(replay.has_ui())?;
    // A requested save must be supported before gameplay begins. The final save
    // is checked again: a script may introduce an unsupported capability later.
    if checkpoint.save.is_some() {
        game.checkpoint_bytes()?;
    }
    let start_tick = game.world()?.tick();
    let (end_tick, frames) =
        checkpoint::frame_range(start_tick, ticks, rate, game.world()?.tick_rate())?;
    let previous_frames = game.rendered_frames()?;
    if checkpoint.load.is_some() {
        observations = prepare_observations(end_tick)?;
    }
    if let Some(plan) = &mut observations {
        plan.observe(game.world()?, game.input()?)?;
    }
    let mut output = vec![0.; frames as usize * 2];
    let mut offset = 0;
    for tick in 1..=ticks {
        let end = offset + game.next_output_frames()? * 2;
        game.tick_recorded(
            replay.events_at(tick)?,
            replay.ui_at(tick)?,
            replay.viewport_at(tick)?,
            &mut output[offset..end],
        )?;
        if let Some(plan) = &mut observations {
            plan.observe(game.world()?, game.input()?)?;
        }
        offset = end;
    }
    if offset != output.len() || game.rendered_frames()? - previous_frames != frames {
        return Err("sample clock invariant".into());
    }
    let world = game.world()?;
    // Bounded by the cooked 128-source cap, not the total world entity count.
    let sources:Vec<_>=world.audio_nodes().filter(|(node,_)|node.source.is_some()).map(|(node,pose)| {
        let physics=world.physics_state(node.entity).map(|s|serde_json::json!({"position":s.translation,"velocity":s.velocity,"sleeping":s.sleeping}));
        serde_json::json!({"id":node.entity.0,"position":pose[3][..3],"physics":physics})
    }).collect();
    let bytes: Vec<_> = output.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut report = serde_json::json!({"tick":world.tick(),"frames":frames as u64,"pcm_sha256":sha256(&bytes),"sources":sources,"logs":game.logs()?});
    // One final bounded observation, never a tick-time projection. Playback
    // rows contain no asset data or private rational clock remainder.
    report["flipbooks"] = world
        .spatial_view()?
        .nodes()
        .filter_map(|node| {
            node.flipbook.map(|state| {
                serde_json::json!({"id":node.id.0,"frame":state.frame(),
            "playing":state.playing(),"completed":state.completed()})
            })
        })
        .collect::<Vec<_>>()
        .into();
    if let Some(input) = game.render_report()? {
        report["render_input"] = input;
    }
    let mut passed = true;
    if let Some(plan) = observations {
        let observed = plan.finish(world)?;
        passed = observed.passed;
        report["observations"] = serde_json::to_value(observed)?;
    }
    let save = if checkpoint.save.is_some() {
        if !passed {
            emit(&report.to_string());
            return Err("checkpoint withheld: native observation assertions failed".into());
        }
        Some(game.checkpoint_bytes()?)
    } else {
        None
    };
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[7])?
        .write_all(&bytes)?;
    if let (Some(path), Some(bytes)) = (&checkpoint.save, save) {
        checkpoint::publish(std::path::Path::new(path), &bytes)?;
    }
    emit(&report.to_string());
    if !passed {
        return Err("native observation assertions failed (see completed report)".into());
    }
    Ok(())
}
