//! Bounded isolated playback. Runtime edits use PlaySession's simulation command
//! bus; the authored project and its journal are never opened for writing.
use clap::Args;
use incant_script::PlaySession;
use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    time::Instant,
};
use thiserror::Error;

const MAX_TICKS: u64 = 10_000;
const MAX_FRAMES: u64 = 128;
const MAX_RAW_FRAME_BYTES: u64 = 256 * 1024 * 1024;
const NOOP: &str = "exports.default = { update() {} };";

#[derive(Args)]
pub struct Options {
    pub project: PathBuf,
    /// Duration rounded up to the next fixed tick; defaults to one second.
    #[arg(long, conflicts_with = "ticks")]
    pub seconds: Option<f64>,
    #[arg(long, conflicts_with = "seconds")]
    pub ticks: Option<u64>,
    /// Sandboxed JavaScript emitted by the bundled TypeScript compiler.
    #[arg(long)]
    pub compiled_script: Option<PathBuf>,
    /// New directory for PNG frames and report.json; existing paths are rejected.
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Capture the initial/final state and every N fixed ticks in between.
    #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u64).range(1..))]
    pub capture_every: u64,
    #[arg(long, default_value_t = 640, value_parser = clap::value_parser!(u32).range(1..=2048))]
    pub width: u32,
    #[arg(long, default_value_t = 360, value_parser = clap::value_parser!(u32).range(1..=2048))]
    pub height: u32,
}

#[derive(Debug, Error)]
pub enum PlayError {
    #[error("play duration must be finite, nonnegative and at most {MAX_TICKS} fixed ticks")]
    Duration,
    #[error("capture plan exceeds {MAX_FRAMES} frames or 256 MiB of raw pixels")]
    CaptureBudget,
    #[error("compiled script exceeds 1,000,000 bytes")]
    ScriptSize,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Script(#[from] incant_script::ScriptError),
    #[error("project or asset loading failed: {0}")]
    Load(String),
    #[error("frame rendering failed: {0}")]
    Render(String),
}

#[derive(Serialize)]
pub struct Frame {
    tick: u64,
    elapsed_seconds: f64,
    file: String,
    geometry: incant_render::SceneStats,
}

#[derive(Serialize)]
pub struct Report {
    format_version: u32,
    completed: bool,
    ticks: u64,
    script_commands: usize,
    state: incant_core::RuntimeSnapshot,
    script_state: serde_json::Value,
    frames: Vec<Frame>,
    adapter: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    wall_ms: f64,
}

fn ticks(options: &Options, rate: u32) -> Result<u64, PlayError> {
    let count = if let Some(ticks) = options.ticks {
        ticks
    } else {
        let seconds = options.seconds.unwrap_or(1.);
        let count = (seconds * f64::from(rate)).ceil();
        if !seconds.is_finite() || seconds < 0. || count > MAX_TICKS as f64 {
            return Err(PlayError::Duration);
        }
        count as u64
    };
    if count > MAX_TICKS {
        return Err(PlayError::Duration);
    }
    if options.output.is_some() {
        let frames = 1 + count.div_ceil(options.capture_every);
        let bytes = frames * u64::from(options.width) * u64::from(options.height) * 4;
        if frames > MAX_FRAMES || bytes > MAX_RAW_FRAME_BYTES {
            return Err(PlayError::CaptureBudget);
        }
    }
    Ok(count)
}

pub fn run(options: Options) -> Result<Report, PlayError> {
    let document = super::read_project(&options.project)
        .map_err(|error| PlayError::Load(error.to_string()))?;
    let count = ticks(&options, document.settings.tick_rate)?;
    let mut assets = super::assets::load_runtime(&options.project, &document)
        .map_err(|error| PlayError::Load(error.to_string()))?;
    let source = if let Some(path) = &options.compiled_script {
        let mut source = String::new();
        fs::File::open(path)?
            .take(1_000_001)
            .read_to_string(&mut source)?;
        if source.len() > 1_000_000 {
            return Err(PlayError::ScriptSize);
        }
        source
    } else {
        NOOP.into()
    };
    let mut play = PlaySession::new(&document, &source)?;
    // Reserve a new output directory before GPU setup. Never overwrite a prior run.
    // A failed run may leave partial PNGs, but never a completed report.json.
    let renderer = if let Some(output) = &options.output {
        if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir(output)?;
        Some(incant_render::Renderer::headless().map_err(|e| PlayError::Render(e.to_string()))?)
    } else {
        None
    };
    let root = options
        .project
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let start = Instant::now();
    let mut frames = Vec::new();
    let mut commands = 0;
    // Keep the previous GPU versions alive until their replacement is prepared,
    // allowing static geometry buffers to be reused between captured frames.
    let mut retained_scene = None;
    for tick in 0..=count {
        if tick != 0 {
            commands += play.tick()?;
        }
        if let (Some(renderer), Some(output)) = (&renderer, &options.output)
            && (tick % options.capture_every == 0 || tick == count)
        {
            assets
                .sync_project(play.project(), root)
                .map_err(|error| PlayError::Load(error.to_string()))?;
            let scene = retained_scene.insert(
                renderer
                    .prepare_scene(play.project(), &assets)
                    .map_err(|error| PlayError::Render(error.to_string()))?,
            );
            let png = renderer
                .screenshot_scene_png(scene, options.width, options.height)
                .map_err(|error| PlayError::Render(error.to_string()))?;
            let file = format!("frame-{tick:06}.png");
            fs::write(output.join(&file), png)?;
            frames.push(Frame {
                tick,
                elapsed_seconds: play.snapshot().elapsed_seconds,
                file,
                geometry: scene.stats().clone(),
            });
        }
    }
    let report = Report {
        format_version: 1,
        completed: true,
        ticks: count,
        script_commands: commands,
        state: play.snapshot(),
        script_state: play.host.state().clone(),
        frames,
        adapter: renderer.as_ref().map(|r| r.adapter_name.clone()),
        width: renderer.as_ref().map(|_| options.width),
        height: renderer.as_ref().map(|_| options.height),
        wall_ms: start.elapsed().as_secs_f64() * 1000.,
    };
    if let Some(output) = options.output {
        let mut file = tempfile::NamedTempFile::new_in(&output)?;
        file.write_all(&serde_json::to_vec_pretty(&report)?)?;
        file.as_file().sync_all()?;
        file.persist(output.join("report.json"))
            .map_err(|error| error.error)?;
    }
    Ok(report)
}
