//! Bounded native turn transport. Providers, credentials, project access and
//! conversation context belong to the caller; this controller only owns workers
//! and their observable state. Cancellation is cooperative, never thread killing.
mod durable;
mod payload;
#[cfg(test)]
mod tests;
mod types;
use crate::{
    AgentError, Budget,
    ledger::{Ledger, LedgerSnapshot},
};
use serde_json::Value;
use std::{
    collections::VecDeque,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};
pub use types::*;

struct Active {
    id: TurnId,
    cancel: Arc<AtomicBool>,
    approval_bytes: usize,
    initial_used_tokens: u64,
}
struct State {
    version: u64,
    next_turn: u64,
    next_approval: u64,
    budget: Budget,
    uncertain: bool,
    active: Option<Active>,
    turns: VecDeque<TurnSnapshot>,
}
impl State {
    fn bump(&mut self) -> Result<(), SessionError> {
        if self.version >= types::MAX_SEQUENCE - 1 {
            self.version = types::MAX_SEQUENCE;
            if let Some(active) = &self.active {
                active.cancel.store(true, Ordering::Release);
            }
            return Err(SessionError::SequenceExhausted);
        }
        self.version += 1;
        Ok(())
    }
    fn current(&mut self, id: TurnId) -> Result<&mut TurnSnapshot, SessionError> {
        if self.active.as_ref().map(|a| a.id) != Some(id) {
            return Err(SessionError::StaleTurn);
        }
        self.turns
            .back_mut()
            .filter(|t| t.id == id)
            .ok_or(SessionError::StaleTurn)
    }
    fn live(&self, id: TurnId) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.id == id && !a.cancel.load(Ordering::Acquire))
            && self
                .turns
                .back()
                .is_some_and(|t| matches!(t.status, Status::Running | Status::AwaitingApproval))
            && self.version < types::MAX_SEQUENCE
    }
    fn stop(&mut self, id: TurnId, failure: Option<Failure>) -> Result<(), SessionError> {
        self.current(id)?;
        self.bump()?;
        self.active
            .as_ref()
            .unwrap()
            .cancel
            .store(true, Ordering::Release);
        let turn = self.current(id)?;
        turn.status = Status::Stopping;
        if turn.failure.is_none() {
            turn.failure = failure;
        }
        if let Some(pending) = turn.pending.take()
            && let Some(a) = turn.approvals.iter_mut().find(|a| a.id == pending.id)
        {
            a.decision = Decision::Cancelled;
        }
        Ok(())
    }
}
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}
impl Shared {
    fn lock(&self) -> Result<MutexGuard<'_, State>, SessionError> {
        self.state.lock().map_err(|_| SessionError::Unavailable)
    }
    fn bump(&self, state: &mut State) -> Result<(), SessionError> {
        let result = state.bump();
        if result.is_err() {
            self.changed.notify_all();
        }
        result
    }
}
pub struct Session {
    shared: Arc<Shared>,
    // Worker callbacks never acquire this lock. Joining a finished worker cannot
    // deadlock against its state publication, and competing starts serialize.
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl Session {
    pub fn new(budget: Budget) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    version: 0,
                    next_turn: 0,
                    next_approval: 0,
                    budget,
                    uncertain: false,
                    active: None,
                    turns: VecDeque::new(),
                }),
                changed: Condvar::new(),
            }),
            worker: Mutex::new(None),
        }
    }
    fn reap(&self, worker: &mut Option<JoinHandle<()>>) -> Result<(), SessionError> {
        if worker.as_ref().is_some_and(JoinHandle::is_finished) {
            // Join before freeing single-flight ownership, including thread-local teardown.
            let joined = worker.take().unwrap().join();
            let mut state = self.shared.lock()?;
            if joined.is_err() {
                state.budget.used_tokens = state.budget.used_tokens.max(state.budget.max_tokens);
                state.uncertain = true;
                let charged = state.active.as_ref().map(|active| {
                    state
                        .budget
                        .used_tokens
                        .saturating_sub(active.initial_used_tokens)
                });
                if let Some(turn) = state.turns.back_mut() {
                    turn.status = Status::Failed;
                    turn.failure = Some(Failure::WorkerPanicked);
                    turn.accounting_uncertain = true;
                    if let Some(charged) = charged {
                        turn.charged_tokens = charged;
                    }
                }
            }
            state.active = None;
            state.bump()?;
        }
        Ok(())
    }
    pub fn start(
        &self,
        request: TurnRequest,
        job: impl FnOnce(TurnContext) -> TurnOutcome + Send + 'static,
    ) -> Result<TurnId, SessionError> {
        self.start_inner(request, None, job, spawn_worker)
    }
    /// The caller opens the project ledger on a worker before admission. Only
    /// an accepted turn's worker reserves usage, before invoking `job`.
    pub fn start_durable(
        &self,
        request: TurnRequest,
        ledger: Ledger,
        max_tokens: u64,
        job: impl FnOnce(TurnContext) -> TurnOutcome + Send + 'static,
    ) -> Result<TurnId, SessionError> {
        let restored = ledger
            .snapshot(max_tokens)
            .map_err(|_| SessionError::InvalidRequest)?;
        self.start_inner(request, Some((ledger, restored)), job, spawn_worker)
    }
    /// Restore an idle display from a host-selected project's locked ledger.
    /// The caller must keep that ledger open through this call and fence its
    /// current project/cap identity; a detached snapshot has no freshness token.
    /// No filesystem work occurs here. An active worker's budget is unchanged.
    pub fn restore_budget_if_idle(&self, snapshot: LedgerSnapshot) -> Result<(), SessionError> {
        durable::validate_snapshot(&snapshot)?;
        let mut worker = self.worker.lock().map_err(|_| SessionError::Unavailable)?;
        self.reap(&mut worker)?;
        if worker.is_some() {
            return Err(SessionError::Busy);
        }
        let mut state = self.shared.lock()?;
        restore(&self.shared, &mut state, snapshot)
    }
    fn start_inner(
        &self,
        request: TurnRequest,
        durable: Option<(Ledger, LedgerSnapshot)>,
        job: impl FnOnce(TurnContext) -> TurnOutcome + Send + 'static,
        spawn: impl FnOnce(String, WorkerJob) -> std::io::Result<JoinHandle<()>>,
    ) -> Result<TurnId, SessionError> {
        if !cfg!(any(unix, windows)) {
            return Err(SessionError::Unsupported);
        }
        if request.prompt.len() > MAX_PROMPT_BYTES
            || request.prompt.trim().is_empty()
            || request.model.len() > MAX_METADATA_BYTES
            || request.model.trim().is_empty()
            || request
                .view_context
                .as_ref()
                .is_some_and(|view| !view.is_valid())
        {
            return Err(SessionError::InvalidRequest);
        }
        let mut worker = self.worker.lock().map_err(|_| SessionError::Unavailable)?;
        self.reap(&mut worker)?;
        if worker.is_some() {
            return Err(SessionError::Busy);
        }
        let mut state = self.shared.lock()?;
        let ledger = if let Some((ledger, snapshot)) = durable {
            restore(&self.shared, &mut state, snapshot)?;
            Some(ledger)
        } else {
            None
        };
        if state.budget.remaining() == 0 {
            return Err(SessionError::Budget);
        }
        let next = state
            .next_turn
            .checked_add(1)
            .filter(|v| *v < types::MAX_SEQUENCE)
            .ok_or(SessionError::SequenceExhausted)?;
        state.bump()?;
        state.next_turn = next;
        let id = TurnId(next);
        let cancel = Arc::new(AtomicBool::new(false));
        let initial = state.budget.clone();
        if state.turns.len() == MAX_HISTORY {
            state.turns.pop_front();
        }
        state.turns.push_back(TurnSnapshot {
            id,
            request: request.clone(),
            status: Status::Running,
            text: String::new(),
            approvals: vec![],
            pending: None,
            report: None,
            failure: None,
            charged_tokens: 0,
            accounting_uncertain: false,
        });
        state.active = Some(Active {
            id,
            cancel: cancel.clone(),
            approval_bytes: 0,
            initial_used_tokens: initial.used_tokens,
        });
        let context = TurnContext {
            id,
            request,
            budget: initial.clone(),
            cancel,
            shared: self.shared.clone(),
        };
        let shared = self.shared.clone();
        match spawn(
            format!("incant-agent-{}", id.0),
            Box::new(move || {
                let completion = catch_unwind(AssertUnwindSafe(|| match ledger {
                    Some(ledger) => durable::run(context, ledger, job),
                    None => prepare(&initial, Some(job(context))),
                }))
                .unwrap_or_else(|_| conservative(&initial, Failure::WorkerPanicked));
                finish(&shared, id, initial, completion);
            }),
        ) {
            Ok(handle) => {
                *worker = Some(handle);
                Ok(id)
            }
            Err(_) => {
                state.active = None;
                let turn = state.turns.back_mut().unwrap();
                turn.status = Status::Failed;
                turn.failure = Some(Failure::WorkerSpawn);
                Err(SessionError::Spawn)
            }
        }
    }
    /// Stop publication immediately; the active slot remains occupied until join.
    pub fn cancel(&self, id: TurnId) -> Result<(), SessionError> {
        let mut state = self.shared.lock()?;
        if !state.live(id) {
            return Err(SessionError::StaleTurn);
        }
        let result = state.stop(id, None);
        self.shared.changed.notify_all();
        result
    }
    pub fn decide(&self, turn: TurnId, id: ApprovalId, allow: bool) -> Result<(), SessionError> {
        let mut state = self.shared.lock()?;
        if !state.live(turn) {
            return Err(SessionError::StaleTurn);
        }
        if state.current(turn)?.pending.as_ref().map(|p| p.id) != Some(id) {
            return Err(SessionError::StaleApproval);
        }
        self.shared.bump(&mut state)?;
        let current = state.current(turn)?;
        current.pending = None; // Discard source bodies as soon as the decision is made.
        current
            .approvals
            .iter_mut()
            .find(|a| a.id == id)
            .unwrap()
            .decision = if allow {
            Decision::Allowed
        } else {
            Decision::Denied
        };
        current.status = Status::Running;
        self.shared.changed.notify_all();
        Ok(())
    }
    pub fn snapshot(&self) -> Result<SessionSnapshot, SessionError> {
        self.snapshot_if_changed(None)?
            .ok_or(SessionError::Unavailable)
    }
    /// Version comparison happens before cloning history or a pending payload.
    pub fn snapshot_if_changed(
        &self,
        after_version: Option<u64>,
    ) -> Result<Option<SessionSnapshot>, SessionError> {
        let mut worker = self.worker.lock().map_err(|_| SessionError::Unavailable)?;
        self.reap(&mut worker)?;
        let state = self.shared.lock()?;
        if state.version >= types::MAX_SEQUENCE {
            return Err(SessionError::SequenceExhausted);
        }
        if after_version == Some(state.version) {
            return Ok(None);
        }
        Ok(Some(SessionSnapshot {
            version: state.version,
            budget: state.budget.clone(),
            accounting_uncertain: state.uncertain,
            worker_running: worker.is_some(),
            turns: state.turns.iter().cloned().collect(),
        }))
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.lock()
            && let Some(id) = state.active.as_ref().map(|a| a.id)
        {
            let _ = state.stop(id, None);
        }
        // A blocked provider is not forcibly killed or joined on the UI thread.
        self.shared.changed.notify_all();
    }
}

#[derive(Clone)]
pub struct TurnContext {
    id: TurnId,
    pub request: TurnRequest,
    pub budget: Budget,
    cancel: Arc<AtomicBool>,
    shared: Arc<Shared>,
}
impl TurnContext {
    pub fn turn_id(&self) -> TurnId {
        self.id
    }
    pub fn cancellation(&self) -> &AtomicBool {
        &self.cancel
    }
    pub fn stream(&self, text: &str) -> Result<(), SessionError> {
        let mut state = self.shared.lock()?;
        if !state.live(self.id) {
            return Err(SessionError::StaleTurn);
        }
        if text.is_empty() {
            return Ok(());
        }
        if text.len() > MAX_TEXT_BYTES - state.current(self.id)?.text.len() {
            let result = state.stop(self.id, Some(Failure::OutputLimit));
            self.shared.changed.notify_all();
            result?;
            return Err(SessionError::OutputLimit);
        }
        self.shared.bump(&mut state)?;
        // Whole UTF-8 chunks are accepted or rejected; no split code point or
        // silent truncation can be mistaken for a complete model response.
        state.current(self.id)?.text.push_str(text);
        Ok(())
    }
    /// Blocks only this worker. Pass directly to Agent's approval callback;
    /// ApprovalChoice is enforced by the Agent, never guessed by the controller.
    pub fn approve(&self, tool: &str, arguments: &Value) -> bool {
        self.approval(tool, arguments).unwrap_or(false)
    }
    fn approval(&self, tool: &str, arguments: &Value) -> Result<bool, SessionError> {
        // Serialization may be relatively expensive; no shared-state lock is held.
        {
            let state = self.shared.lock()?;
            if !state.live(self.id) {
                return Err(SessionError::StaleTurn);
            }
        }
        let prepared = if tool.len() > MAX_METADATA_BYTES || tool.trim().is_empty() {
            Err(Failure::ApprovalInvalid)
        } else {
            payload::prepare(arguments)
        };
        let mut state = self.shared.lock()?;
        if !state.live(self.id) {
            return Err(SessionError::StaleTurn);
        }
        let failure = if state.current(self.id)?.pending.is_some() {
            Some(Failure::ApprovalAlreadyPending)
        } else if state.current(self.id)?.approvals.len() == MAX_APPROVALS {
            Some(Failure::ApprovalCountLimit)
        } else if let Err(failure) = &prepared {
            Some(*failure)
        } else if prepared.as_ref().unwrap().1
            > MAX_APPROVAL_TOTAL_BYTES - state.active.as_ref().unwrap().approval_bytes
        {
            Some(Failure::ApprovalTotalLimit)
        } else {
            None
        };
        if let Some(failure) = failure {
            let result = state.stop(self.id, Some(failure));
            self.shared.changed.notify_all();
            result?;
            return Ok(false);
        }
        let (arguments, bytes) = prepared.unwrap();
        let next = state
            .next_approval
            .checked_add(1)
            .filter(|v| *v < types::MAX_SEQUENCE)
            .ok_or(SessionError::SequenceExhausted)?;
        state.bump()?;
        state.next_approval = next;
        let id = ApprovalId(next);
        state.active.as_mut().unwrap().approval_bytes += bytes;
        let turn = state.current(self.id)?;
        turn.status = Status::AwaitingApproval;
        turn.approvals.push(ApprovalSummary {
            id,
            tool: tool.into(),
            bytes,
            decision: Decision::Pending,
        });
        turn.pending = Some(PendingApproval {
            turn: self.id,
            id,
            tool: tool.into(),
            arguments,
        });
        loop {
            if !state.live(self.id) {
                return Ok(false);
            }
            let decision = state
                .current(self.id)?
                .approvals
                .iter()
                .find(|a| a.id == id)
                .ok_or(SessionError::StaleApproval)?
                .decision;
            match decision {
                Decision::Allowed => return Ok(true),
                Decision::Denied | Decision::Cancelled => return Ok(false),
                Decision::Pending => {}
            }
            state = self
                .shared
                .changed
                .wait(state)
                .map_err(|_| SessionError::Unavailable)?;
        }
    }
}
fn failure(error: AgentError) -> Option<Failure> {
    Some(match error {
        AgentError::Provider(_) | AgentError::Response { .. } => Failure::Provider,
        AgentError::Tool(_) => Failure::Tool,
        AgentError::Budget => Failure::BudgetExhausted,
        AgentError::Interrupted => return None,
        AgentError::StepLimit => Failure::StepLimit,
        AgentError::ContextLimit => Failure::ContextLimit,
        AgentError::ProjectUnavailable => Failure::ProjectUnavailable,
        AgentError::ProjectChanged => Failure::ProjectChanged,
    })
}
type WorkerJob = Box<dyn FnOnce() + Send + 'static>;
fn spawn_worker(name: String, job: WorkerJob) -> std::io::Result<JoinHandle<()>> {
    thread::Builder::new().name(name).spawn(job)
}
fn restore(
    shared: &Shared,
    state: &mut State,
    snapshot: LedgerSnapshot,
) -> Result<(), SessionError> {
    durable::validate_snapshot(&snapshot)?;
    if state.version >= types::MAX_SEQUENCE {
        return Err(SessionError::SequenceExhausted);
    }
    if state.budget.max_tokens != snapshot.budget.max_tokens
        || state.budget.used_tokens != snapshot.budget.used_tokens
        || state.uncertain != snapshot.accounting_uncertain
    {
        shared.bump(state)?;
        state.budget = snapshot.budget;
        state.uncertain = snapshot.accounting_uncertain;
        shared.changed.notify_all();
    }
    Ok(())
}
struct Completion {
    budget: Budget,
    report: Option<ReportSummary>,
    error: Option<Option<Failure>>,
    uncertain: bool,
    valid_receipt: bool,
}
fn conservative(initial: &Budget, failure: Failure) -> Completion {
    Completion {
        budget: Budget {
            max_tokens: initial.max_tokens,
            used_tokens: initial.used_tokens.max(initial.max_tokens),
        },
        report: None,
        error: Some(Some(failure)),
        uncertain: true,
        valid_receipt: false,
    }
}
fn prepare(initial: &Budget, outcome: Option<TurnOutcome>) -> Completion {
    // Bound/summarize an owned report and drop any excess outside the state lock.
    match outcome {
        Some(TurnOutcome { budget, result })
            if budget.max_tokens == initial.max_tokens
                && budget.used_tokens >= initial.used_tokens
                && budget.used_tokens >= minimum_usage(initial, &result) =>
        {
            // The agent reserves an allowance when a transport failure has no
            // measured usage. Preserve that charge without labelling it exact.
            let uncertain = budget.used_tokens > initial.used_tokens
                && matches!(&result, Err(AgentError::Response { usage: None, .. }));
            let (report, error) = match result {
                Ok(r) => (
                    Some(ReportSummary::from(r)),
                    (budget.used_tokens > budget.max_tokens)
                        .then_some(Some(Failure::BudgetExhausted)),
                ),
                Err(e) => (None, Some(failure(e))),
            };
            Completion {
                budget,
                report,
                error,
                uncertain,
                valid_receipt: true,
            }
        }
        value => conservative(
            initial,
            if value.is_none() {
                Failure::WorkerPanicked
            } else {
                Failure::BudgetAccounting
            },
        ),
    }
}
fn finish(shared: &Shared, id: TurnId, initial: Budget, completion: Completion) {
    let Completion {
        budget,
        report,
        error,
        uncertain,
        ..
    } = completion;
    let Ok(mut state) = shared.lock() else { return };
    if state.active.as_ref().map(|a| a.id) != Some(id) {
        return;
    }
    let cancelled = state
        .active
        .as_ref()
        .unwrap()
        .cancel
        .load(Ordering::Acquire);
    state.budget = budget;
    state.uncertain |= uncertain;
    let charged = state.budget.used_tokens.saturating_sub(initial.used_tokens);
    if state.bump().is_err() {
        shared.changed.notify_all();
        return;
    }
    let Ok(turn) = state.current(id) else { return };
    turn.report = report;
    turn.charged_tokens = charged;
    turn.accounting_uncertain = uncertain;
    if let Some(pending) = turn.pending.take()
        && let Some(a) = turn.approvals.iter_mut().find(|a| a.id == pending.id)
    {
        a.decision = Decision::Cancelled;
    }
    if matches!(
        error,
        Some(Some(Failure::WorkerPanicked | Failure::BudgetAccounting))
    ) || (!cancelled && turn.failure.is_none())
    {
        turn.failure = error.flatten();
    }
    turn.status = if turn.failure.is_some() {
        Status::Failed
    } else if cancelled || error == Some(None) {
        Status::Cancelled
    } else {
        Status::Completed
    };
    shared.changed.notify_all();
}

fn minimum_usage(initial: &Budget, result: &Result<crate::TurnReport, AgentError>) -> u64 {
    let usage = match result {
        Ok(report) => Some(&report.usage),
        Err(AgentError::Response { usage, .. }) => usage.as_ref(),
        _ => None,
    };
    usage.map_or(initial.used_tokens, |u| {
        initial
            .used_tokens
            .saturating_add(u.input_tokens)
            .saturating_add(u.output_tokens)
    })
}
