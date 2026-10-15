use crate::{ApprovalMode, Budget, ToolObservation, TurnReport, Usage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const MAX_PROMPT_BYTES: usize = 16 * 1024;
pub const MAX_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_APPROVAL_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_APPROVAL_TOTAL_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_APPROVALS: usize = 32;
pub const MAX_HISTORY: usize = 16;
pub const MAX_METADATA_BYTES: usize = 128;
pub(super) const MAX_SEQUENCE: u64 = (1 << 53) - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TurnId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApprovalId(pub u64);
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalChoice {
    Auto,
    Destructive,
    Always,
}
impl ApprovalChoice {
    pub fn mode(self) -> ApprovalMode {
        match self {
            Self::Auto => ApprovalMode::Auto,
            Self::Destructive => ApprovalMode::Destructive,
            Self::Always => ApprovalMode::Always,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnRequest {
    pub prompt: String,
    /// Selected provider model, not credentials or provider configuration.
    pub model: String,
    pub approval: ApprovalChoice,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_context: Option<crate::EditorViewContext>,
}
pub struct TurnOutcome {
    pub budget: Budget,
    pub result: Result<TurnReport, crate::AgentError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    Provider,
    Tool,
    BudgetExhausted,
    /// The next provider input exceeds the bounded conversation context.
    ContextLimit,
    StepLimit,
    ProjectUnavailable,
    ProjectChanged,
    WorkerPanicked,
    WorkerSpawn,
    BudgetAccounting,
    OutputLimit,
    ApprovalCountLimit,
    ApprovalPayloadLimit,
    ApprovalTotalLimit,
    ApprovalInvalid,
    ApprovalAlreadyPending,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Running,
    AwaitingApproval,
    Stopping,
    Completed,
    Cancelled,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Pending,
    Allowed,
    Denied,
    Cancelled,
}
#[derive(Debug, Clone, Serialize)]
pub struct ApprovalSummary {
    pub id: ApprovalId,
    pub tool: String,
    pub bytes: usize,
    pub decision: Decision,
}
#[derive(Debug, Clone, Serialize)]
pub struct PendingApproval {
    pub turn: TurnId,
    pub id: ApprovalId,
    pub tool: String,
    pub arguments: Value,
}
#[derive(Debug, Clone, Serialize)]
pub struct ToolSummary {
    /// None means the supplied metadata exceeded the controller's bound.
    pub name: Option<String>,
    pub succeeded: bool,
    pub selected_camera: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ReportSummary {
    pub steps: usize,
    pub usage: Usage,
    pub transaction_count: usize,
    pub tool_call_count: usize,
    pub transaction_ids: Vec<String>,
    pub tool_calls: Vec<ToolSummary>,
    pub truncated: bool,
}
impl From<TurnReport> for ReportSummary {
    fn from(r: TurnReport) -> Self {
        const MAX_ROWS: usize = 128;
        let mut truncated = r.transaction_ids.len() > MAX_ROWS || r.tool_calls.len() > MAX_ROWS;
        let mut metadata = |s: String| {
            if s.len() <= MAX_METADATA_BYTES {
                Some(s)
            } else {
                truncated = true;
                None
            }
        };
        let transaction_count = r.transaction_ids.len();
        let tool_call_count = r.tool_calls.len();
        let transaction_ids = r
            .transaction_ids
            .into_iter()
            .take(MAX_ROWS)
            .filter_map(&mut metadata)
            .collect();
        let tool_calls = r
            .tool_calls
            .into_iter()
            .take(MAX_ROWS)
            .map(
                |ToolObservation {
                     name,
                     succeeded,
                     selected_camera,
                 }| ToolSummary {
                    name: metadata(name),
                    succeeded,
                    selected_camera: selected_camera.and_then(&mut metadata),
                },
            )
            .collect();
        Self {
            steps: r.steps,
            usage: r.usage,
            transaction_count,
            tool_call_count,
            transaction_ids,
            tool_calls,
            truncated,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct TurnSnapshot {
    pub id: TurnId,
    pub request: TurnRequest,
    pub status: Status,
    pub text: String,
    pub approvals: Vec<ApprovalSummary>,
    pub pending: Option<PendingApproval>,
    pub report: Option<ReportSummary>,
    pub failure: Option<Failure>,
    /// Budget delta, not an invented split into input/output token usage.
    pub charged_tokens: u64,
    pub accounting_uncertain: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct SessionSnapshot {
    pub version: u64,
    pub budget: Budget,
    /// Some charges are conservative allowances rather than measured usage.
    /// Inconsistent/missing worker accounting exhausts the allowance entirely;
    /// provider failures without usage retain the agent's bounded reservation.
    pub accounting_uncertain: bool,
    pub worker_running: bool,
    pub turns: Vec<TurnSnapshot>,
}
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    #[error("the previous agent worker has not stopped")]
    Busy,
    #[error("invalid or excessive prompt/model metadata")]
    InvalidRequest,
    #[error("session token allowance exhausted")]
    Budget,
    #[error("stale or inactive turn")]
    StaleTurn,
    #[error("stale or already resolved approval")]
    StaleApproval,
    #[error("stream output limit exceeded; the turn is stopping")]
    OutputLimit,
    #[error("agent worker could not be started")]
    Spawn,
    #[error("native worker threads are unavailable")]
    Unsupported,
    #[error("agent controller is unavailable")]
    Unavailable,
    #[error("agent controller sequence exhausted")]
    SequenceExhausted,
}
