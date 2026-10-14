//! Native Play transport. Cooking runs on a blocking authoring worker; all live
//! VM, audio and GPU state stays on the render thread. No per-tick document read.
mod clock;
mod logs;
#[cfg(feature = "play-probe")]
pub mod probe;
#[cfg(test)]
mod tests;
mod texture_refresh;
mod worker;
use crate::{Editor, project};
use incant_input::InputEvent;
use incant_play::{PreparedPlay, StartOptions};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
pub use texture_refresh::schedule_refresh;
pub use worker::Worker;

const UNSAVED_PROJECT: &str = "Open a saved project to run Play";

pub fn start_availability(saved: bool) -> Value {
    if saved {
        serde_json::json!({"available": true})
    } else {
        serde_json::json!({"available": false, "reason": UNSAVED_PROJECT})
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub generation: u32,
    pub sequence: u64,
    pub phase: &'static str,
    pub tick: u64,
    pub discarded_ticks: u64,
    pub presented_frames: u64,
    pub scene: Option<String>,
    pub source_revision: Option<u64>,
    pub spatial: bool,
    pub suspended: bool,
    pub error: Option<String>,
    pub preparing: bool,
    pub logs: logs::Summary,
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Start {
        generation: u32,
        expected_revision: u64,
        #[serde(default)]
        options: StartOptions,
    },
    Pause {
        generation: u32,
    },
    Resume {
        generation: u32,
    },
    Step {
        generation: u32,
    },
    Stop {
        generation: u32,
    },
    Inspect {
        generation: u32,
        entity: String,
    },
    Input {
        generation: u32,
        events: Vec<InputEvent>,
    },
}
impl Action {
    fn generation(&self) -> u32 {
        match self {
            Self::Start { generation, .. }
            | Self::Pause { generation }
            | Self::Resume { generation }
            | Self::Step { generation }
            | Self::Stop { generation }
            | Self::Inspect { generation, .. }
            | Self::Input { generation, .. } => *generation,
        }
    }
}
pub struct Service {
    tx: mpsc::SyncSender<Message>,
    status: Mutex<State>,
    loading: AtomicBool,
    basis: Mutex<Option<(u32, Arc<incant_play::TextureRefreshBasis>)>>,
    refreshing: AtomicBool,
    refresh_attempt: Mutex<Option<(u32, u64)>>,
    refresh_event: AtomicBool,
}
struct State {
    status: Status,
    logs: logs::Store,
}
pub struct Message {
    request: Request,
    reply: mpsc::Sender<Result<Value, String>>,
}
enum Request {
    Load {
        generation: u32,
        revision: u64,
        prepared: Box<PreparedPlay>,
    },
    Action(Action),
    TextureTicket {
        generation: u32,
        reply: mpsc::Sender<Result<incant_render_game::TextureRefreshTicket, String>>,
    },
    StageTextures {
        generation: u32,
        revision: u64,
        prepared: incant_render_game::PreparedTextureRefresh,
    },
    PublishTextures {
        generation: u32,
        revision: u64,
        token: u64,
        bus: Arc<Mutex<project::LoadState>>,
    },
    CancelTextures {
        generation: u32,
        token: u64,
    },
}
impl Service {
    pub fn new() -> (Arc<Self>, mpsc::Receiver<Message>) {
        let (tx, rx) = mpsc::sync_channel(16);
        (
            Arc::new(Self {
                tx,
                loading: AtomicBool::new(false),
                basis: Mutex::new(None),
                refreshing: AtomicBool::new(false),
                refresh_attempt: Mutex::new(None),
                refresh_event: AtomicBool::new(false),
                status: Mutex::new(State {
                    status: Status {
                        generation: 0,
                        sequence: 0,
                        phase: "stopped",
                        tick: 0,
                        discarded_ticks: 0,
                        presented_frames: 0,
                        scene: None,
                        source_revision: None,
                        spatial: false,
                        suspended: false,
                        error: None,
                        preparing: false,
                        logs: logs::Summary::default(),
                    },
                    logs: logs::Store::default(),
                }),
            }),
            rx,
        )
    }
    pub fn snapshot(&self) -> Result<Status, String> {
        self.status
            .lock()
            .map(|s| s.status.clone())
            .map_err(|_| "Play state is unavailable".into())
    }
    pub fn take_refresh_event(&self) -> bool {
        self.refresh_event.swap(false, Ordering::AcqRel)
    }
    fn edit<T>(
        &self,
        operation: impl FnOnce(&mut Status) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut state = self
            .status
            .lock()
            .map_err(|_| "Play state is unavailable")?;
        let next = state
            .status
            .sequence
            .checked_add(1)
            .ok_or("Play status counter exhausted")?;
        let generation = state.status.generation;
        let result = operation(&mut state.status)?;
        if generation != state.status.generation {
            state.logs.reset();
            state.status.logs = logs::Summary::default();
        }
        state.status.sequence = next;
        Ok(result)
    }
    // Called only after the actual game operation has accepted its script update.
    // One mutex makes the prefix/count and status publication indivisible.
    fn publish_logs(
        &self,
        generation: u32,
        logs: &incant_runtime::logging::NativeLogs,
        operation: impl FnOnce(&mut Status),
    ) -> Result<(), String> {
        let mut state = self
            .status
            .lock()
            .map_err(|_| "Play state is unavailable")?;
        if state.status.generation != generation
            || !matches!(state.status.phase, "running" | "paused")
        {
            return Err("Play session changed".into());
        }
        let next = state
            .status
            .sequence
            .checked_add(1)
            .ok_or("Play status counter exhausted")?;
        state.logs.append(logs).map_err(|e| e.to_string())?;
        state.status.logs = state.logs.summary();
        operation(&mut state.status);
        state.status.sequence = next;
        Ok(())
    }
    fn read_logs(&self, generation: u32, after_index: usize) -> Result<logs::Batch, String> {
        let state = self
            .status
            .lock()
            .map_err(|_| "Play state is unavailable")?;
        if state.status.generation != generation
            || !matches!(state.status.phase, "running" | "paused")
        {
            return Err("Play logs are unavailable for this session".into());
        }
        state
            .logs
            .read(generation, after_index)
            .map_err(|e| e.to_string())
    }
    fn call(&self, request: Request) -> Result<Value, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .try_send(Message { request, reply: tx })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => "Play request queue is full; retry shortly",
                mpsc::TrySendError::Disconnected(_) => "Native Play worker has stopped",
            })?;
        rx.recv().map_err(|_| "Native Play worker has stopped")?
    }
    fn failed(&self, generation: u32, error: String) {
        let _ = self.edit(|s| {
            if s.generation == generation {
                s.phase = "error";
                s.error = Some(error);
                s.suspended = false;
            }
            Ok(())
        });
    }
}
struct Loading<'a>(&'a Service);
impl Drop for Loading<'_> {
    fn drop(&mut self) {
        let _ = self.0.edit(|s| {
            s.preparing = false;
            Ok(())
        });
        self.0.loading.store(false, Ordering::Release);
    }
}
#[tauri::command]
pub fn play_read(state: tauri::State<'_, Arc<Editor>>) -> Result<Status, String> {
    state.play.snapshot()
}

#[tauri::command]
pub fn play_logs_read(
    state: tauri::State<'_, Arc<Editor>>,
    generation: u32,
    after_index: usize,
) -> Result<logs::Batch, String> {
    state.play.read_logs(generation, after_index)
}

#[tauri::command]
pub async fn play_command(
    state: tauri::State<'_, Arc<Editor>>,
    command: Action,
) -> Result<Value, String> {
    let editor = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || execute(&editor, command))
        .await
        .map_err(|e| e.to_string())?
}
fn execute(editor: &Editor, command: Action) -> Result<Value, String> {
    let Action::Start {
        generation,
        expected_revision,
        options,
    } = command
    else {
        if matches!(&command,Action::Input {events,..} if events.len()>128) {
            return Err("Input packet exceeds 128 events".into());
        }
        return editor.play.call(Request::Action(command));
    };
    if editor
        .play
        .loading
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("A previous Play startup is still finishing".into());
    }
    let _loading = Loading(&editor.play);
    let next = editor.play.edit(|s| {
        if s.generation != generation {
            return Err("Play session changed; retry from current state".into());
        }
        if !matches!(s.phase, "stopped" | "error") {
            return Err("Stop the current Play session before starting another".into());
        }
        s.generation = s
            .generation
            .checked_add(1)
            .ok_or("Play generation exhausted")?;
        s.phase = "preparing";
        s.error = None;
        s.preparing = true;
        s.tick = 0;
        s.discarded_ticks = 0;
        s.presented_frames = 0;
        s.scene = None;
        s.source_revision = None;
        Ok(s.generation)
    })?;
    let result = (|| {
        let root = editor.asset_root.as_deref().ok_or(UNSAVED_PROJECT)?;
        let document = {
            let load = editor.bus.lock().map_err(|_| "Engine lock failed")?;
            let project::LoadState::Ready(bus) = &*load else {
                return Err("Project is not ready".into());
            };
            if bus.revision() != expected_revision {
                return Err("Project changed; retry Play from the latest revision".into());
            }
            bus.project().clone()
        };
        let prepared = PreparedPlay::load(&document, root, &options, 48000, || {
            incant_script::CompiledScript::compile("exports.default={update(){}};")
        })
        .map_err(|e| e.to_string())?;
        let basis = Arc::new(
            incant_play::TextureRefreshBasis::capture(&document, prepared.cooked.visual_assets())
                .map_err(|e| e.to_string())?,
        );
        drop(document);
        {
            let load = editor.bus.lock().map_err(|_| "Engine lock failed")?;
            if !matches!(&*load,project::LoadState::Ready(bus) if bus.revision()==expected_revision)
            {
                return Err("Project changed during Play startup; retry".into());
            }
        }
        let result = editor.play.call(Request::Load {
            generation: next,
            revision: expected_revision,
            prepared: Box::new(prepared),
        })?;
        *editor
            .play
            .basis
            .lock()
            .map_err(|_| "Play authoring basis is unavailable")? = Some((next, basis));
        *editor
            .play
            .refresh_attempt
            .lock()
            .map_err(|_| "Play refresh state is unavailable")? = Some((next, expected_revision));
        Ok(result)
    })();
    if let Err(error) = &result {
        editor.play.failed(next, error.clone());
    }
    result
}
