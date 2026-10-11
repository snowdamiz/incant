//! Development project play on the native graph. Project validation/cooking and
//! binary loading precede simulation. Scripts are embedded by the trusted cook
//! during the native build; playback never reads or parses script source.
use incant_audio_runtime::{ClipBinding, MAX_SCENE_CLIPS};
use incant_game_runtime::{HostedOfflineGameSession, OfflineGameSession};
use incant_input::{InputActions, InputEvent, InputFrame, InputReplay};
use incant_observe::Plan;
use incant_render_game::{NativeRenderHost, NativeRenderSettings};
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_script::{CompiledScript, NativeLogs};
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/behavior.rs"));
}
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
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
fn id(text: &str) -> Result<StableId> {
    let value = text.parse::<ulid::Ulid>().map_err(native)?;
    if value.to_string() != text {
        return Err(invalid("expected canonical uppercase ULID"));
    }
    Ok(StableId(value.to_bytes()))
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

enum Game {
    Plain(Box<OfflineGameSession>),
    Spatial(Box<HostedOfflineGameSession<NativeRenderHost>>),
}
impl Game {
    fn new(
        scene: CookedScene,
        code: CompiledScript,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        rate: u32,
        settings: Option<NativeRenderSettings>,
    ) -> Result<Self> {
        if let Some(settings) = settings {
            Ok(Self::Spatial(Box::new(
                HostedOfflineGameSession::new(scene, code, actions, clips, rate, settings)
                    .map_err(native)?,
            )))
        } else {
            Ok(Self::Plain(Box::new(
                OfflineGameSession::new(scene, code, actions, clips, rate).map_err(native)?,
            )))
        }
    }
    fn world(&self) -> Result<&NativeWorld> {
        match self {
            Self::Plain(g) => g.world().map_err(native),
            Self::Spatial(g) => g.world().map_err(native),
        }
    }
    fn input(&self) -> Result<&InputFrame> {
        match self {
            Self::Plain(g) => g.input().map_err(native),
            Self::Spatial(g) => g.input().map_err(native),
        }
    }
    fn logs(&self) -> Result<&NativeLogs> {
        match self {
            Self::Plain(g) => g.logs().map_err(native),
            Self::Spatial(g) => g.logs().map_err(native),
        }
    }
    fn next_frames(&self) -> Result<usize> {
        match self {
            Self::Plain(g) => g.next_output_frames().map_err(native),
            Self::Spatial(g) => g.next_output_frames().map_err(native),
        }
    }
    fn frames(&self) -> Result<u128> {
        match self {
            Self::Plain(g) => g.rendered_frames().map_err(native),
            Self::Spatial(g) => g.rendered_frames().map_err(native),
        }
    }
    fn tick(&mut self, input: &[InputEvent], output: &mut [f32]) -> Result<()> {
        match self {
            Self::Plain(g) => g.tick(input, output).map_err(native),
            Self::Spatial(g) => g.tick(input, output).map_err(native),
        }
    }
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
    let scene = match &options.scene {
        Some(value) => value.clone(),
        None if document.scenes.len() == 1 => {
            document.scenes.keys().next().expect("one scene").clone()
        }
        None => {
            return Err(invalid(
                "choose --scene when the project does not have exactly one scene",
            ));
        }
    };
    id(&scene)?;
    let end = ticks(&options, document.settings.tick_rate)?;
    let frame_count =
        u128::from(end) * u128::from(options.sample_rate) / u128::from(document.settings.tick_rate);
    if options.pcm_output.is_some() && frame_count * 8 > 512 * 1024 * 1024 {
        return Err(invalid("PCM output exceeds 512 MiB"));
    }
    let camera = options.camera.as_deref().map(id).transpose()?;
    let cooked = {
        let bytes = incant_cook::cook_scene(&document, &scene).map_err(native)?;
        CookedScene::from_bytes(&bytes).map_err(native)?
    };
    if camera.is_none() && (!cooked.spatial().is_empty() || !cooked.visual_assets().is_empty()) {
        return Err(invalid(
            "spatial content requires an explicit --camera and native render host",
        ));
    }
    let root = options
        .project
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let clip_ids: BTreeSet<_> = cooked
        .audio()
        .iter()
        .filter_map(|n| n.source.as_ref().map(|s| s.clip))
        .collect();
    if clip_ids.len() > MAX_SCENE_CLIPS {
        return Err(invalid("too many scene clips"));
    }
    let settings = camera
        .map(|camera| {
            let path = |kind, asset_kind, suffix| {
                if cooked.visual_assets().iter().any(|a| a.kind == asset_kind) {
                    incant_assets::project_cache_directory(root, kind).map_err(native)
                } else {
                    Ok(root.join(suffix))
                }
            };
            Ok::<_, PlayError>(NativeRenderSettings {
                model_cache: path(
                    incant_assets::CacheKind::Models,
                    incant_runtime::VisualAssetKind::Model,
                    ".incant/cache/models",
                )?,
                texture_cache: path(
                    incant_assets::CacheKind::Textures,
                    incant_runtime::VisualAssetKind::Texture,
                    ".incant/cache/textures",
                )?,
                camera,
                consumer: (),
            })
        })
        .transpose()?;
    let audio_cache = if clip_ids.is_empty() {
        None
    } else {
        Some(
            incant_assets::project_cache_directory(root, incant_assets::CacheKind::Audio)
                .map_err(native)?,
        )
    };
    let mut clips = Vec::with_capacity(clip_ids.len());
    for clip in clip_ids {
        let key = ulid::Ulid::from_bytes(clip.0).to_string();
        let asset = document
            .assets
            .get(&key)
            .ok_or_else(|| invalid("missing audio asset"))?;
        if asset.kind != "audio" {
            return Err(invalid("clip reference is not an audio asset"));
        }
        clips.push(
            ClipBinding::load(
                clip,
                audio_cache.as_deref().expect("referenced clips"),
                &asset.sha256,
            )
            .map_err(native)?,
        );
    }
    let code = embedded::load().map_err(native)?;
    let script_source_sha256 = code
        .metadata()
        .source_sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if options
        .script_sha256
        .as_ref()
        .is_some_and(|expected| expected != &script_source_sha256)
    {
        return Err(invalid(
            "embedded script does not match requested source hash",
        ));
    }
    let replay = options
        .input_replay
        .as_deref()
        .map(|path| {
            let bytes = bounded(path, incant_input::MAX_RECORDING_BYTES)?;
            let replay = InputReplay::from_text(
                std::str::from_utf8(&bytes).map_err(native)?,
                0,
                cooked.tick_rate(),
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
    let actions = document.settings.input_actions.clone();
    // Authoring data and binary startup image are gone before any guest update.
    drop(document);
    let mut game = Game::new(cooked, code, actions, clips, options.sample_rate, settings)?;
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
        script_source_sha256,
        script_source_parsing: false,
        scene,
        tick: world.tick(),
        tick_rate: world.tick_rate(),
        entity_count: world.entity_count(),
        frames: game.frames()?,
        pcm_sha256: format!("{:x}", hash.finalize()),
        logs: serde_json::to_value(game.logs()?).map_err(native)?,
        observations,
        cpu_render_input: matches!(game, Game::Spatial(_)),
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
