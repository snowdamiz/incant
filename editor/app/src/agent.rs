//! Native editor agent transport. Credentials and provider calls stay on the
//! worker; authoring edits use the editor's existing bus and journal.
mod owner;
#[cfg(test)]
mod tests;
mod worker;
use incant_agent::{Budget, Conversation, session::*};
use serde::Serialize;
use std::sync::{Arc, Mutex};

/// An account registration, never a credential. Also scopes in-memory context
/// so changing accounts cannot send the previous account's conversation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Connection {
    OAuth(String),
    ApiKey,
}
impl Connection {
    pub(super) fn from_public(value: &serde_json::Value) -> Option<Self> {
        if value["status"] != "connected" {
            return None;
        }
        match value["method"].as_str()? {
            "oauth" => Some(Self::OAuth(value["activeAccount"].as_str()?.to_owned())),
            "api-key" => Some(Self::ApiKey),
            _ => None,
        }
    }
    fn from_store(store: &incant_agent::accounts::AccountStore) -> Option<Self> {
        store
            .selected()
            .map(|a| Self::OAuth(a.id.clone()))
            .or_else(|| store.data.api_key_connected.then_some(Self::ApiKey))
    }
}
#[derive(Clone, PartialEq, Eq)]
enum ContextKey {
    Account(incant_agent::accounts::AccountSelection),
    #[cfg(test)]
    Fixture(Connection),
}
#[derive(Default)]
struct Context {
    conversation: Conversation,
}
pub(super) struct Service {
    session: Session,
    owner: owner::Owner,
    // Serializes starts with account operations and shutdown, never provider IO.
    closed: Mutex<bool>,
}
impl Service {
    pub(super) fn new() -> Self {
        Self {
            session: Session::new(Budget {
                max_tokens: 1_000_000,
                used_tokens: 0,
            }),
            owner: owner::Owner::new(),
            closed: Mutex::new(false),
        }
    }
    fn start(
        &self,
        request: TurnRequest,
        ready: impl FnOnce() -> Result<Connection, Error>,
        job: impl FnOnce(TurnContext, Connection, owner::Client) -> TurnOutcome + Send + 'static,
    ) -> Result<TurnId, Error> {
        let closed = self.closed.lock().map_err(|_| Error::unavailable())?;
        if *closed {
            return Err(Error::unavailable());
        }
        let connection = ready()?;
        let owner = self.owner.client();
        self.session
            .start(request, move |turn| job(turn, connection, owner))
            .map_err(Into::into)
    }
    fn cancel_active(&self) {
        if let Ok(snapshot) = self.session.snapshot()
            && snapshot.worker_running
            && let Some(turn) = snapshot.turns.last()
        {
            let _ = self.session.cancel(turn.id);
        }
    }
    /// Called before account mutations start, also used for changed restore
    /// results. Taking this gate prevents a concurrently accepted turn escaping
    /// cancellation. The closure must only schedule work, never do network IO.
    pub(super) fn account_operation<T>(&self, schedule: impl FnOnce() -> T) -> T {
        let _gate = self.closed.lock().unwrap_or_else(|e| e.into_inner());
        self.cancel_active();
        schedule()
    }
    pub(super) fn shutdown(&self) {
        let mut closed = self.closed.lock().unwrap_or_else(|e| e.into_inner());
        *closed = true;
        self.cancel_active();
        self.owner.close();
    }
}
#[derive(Debug, Serialize)]
pub(super) struct Error {
    code: &'static str,
    message: &'static str,
}
impl Error {
    fn unavailable() -> Self {
        Self {
            code: "agent.unavailable",
            message: "The agent is unavailable. Check the project and account connection.",
        }
    }
}
impl From<SessionError> for Error {
    fn from(error: SessionError) -> Self {
        let (code, message) = match error {
            SessionError::Busy => ("agent.busy", "The previous agent turn is still finishing."),
            SessionError::StaleTurn => ("agent.stale-turn", "This agent turn is no longer active."),
            SessionError::StaleApproval => (
                "agent.stale-approval",
                "This approval is no longer pending.",
            ),
            SessionError::InvalidRequest => (
                "agent.invalid-request",
                "Enter a bounded prompt and model name.",
            ),
            SessionError::Budget => (
                "agent.budget",
                "The agent session token allowance is exhausted.",
            ),
            _ => return Self::unavailable(),
        };
        Self { code, message }
    }
}
#[derive(Serialize)]
pub(super) struct Read {
    session: Option<SessionSnapshot>,
    project_revision: Option<u64>,
}
fn read(
    service: &Service,
    bus: &Mutex<super::project::LoadState>,
    after: Option<u64>,
) -> Result<Read, Error> {
    let session = service.session.snapshot_if_changed(after)?;
    let load = bus.lock().map_err(|_| Error::unavailable())?;
    let project_revision = match &*load {
        super::project::LoadState::Ready(bus) => Some(bus.revision()),
        _ => None,
    };
    Ok(Read {
        session,
        project_revision,
    })
}
#[tauri::command]
pub(super) fn engine_agent_read(
    state: tauri::State<'_, Arc<super::Editor>>,
    after_version: Option<u64>,
) -> Result<Read, Error> {
    read(&state.agent, &state.bus, after_version)
}
#[tauri::command]
pub(super) fn engine_agent_start(
    state: tauri::State<'_, Arc<super::Editor>>,
    request: TurnRequest,
) -> Result<TurnId, Error> {
    let bus = state.bus.clone();
    let root = state.asset_root.clone();
    state.agent.start(
        request,
        || {
            let connection = state.provider.connection().ok_or_else(Error::unavailable)?;
            if !matches!(
                &*state.bus.lock().map_err(|_| Error::unavailable())?,
                super::project::LoadState::Ready(_)
            ) {
                return Err(Error::unavailable());
            }
            Ok(connection)
        },
        move |turn, connection, owner| {
            owner.run(turn, move |state, turn| {
                worker::run(turn, connection, state, bus, root)
            })
        },
    )
}
#[tauri::command]
pub(super) fn engine_agent_cancel(
    state: tauri::State<'_, Arc<super::Editor>>,
    turn_id: TurnId,
) -> Result<(), Error> {
    state.agent.session.cancel(turn_id).map_err(Into::into)
}
#[tauri::command]
pub(super) fn engine_agent_decide(
    state: tauri::State<'_, Arc<super::Editor>>,
    turn_id: TurnId,
    approval_id: ApprovalId,
    approved: bool,
) -> Result<(), Error> {
    state
        .agent
        .session
        .decide(turn_id, approval_id, approved)
        .map_err(Into::into)
}
