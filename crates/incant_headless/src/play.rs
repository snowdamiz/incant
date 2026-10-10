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
    /// Restore logical game state for this exact authored project/script revision.
    #[arg(long)]
    pub load_save: Option<PathBuf>,
    /// Atomically publish a new game-save file after successful playback.
    #[arg(long)]
    pub save_output: Option<PathBuf>,
    /// Versioned fixed-tick keyboard/mouse/gamepad/touch clip; no device access.
    #[arg(long)]
    pub input_replay: Option<PathBuf>,
    /// Data-only assertions at absolute game ticks; failure exits nonzero.
    #[arg(long)]
    pub assertions: Option<PathBuf>,
    /// New directory for PNG frames and report.json; existing paths are rejected.
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Authored Camera entity ID, followed through each captured simulation tick.
    #[arg(long, requires = "output")]
    pub camera: Option<String>,
    /// New JSONL file for committed script logs; does not require GPU captures.
    #[arg(long)]
    pub log_output: Option<PathBuf>,
    /// New stereo float WAV file mixed offline; never opens an audio device.
    #[arg(long)]
    pub audio_output: Option<PathBuf>,
    #[arg(long, default_value_t = 48000, value_parser = clap::value_parser!(u32).range(8000..=192000))]
    pub audio_rate: u32,
    /// Capture the initial/final state and every N fixed ticks in between.
    #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u64).range(1..))]
    pub capture_every: u64,
    #[arg(long, default_value_t = 640, value_parser = clap::value_parser!(u32).range(i64::from(incant_render::MIN_SCREENSHOT_DIMENSION)..=i64::from(incant_render::MAX_SCREENSHOT_WIDTH)))]
    pub width: u32,
    #[arg(long, default_value_t = 360, value_parser = clap::value_parser!(u32).range(i64::from(incant_render::MIN_SCREENSHOT_DIMENSION)..=i64::from(incant_render::MAX_SCREENSHOT_HEIGHT)))]
    pub height: u32,
}

#[derive(Debug, Error)]
pub enum PlayError {
    #[error("play duration must be finite, nonnegative and at most {MAX_TICKS} fixed ticks")]
    Duration,
    #[error("capture plan exceeds {MAX_FRAMES} frames or 256 MiB of raw pixels")]
    CaptureBudget,
    #[error("script log capture exceeds 10,000 records or 8 MiB of JSONL output")]
    LogBudget,
    #[error("log output conflicts with a reserved frame or report filename")]
    LogOutputConflict,
    #[error("save output conflicts with a reserved frame, report or log filename")]
    SaveOutputConflict,
    #[error("audio output conflicts with a reserved frame, report, save or log filename")]
    AudioOutputConflict,
    #[error("audio export exceeds 256 MiB or its fixed-tick sample budget")]
    AudioBudget,
    #[error(transparent)]
    Audio(#[from] incant_audio::AudioError),
    #[error("compiled script exceeds 1,000,000 bytes")]
    ScriptSize,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Script(#[from] incant_script::ScriptError),
    #[error("play failed at tick {tick}: {source}")]
    Tick {
        tick: u64,
        #[source]
        source: incant_script::ScriptError,
    },
    #[error(transparent)]
    Save(#[from] incant_script::SaveError),
    #[error(transparent)]
    InputRecording(#[from] incant_input::RecordingError),
    #[error(transparent)]
    Assertions(#[from] super::play_assertions::AssertionError),
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
    shading: &'static str,
}

#[derive(Serialize)]
pub struct Report {
    format_version: u32,
    completed: bool,
    pub passed: bool,
    assertions: Vec<super::play_assertions::Outcome>,
    ticks: u64,
    start_tick: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    save_output: Option<PathBuf>,
    script_commands: usize,
    state: incant_core::RuntimeSnapshot,
    script_state: serde_json::Value,
    input: incant_input::InputFrame,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio: Option<super::play_audio::Report>,
    frames: Vec<Frame>,
    logs: Vec<super::play_logs::Entry>,
    adapter: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    camera: Option<String>,
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
    let mut play = if let Some(path) = &options.load_save {
        PlaySession::from_save_with_navigation_resources(
            &document,
            &source,
            &super::play_saves::load(path)?,
            assets.navigation_resources(),
        )?
    } else {
        PlaySession::with_navigation_resources(&document, &source, assets.navigation_resources())?
    };
    let start_tick = play.snapshot().tick;
    if let Some(path) = &options.input_replay {
        let mut text = String::new();
        fs::File::open(path)?
            .take(incant_input::MAX_RECORDING_BYTES as u64 + 1)
            .read_to_string(&mut text)?;
        play.replay_input(&text)?;
        if start_tick + count > play.input_replay_end().expect("installed replay") {
            return Err(incant_input::RecordingError::Range.into());
        }
    }
    let mut assertions = options
        .assertions
        .as_deref()
        .map(|path| super::play_assertions::Assertions::load(path, start_tick, start_tick + count))
        .transpose()?
        .unwrap_or_default();
    // Reserve a new output directory before GPU setup. Never overwrite a prior run.
    // A failed simulation may leave partial PNGs, but no completed report.
    // Completed runs with failed assertions retain a report with passed=false.
    if let Some(output) = &options.output {
        if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir(output)?;
    }
    let mut audio = options
        .audio_output
        .as_deref()
        .map(|path| {
            let snapshot = play.snapshot();
            super::play_audio::Output::new(
                path,
                play.project(),
                &snapshot,
                &assets,
                options.audio_rate,
                count,
                [
                    options.output.as_deref(),
                    options.save_output.as_deref(),
                    options.log_output.as_deref(),
                ],
            )
        })
        .transpose()?;
    let save_output = options
        .save_output
        .as_deref()
        .map(|path| {
            super::play_saves::Output::new(
                path,
                options.output.as_deref(),
                options.log_output.as_deref(),
            )
        })
        .transpose()?;
    let mut logs =
        super::play_logs::Capture::new(options.log_output.as_deref(), options.output.as_deref())?;
    let renderer = options
        .output
        .as_ref()
        .map(|_| incant_render::Renderer::headless().map_err(|e| PlayError::Render(e.to_string())))
        .transpose()?;
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
            commands += play.tick().map_err(|source| PlayError::Tick {
                tick: play.host.clock().tick.saturating_add(1),
                source,
            })?;
            if let Some(audio) = &mut audio {
                assets
                    .sync_project(play.project(), root)
                    .map_err(|e| PlayError::Load(e.to_string()))?;
                let snapshot = play.snapshot();
                audio.tick(tick, play.project(), &snapshot, &assets)?;
            }
            let entries = play.host.take_logs();
            if !entries.is_empty() {
                let snapshot = play.snapshot();
                logs.append(snapshot.tick, snapshot.elapsed_seconds, entries)?;
            }
        }
        assertions.evaluate(start_tick + tick, &mut play);
        if let (Some(renderer), Some(output)) = (&renderer, &options.output)
            && (tick % options.capture_every == 0 || tick == count)
        {
            assets
                .sync_project(play.project(), root)
                .map_err(|error| PlayError::Load(error.to_string()))?;
            let prepared = renderer
                .prepare_scene(play.project(), &assets)
                .map_err(|error| PlayError::Render(error.to_string()))?;
            let prepared = if let Some(id) = &options.camera {
                prepared
                    .with_camera(id)
                    .map_err(|error| PlayError::Render(error.to_string()))?
            } else {
                prepared
            };
            let scene = retained_scene.insert(prepared);
            let png = renderer
                .screenshot_scene_png(scene, options.width, options.height)
                .map_err(|error| PlayError::Render(error.to_string()))?;
            let clock = play.snapshot();
            let file = format!("frame-{:06}.png", clock.tick);
            fs::write(output.join(&file), png)?;
            frames.push(Frame {
                tick: clock.tick,
                elapsed_seconds: clock.elapsed_seconds,
                file,
                geometry: scene.stats().clone(),
                shading: scene.shading(),
            });
        }
    }
    let passed = assertions.passed();
    let report = Report {
        format_version: 1,
        completed: true,
        passed,
        assertions: assertions.results(),
        ticks: count,
        start_tick,
        save_output: options.save_output.filter(|_| passed),
        script_commands: commands,
        state: play.snapshot(),
        script_state: play.host.state().clone(),
        input: play.input().clone(),
        audio: audio.map(super::play_audio::Output::finish).transpose()?,
        frames,
        logs: logs.finish()?,
        adapter: renderer.as_ref().map(|r| r.adapter_name.clone()),
        width: renderer.as_ref().map(|_| options.width),
        height: renderer.as_ref().map(|_| options.height),
        camera: options.camera,
        wall_ms: start.elapsed().as_secs_f64() * 1000.,
    };
    if let Some(save_output) = save_output.filter(|_| passed) {
        save_output.finish(&play)?;
    }
    if let Some(output) = options.output {
        let mut file = tempfile::NamedTempFile::new_in(&output)?;
        file.write_all(&serde_json::to_vec_pretty(&report)?)?;
        file.as_file().sync_all()?;
        file.persist(output.join("report.json"))
            .map_err(|error| error.error)?;
    }
    Ok(report)
}
