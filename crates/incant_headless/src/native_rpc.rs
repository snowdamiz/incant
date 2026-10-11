//! Retained native Play for the trusted editor/headless RPC boundary. Only start
//! cooks authoring; stepping and selected observation use the disposable world.
use crate::native_play::{Session, StartOptions};
use incant_cmd::CommandBus;
use incant_doc::{Transform, Velocity};
use incant_input::{InputEvent, InputFrame};
use incant_runtime::StableId;
use incant_script::NativeLogs;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};
mod run;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("native play interrupted")]
    Interrupted,
    #[error("native play exceeded its 10 second host execution budget")]
    Deadline,
    #[error("native play RPC: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Decode(#[from] serde_json::Error),
    #[error(transparent)]
    Native(#[from] crate::native_play::PlayError),
}
type Result<T> = std::result::Result<T, Error>;

const MAX_SELECTED_ENTITIES: usize = 256;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    scene: Option<String>,
    camera: Option<String>,
    script_sha256: Option<String>,
    #[serde(default = "default_sample_rate")]
    sample_rate: u32,
}
fn default_sample_rate() -> u32 {
    48_000
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Observation {
    entities: Vec<String>,
    include_input: bool,
    include_logs: bool,
}
impl Observation {
    fn selected(&self) -> Result<Vec<(String, StableId)>> {
        if self.entities.len() > MAX_SELECTED_ENTITIES {
            return Err(Error::Invalid("select at most 256 entities"));
        }
        let mut seen = std::collections::BTreeSet::new();
        self.entities
            .iter()
            .map(|text| {
                let id = text
                    .parse::<ulid::Ulid>()
                    .map_err(|_| Error::Invalid("expected canonical uppercase ULID"))?;
                if id.to_string() != *text {
                    return Err(Error::Invalid("expected canonical uppercase ULID"));
                }
                if !seen.insert(id) {
                    return Err(Error::Invalid("duplicate selected entity"));
                }
                Ok((text.clone(), StableId(id.to_bytes())))
            })
            .collect()
    }
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Step {
    events: Vec<InputEvent>,
    observe: Observation,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

#[derive(Serialize)]
struct EntityState {
    parent: Option<String>,
    transform: Transform,
    velocity: Option<Velocity>,
    world_transform: [[f64; 4]; 4],
}
#[derive(Serialize)]
struct State<'a> {
    runtime: &'static str,
    playing: bool,
    started_revision: u64,
    scene: &'a str,
    script_source_sha256: &'a str,
    script_source_parsing: bool,
    tick: u64,
    tick_rate: u32,
    entity_count: usize,
    frames: u128,
    cpu_render_input: bool,
    gpu_submitted: bool,
    /// Missing/despawned selected IDs have null values; this is never all-world enumeration.
    entities: BTreeMap<String, Option<EntityState>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<&'a InputFrame>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logs: Option<&'a NativeLogs>,
}

struct Live {
    session: Session,
    started_revision: u64,
    samples: Vec<f32>,
}
impl Live {
    fn state(&self, observation: &Observation, selected: Vec<(String, StableId)>) -> Result<Value> {
        // Failed sessions reject observations as well as updates; stale committed
        // output must not masquerade as a successful live world after a failure.
        let world = self.session.world()?;
        let entities = selected
            .into_iter()
            .map(|(text, id)| {
                let state = world.inspect(id).map(|s| EntityState {
                    parent: s.parent.map(|p| ulid::Ulid::from_bytes(p.0).to_string()),
                    transform: s.transform,
                    velocity: s.velocity,
                    world_transform: s.world_transform,
                });
                (text, state)
            })
            .collect();
        Ok(serde_json::to_value(State {
            runtime: "native",
            playing: true,
            started_revision: self.started_revision,
            scene: &self.session.scene,
            script_source_sha256: &self.session.script_source_sha256,
            script_source_parsing: self.session.authored_scripts,
            tick: world.tick(),
            tick_rate: world.tick_rate(),
            entity_count: world.entity_count(),
            frames: self.session.frames()?,
            cpu_render_input: self.session.is_spatial(),
            gpu_submitted: false,
            entities,
            input: observation
                .include_input
                .then(|| self.session.input())
                .transpose()?,
            logs: observation
                .include_logs
                .then(|| self.session.logs())
                .transpose()?,
        })?)
    }
}

#[derive(Default)]
pub(crate) struct Controller {
    live: Option<Live>,
}
impl Controller {
    pub fn start(&mut self, bus: &CommandBus, root: &Path, params: Value) -> Result<Value> {
        let request: Start = serde_json::from_value(params)?;
        if self.live.is_some() {
            return Err(Error::Invalid(
                "stop the existing play session before starting another",
            ));
        }
        let session = Session::new(
            bus.project(),
            root,
            &StartOptions {
                scene: request.scene,
                camera: request.camera,
                script_sha256: request.script_sha256,
            },
            request.sample_rate,
        )?;
        let max_frames =
            (u64::from(request.sample_rate) / u64::from(session.world()?.tick_rate()) + 1) as usize;
        let live = Live {
            session,
            started_revision: bus.revision(),
            samples: vec![0.; max_frames * 2],
        };
        let result = live.state(&Observation::default(), Vec::new())?;
        self.live = Some(live);
        Ok(result)
    }
    pub fn step(&mut self, params: Value) -> Result<Value> {
        let request: Step = serde_json::from_value(params)?;
        // Validate observation selection before accepting input or advancing time.
        let selected = request.observe.selected()?;
        let live = self
            .live
            .as_mut()
            .ok_or(Error::Invalid("no play session"))?;
        let frames = live.session.next_frames()?;
        live.session
            .tick(&request.events, &mut live.samples[..frames * 2])?;
        live.state(&request.observe, selected)
    }
    pub fn state(&self, params: Value) -> Result<Value> {
        let observation: Observation = serde_json::from_value(params)?;
        let selected = observation.selected()?;
        self.live
            .as_ref()
            .ok_or(Error::Invalid("no play session"))?
            .state(&observation, selected)
    }
    pub fn stop(&mut self, params: Value) -> Result<()> {
        let _: Empty = serde_json::from_value(params)?;
        self.live = None;
        Ok(())
    }
}
