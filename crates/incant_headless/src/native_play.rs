//! Development project play on the native graph. Project validation/cooking and
//! binary loading precede simulation. Authored TypeScript uses a fixed trusted
//! startup cook; scenes without attachments use the native build's embedded
//! behavior. Simulation has no source parsing or authoring dependency.
use incant_input::InputReplay;
use incant_observe::Plan;
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/behavior.rs"));
}
use serde::Serialize;
mod session;
pub(crate) use session::{Session, StartOptions};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub struct Options {
    pub project: PathBuf,
    /// Select a scene; omission is accepted only when the project has one scene.
    #[arg(long)]
    pub scene: Option<String>,
    /// Require the build-produced behavior to match this source SHA-256.
    #[arg(long)]
    pub script_sha256: Option<String>,
    #[arg(long, conflicts_with="seconds", value_parser=clap::value_parser!(u64).range(1..=36000))]
    pub ticks: Option<u64>,
    /// Simulation time, rounded up to a whole tick. Defaults to one second.
    #[arg(long, conflicts_with = "ticks")]
    pub seconds: Option<f64>,
    #[arg(long)]
    pub input_replay: Option<PathBuf>,
    /// Bounded typed assertions and selected final entities, as in the native player.
    #[arg(long)]
    pub observe: Option<PathBuf>,
    /// Required for any scene with spatial content. Validates retained CPU input.
    #[arg(long)]
    pub camera: Option<String>,
    #[arg(long,default_value_t=48000,value_parser=clap::value_parser!(u32).range(8000..=192000))]
    pub sample_rate: u32,
    /// New raw stereo f32-le PCM file; published only after a completed run.
    #[arg(long)]
    pub pcm_output: Option<PathBuf>,
}
#[derive(Debug, thiserror::Error)]
pub enum PlayError {
    #[error("native play: {0}")]
    Invalid(&'static str),
    #[error("native play IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("native play startup/runtime failure: {0}")]
    Native(#[source] Box<dyn std::error::Error>),
}
type Result<T> = std::result::Result<T, PlayError>;
fn native<E: std::error::Error + 'static>(error: E) -> PlayError {
    PlayError::Native(Box::new(error))
}
fn invalid(message: &'static str) -> PlayError {
    PlayError::Invalid(message)
}
fn bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid("startup input exceeds its byte limit"));
    }
    Ok(bytes)
}
fn ticks(options: &Options, rate: u32) -> Result<u64> {
    if let Some(ticks) = options.ticks {
        if !(1..=36000).contains(&ticks) || options.seconds.is_some() {
            return Err(invalid("invalid tick count"));
        }
        return Ok(ticks);
    }
    let seconds = options.seconds.unwrap_or(1.);
    let ticks = (seconds * f64::from(rate)).ceil();
    if !seconds.is_finite() || seconds <= 0. || !(1. ..=36000.).contains(&ticks) {
        return Err(invalid(
            "simulation duration must be finite, positive and at most 36000 ticks",
        ));
    }
    Ok(ticks as u64)
}

#[derive(Serialize)]
pub struct Report {
    pub passed: bool,
    runtime: &'static str,
    script_source_sha256: String,
    script_source_parsing: bool,
    scene: String,
    tick: u64,
    tick_rate: u32,
    entity_count: usize,
    frames: u128,
    pcm_sha256: String,
    logs: serde_json::Value,
    observations: Option<incant_observe::Report>,
    cpu_render_input: bool,
    gpu_submitted: bool,
    project_sha256: String,
}

pub fn run(options: Options) -> Result<Report> {
    if !(8000..=192000).contains(&options.sample_rate) {
        return Err(invalid("invalid sample rate"));
    }
    let bytes = bounded(&options.project, 128 * 1024 * 1024)?;
    let project_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let document = incant_doc::Project::from_text(std::str::from_utf8(&bytes).map_err(native)?)
        .map_err(native)?;
    drop(bytes);
    let end = ticks(&options, document.settings.tick_rate)?;
    let frame_count =
        u128::from(end) * u128::from(options.sample_rate) / u128::from(document.settings.tick_rate);
    if options.pcm_output.is_some() && frame_count * 8 > 512 * 1024 * 1024 {
        return Err(invalid("PCM output exceeds 512 MiB"));
    }
    let replay = options
        .input_replay
        .as_deref()
        .map(|path| {
            let bytes = bounded(path, incant_input::MAX_RECORDING_BYTES)?;
            let replay = InputReplay::from_text(
                std::str::from_utf8(&bytes).map_err(native)?,
                0,
                document.settings.tick_rate,
            )
            .map_err(native)?;
            if replay.end_tick() < end {
                return Err(invalid("recording does not cover requested ticks"));
            }
            Ok(replay)
        })
        .transpose()?;
    let mut plan = options
        .observe
        .as_deref()
        .map(|p| Plan::parse(&bounded(p, incant_observe::MAX_PLAN_BYTES)?, end).map_err(native))
        .transpose()?;
    let root = options
        .project
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut game = Session::new(
        &document,
        root,
        &StartOptions {
            scene: options.scene.clone(),
            script_sha256: options.script_sha256.clone(),
            camera: options.camera.clone(),
        },
        options.sample_rate,
    )?;
    // No guest update retains or consults this authoring document.
    drop(document);
    let mut pcm = if let Some(path) = &options.pcm_output {
        if path.exists() {
            return Err(invalid("PCM output already exists"));
        }
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        Some(tempfile::NamedTempFile::new_in(parent)?)
    } else {
        None
    };
    if let Some(plan) = &mut plan {
        plan.observe(game.world()?, game.input()?).map_err(native)?;
    }
    let max_frames =
        (u64::from(options.sample_rate) / u64::from(game.world()?.tick_rate()) + 1) as usize;
    let mut samples = vec![0.; max_frames * 2];
    let mut bytes = Vec::with_capacity(max_frames * 8);
    let mut hash = Sha256::new();
    for tick in 1..=end {
        let frames = game.next_frames()?;
        let output = &mut samples[..frames * 2];
        let events = replay
            .as_ref()
            .map(|r| r.events_at(tick))
            .transpose()
            .map_err(native)?
            .unwrap_or(&[]);
        game.tick(events, output)?;
        bytes.clear();
        bytes.extend(output.iter().flat_map(|v| v.to_le_bytes()));
        hash.update(&bytes);
        if let Some(pcm) = &mut pcm {
            pcm.write_all(&bytes)?;
        }
        if let Some(plan) = &mut plan {
            plan.observe(game.world()?, game.input()?).map_err(native)?;
        }
    }
    let world = game.world()?;
    let observations = plan.map(|p| p.finish(world)).transpose().map_err(native)?;
    let report = Report {
        passed: observations.as_ref().is_none_or(|r| r.passed),
        runtime: "native",
        script_source_sha256: game.script_source_sha256.clone(),
        script_source_parsing: game.authored_scripts,
        scene: game.scene.clone(),
        tick: world.tick(),
        tick_rate: world.tick_rate(),
        entity_count: world.entity_count(),
        frames: game.frames()?,
        pcm_sha256: format!("{:x}", hash.finalize()),
        logs: serde_json::to_value(game.logs()?).map_err(native)?,
        observations,
        cpu_render_input: game.is_spatial(),
        gpu_submitted: false,
        project_sha256,
    };
    if let Some(pcm) = pcm {
        pcm.as_file().sync_all()?;
        pcm.persist_noclobber(options.pcm_output.as_ref().expect("requested output"))
            .map_err(|e| PlayError::Io(e.error))?;
    }
    Ok(report)
}
