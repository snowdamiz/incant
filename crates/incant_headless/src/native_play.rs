//! Development project play on the native graph. Project validation/cooking and
//! binary loading precede simulation. Authored TypeScript uses a fixed trusted
//! startup cook; scenes without attachments use the native build's embedded
//! behavior. Simulation has no source parsing or authoring dependency.
use incant_input::InputReplay;
use incant_observe::Plan;
pub(crate) mod embedded {
    include!(concat!(env!("OUT_DIR"), "/behavior.rs"));
}
use incant_agent_native::{Session, StartOptions};
use incant_render_data::{
    material_keys::MaterialSamples,
    prewarm::{MaterialProfile, MaterialProgram},
};
use serde::Serialize;
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
    /// Logical UI viewport WIDTHxHEIGHT, requires an explicit camera.
    #[arg(long, value_parser = viewport)]
    pub ui_viewport: Option<[f32; 2]>,
    #[arg(long,default_value_t=48000,value_parser=clap::value_parser!(u32).range(8000..=192000))]
    pub sample_rate: u32,
    /// New raw stereo f32-le PCM file; published only after a completed run.
    #[arg(long)]
    pub pcm_output: Option<PathBuf>,
    /// Create a bounded ModelMaterialV1 manifest after successful native gameplay.
    /// This observes CPU selections; it does not create or measure GPU pipelines.
    #[arg(long, requires = "camera")]
    pub record_materials: Option<PathBuf>,
    /// Trusted target program profile for the logical recording.
    #[arg(long, requires = "record_materials", value_enum)]
    pub material_program: Option<Program>,
    /// Target world sampling policy, 1 or 4. Pixel-perfect cameras select 1.
    #[arg(long, requires = "record_materials")]
    pub material_samples: Option<u32>,
}
#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Program {
    Wgsl,
    Cooked,
}
pub(crate) fn viewport(value: &str) -> std::result::Result<[f32; 2], &'static str> {
    let (width, height) = value.split_once('x').ok_or("expected WIDTHxHEIGHT")?;
    let v = [
        width.parse::<f32>().map_err(|_| "invalid UI width")?,
        height.parse::<f32>().map_err(|_| "invalid UI height")?,
    ];
    if v.iter()
        .any(|n| !n.is_finite() || !(1.0..=65_536.0).contains(n))
    {
        return Err("UI viewport must be within 1..=65536 logical pixels");
    }
    Ok(v)
}

#[derive(Debug, thiserror::Error)]
pub enum PlayError {
    #[error("native play: {0}")]
    Invalid(&'static str),
    #[error("native play IO: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Session(#[from] incant_agent_native::SessionError),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    material_recording: Option<serde_json::Value>,
}

pub fn run(options: Options) -> Result<Report> {
    if !(8000..=192000).contains(&options.sample_rate) {
        return Err(invalid("invalid sample rate"));
    }
    let material_profile = if let Some(path) = &options.record_materials {
        if path.exists() {
            return Err(invalid("material manifest output already exists"));
        }
        if options.camera.is_none() {
            return Err(invalid("material recording requires a camera"));
        }
        Some(MaterialProfile {
            program: match options.material_program.unwrap_or(Program::Wgsl) {
                Program::Wgsl => MaterialProgram::Wgsl,
                Program::Cooked => MaterialProgram::Cooked,
            },
            samples: match options.material_samples.unwrap_or(1) {
                1 => MaterialSamples::One,
                4 => MaterialSamples::Four,
                _ => return Err(invalid("material sampling supports only 1 or 4")),
            },
        })
    } else {
        if options.material_program.is_some() || options.material_samples.is_some() {
            return Err(invalid("material target options require recording"));
        }
        None
    };
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
    let mut input_sha256: [u8; 32] = Sha256::digest([]).into();
    let replay = options
        .input_replay
        .as_deref()
        .map(|path| {
            let bytes = bounded(path, incant_input::MAX_RECORDING_BYTES)?;
            input_sha256 = Sha256::digest(&bytes).into();
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
    let start_options = StartOptions {
        scene: options.scene.clone(),
        script_sha256: options.script_sha256.clone(),
        camera: options.camera.clone(),
        ui_viewport: options.ui_viewport,
    };
    let mut game = if let Some(profile) = material_profile {
        Session::new_recording_materials(
            &document,
            root,
            &start_options,
            options.sample_rate,
            embedded::load,
            profile,
            input_sha256,
        )?
    } else {
        Session::new(
            &document,
            root,
            &start_options,
            options.sample_rate,
            embedded::load,
        )?
    };
    game.validate_ui_recording(replay.as_ref().is_some_and(InputReplay::has_ui))?;
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
        game.tick_recorded(
            events,
            replay
                .as_ref()
                .map(|r| r.ui_at(tick))
                .transpose()
                .map_err(native)?
                .unwrap_or(&[]),
            replay
                .as_ref()
                .map(|r| r.viewport_at(tick))
                .transpose()
                .map_err(native)?
                .flatten(),
            output,
        )?;
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
    let mut report = Report {
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
        material_recording: None,
    };
    if let Some(path) = &options.record_materials {
        if report.passed {
            let manifest = game
                .material_manifest()?
                .ok_or(invalid("material recorder is absent"))?;
            let bytes = manifest.to_bytes().map_err(native)?;
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let mut output = tempfile::NamedTempFile::new_in(parent)?;
            output.write_all(&bytes)?;
            output.as_file().sync_all()?;
            output
                .persist_noclobber(path)
                .map_err(|e| PlayError::Io(e.error))?;
            report.material_recording = Some(
                serde_json::json!({"completed":true,"scope":"model_material_v1","sha256":format!("{:x}",Sha256::digest(&bytes)),"bytes":bytes.len(),"observed_keys":manifest.observed.len(),"first_tick":manifest.recording.first_tick,"last_tick":manifest.recording.last_tick}),
            );
        } else {
            report.material_recording = Some(
                serde_json::json!({"completed":false,"reason":"observation assertions failed"}),
            );
        }
    }
    if let Some(pcm) = pcm {
        pcm.as_file().sync_all()?;
        pcm.persist_noclobber(options.pcm_output.as_ref().expect("requested output"))
            .map_err(|e| PlayError::Io(e.error))?;
    }
    Ok(report)
}
