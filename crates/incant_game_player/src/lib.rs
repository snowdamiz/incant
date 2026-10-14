//! Build-produced script + native input/physics/audio proof player. Setup exchange
//! JSON is bounded and parsed once; ticks contain no document or JSON projection.
//! Simulation failure creates no output. PCM publication is atomic and create-only;
//! PCM and an optional checkpoint are separate file transactions.
mod checkpoint;
#[cfg(feature = "render-gpu")]
mod gpu;
#[cfg(feature = "mobile-ffi")]
pub mod mobile;
#[cfg(feature = "mobile-android")]
mod mobile_android;
#[cfg(feature = "mobile-host")]
pub mod mobile_host;
#[cfg(all(feature = "mobile-apple", target_vendor = "apple"))]
pub mod mobile_native;
mod observe;
mod package;
mod pcm_output;
mod profile;
mod program;
mod restart;
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
use std::{fs::File, io::Read};
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
    run_owned(args, &mut emit, false)
}
/// Standalone process entry point. Explicit --profile-tracy starts a loopback-only
/// transport whose lifetime ends with the process; embedded hosts must use run().
pub fn run_cli(
    args: Vec<String>,
    mut emit: impl FnMut(&str),
) -> Result<(), Box<dyn std::error::Error>> {
    run_owned(args, &mut emit, true)
}
fn run_owned(
    args: Vec<String>,
    emit: &mut dyn FnMut(&str),
    standalone: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let (args, profile) = profile::split(args, standalone)?;
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
        {
            let mut report = window::run(&args[1..])?;
            profile::append(&mut report, &profile)?;
            emit(&report.to_string());
            return Ok(());
        }
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
    let program = program::load(module)?;
    let mut restart = restart::Restart::prepare(&scene, &program, &actions, &clips, &spatial)?;
    let mut game = if let Some(path) = &checkpoint.load {
        Player::from_checkpoint(
            &bounded(path, checkpoint::MAX_CHECKPOINT_BYTES)?,
            &scene,
            program,
            actions,
            clips,
            rate,
            spatial,
        )?
    } else {
        Player::new(scene, program, actions, clips, rate, spatial, ticks)?
    };
    restart.enable(&mut game)?;
    game.validate_ui_recording(replay.has_ui())?;
    // Check the selected route before gameplay. Native physics has no completed
    // owner before its first tick, so actual native save admission is performed
    // strictly at the final boundary, before either output is published.
    if checkpoint.save.is_some() {
        game.preflight_checkpoint()?;
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
    // The largest tick is independent of total capture length. Restart resets
    // fractional clock phase, so reserve the per-step ceiling for every step.
    let max_tick_frames = u64::from(rate).div_ceil(u64::from(game.world()?.tick_rate()));
    let max_frames = ticks
        .checked_mul(max_tick_frames)
        .ok_or("output frame bound")?;
    let mut output = vec![0.; usize::try_from(max_tick_frames)? * 2];
    let mut pcm = pcm_output::PcmOutput::new(std::path::Path::new(&args[7]), max_frames)?;
    for tick in 1..=ticks {
        let count = game.next_output_frames()? * 2;
        if count > output.len() {
            return Err("output frame bound".into());
        }
        profile::frame(|| {
            game.tick_recorded(
                restart.input.filter(replay.events_at(tick)?)?,
                replay.ui_at(tick)?,
                replay.viewport_at(tick)?,
                &mut output[..count],
            )
        })?;
        if let Some(plan) = &mut observations {
            plan.observe_at(start_tick + tick, game.world()?, game.input()?)?;
        }
        pcm.append(&output[..count])?;
        restart.viewport(replay.viewport_at(tick)?);
        {
            let _profile = incant_profile::scope(incant_profile::Phase::Restart);
            restart.accept(&mut game, start_tick + tick, u128::from(pcm.frames()))?;
        }
    }
    if restart.boundaries.is_empty()
        && (u128::from(pcm.frames()) != frames
            || game.rendered_frames()? - previous_frames != frames)
    {
        return Err("sample clock invariant".into());
    }
    let frames = pcm.frames();
    let world = game.world()?;
    // Bounded by the cooked 128-source cap, not the total world entity count.
    let sources:Vec<_>=world.audio_nodes().filter(|(node,_)|node.source.is_some()).map(|(node,pose)| {
        let physics=world.physics_state(node.entity).map(|s|serde_json::json!({"position":s.translation,"velocity":s.velocity,"sleeping":s.sleeping}));
        serde_json::json!({"id":node.entity.0,"position":pose[3][..3],"physics":physics})
    }).collect();
    let mut report = serde_json::json!({"tick":world.tick(),"frames":frames,"pcm_sha256":pcm.sha256(),"sources":sources,"logs":game.logs()?});
    if !restart.boundaries.is_empty() {
        report["session"] = serde_json::json!({"generation":restart.generation,
            "host_tick":end_tick,"simulation_tick":world.tick(),"restarts":restart.boundaries});
    }
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
    profile::append(&mut report, &profile)?;
    let save = if checkpoint.save.is_some() {
        if !passed {
            emit(&report.to_string());
            return Err("checkpoint withheld: native observation assertions failed".into());
        }
        Some(game.checkpoint_bytes()?)
    } else {
        None
    };
    pcm.publish(std::path::Path::new(&args[7]))?;
    if let (Some(path), Some(bytes)) = (&checkpoint.save, save) {
        checkpoint::publish(std::path::Path::new(path), &bytes)?;
    }
    emit(&report.to_string());
    if !passed {
        return Err("native observation assertions failed (see completed report)".into());
    }
    Ok(())
}
