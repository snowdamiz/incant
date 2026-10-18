#[cfg(all(test, feature = "agent-profiler"))]
mod profiler_tests;
#[cfg(test)]
mod tests;
use super::{Connection, Context, ContextKey};
use incant_agent::{
    Agent, AgentError, EngineHost, ProjectHost,
    accounts::AccountStore,
    provider::Provider,
    session::{TurnContext, TurnOutcome},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(super) fn run(
    turn: TurnContext,
    connection: Connection,
    state: &mut State,
    bus: Arc<Mutex<crate::project::LoadState>>,
    root: Option<PathBuf>,
    expected: super::budget::Scope,
) -> TurnOutcome {
    let result = (|| {
        if turn
            .cancellation()
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(AgentError::Interrupted);
        }
        expected.check_agent(&bus)?;
        let store = AccountStore::open()?;
        if Connection::from_store(&store).as_ref() != Some(&connection) {
            return Err(AgentError::Provider("The active account changed".into()));
        }
        let selection = store.selection().ok_or(AgentError::ProjectUnavailable)?;
        let provider = store.provider(turn.request.model.clone())?;
        Ok((selection, provider))
    })();
    match result {
        Ok((selection, mut provider)) => state.run_with_scope(
            turn,
            ContextKey::Account(selection),
            bus,
            root,
            &mut provider,
            Some(expected),
        ),
        Err(error) => TurnOutcome {
            budget: turn.budget,
            result: Err(error),
        },
    }
}

struct Scope {
    selection: ContextKey,
    bus: Arc<Mutex<crate::project::LoadState>>,
    project_id: String,
    root: Option<PathBuf>,
}
/// Constructed, used and dropped only inside the dedicated owner thread. The
/// host may contain !Send QuickJS state; it never appears in a channel payload.
#[derive(Default)]
pub(super) struct State {
    module_jobs: Option<Arc<incant_module_build::Service>>,
    scope: Option<Scope>,
    host: Option<Box<dyn EngineHost>>,
    context: Context,
}
impl State {
    pub(super) fn module_jobs(&mut self, jobs: Arc<incant_module_build::Service>) {
        if self
            .module_jobs
            .as_ref()
            .is_none_or(|old| !Arc::ptr_eq(old, &jobs))
        {
            self.host = None;
            self.scope = None;
            self.module_jobs = Some(jobs);
        }
    }
    #[cfg(test)]
    pub(super) fn run_with(
        &mut self,
        turn: TurnContext,
        selection: ContextKey,
        bus: Arc<Mutex<crate::project::LoadState>>,
        root: Option<PathBuf>,
        provider: &mut dyn Provider,
    ) -> TurnOutcome {
        self.run_with_scope(turn, selection, bus, root, provider, None)
    }
    fn run_with_scope(
        &mut self,
        turn: TurnContext,
        selection: ContextKey,
        bus: Arc<Mutex<crate::project::LoadState>>,
        root: Option<PathBuf>,
        provider: &mut dyn Provider,
        expected: Option<super::budget::Scope>,
    ) -> TurnOutcome {
        let mut agent = Agent {
            budget: turn.budget.clone(),
            approval: turn.request.approval.mode(),
            max_steps: 64,
            max_output_tokens: 16_384,
        };
        let result = (|| {
            if turn
                .cancellation()
                .load(std::sync::atomic::Ordering::Acquire)
            {
                return Err(AgentError::Interrupted);
            }
            if let Some(expected) = &expected {
                expected.check_agent(&bus)?;
            }
            // Project identity is read without retaining the authoring lock.
            let project_id = match &*bus.lock().map_err(|_| AgentError::ProjectUnavailable)? {
                crate::project::LoadState::Ready(bus) => bus.project().id.clone(),
                _ => return Err(AgentError::ProjectUnavailable),
            };
            let same = self.scope.as_ref().is_some_and(|scope| {
                scope.selection == selection
                    && Arc::ptr_eq(&scope.bus, &bus)
                    && scope.project_id == project_id
                    && scope.root == root
            });
            if !same {
                self.host = None;
                self.scope = None;
                self.context = Context::default();
                self.host = Some(host_with_module_jobs(
                    &bus,
                    root.clone(),
                    self.module_jobs.clone(),
                )?);
                self.scope = Some(Scope {
                    selection: selection.clone(),
                    bus: bus.clone(),
                    project_id,
                    root,
                });
            }
            // Validate the opaque receipt before granting access to retained
            // state; the agent loop repeats it around provider/tool boundaries.
            let mut provider = BoundProvider {
                inner: provider,
                bus: &bus,
                project_id: &self.scope.as_ref().expect("prepared scope").project_id,
                budget_scope: expected.as_ref(),
            };
            provider.check_active()?;
            execute(
                &mut agent,
                &turn,
                &mut self.context,
                &bus,
                self.host.as_mut().expect("prepared host").as_mut(),
                &mut provider,
            )
        })();
        TurnOutcome {
            budget: agent.budget,
            result,
        }
    }
}

// Close the identity-read/host-preparation/ProjectAccess-capture window. Account
// checks finish before the short project read; no store/project locks overlap.
struct BoundProvider<'a> {
    inner: &'a mut dyn Provider,
    bus: &'a Mutex<crate::project::LoadState>,
    project_id: &'a str,
    budget_scope: Option<&'a super::budget::Scope>,
}
impl Provider for BoundProvider<'_> {
    fn check_active(&self) -> Result<(), AgentError> {
        self.inner.check_active()?;
        match &*self
            .bus
            .lock()
            .map_err(|_| AgentError::ProjectUnavailable)?
        {
            crate::project::LoadState::Ready(bus) if bus.project().id == self.project_id => {
                if let Some(scope) = self.budget_scope {
                    if bus.project().id != scope.project_id {
                        return Err(AgentError::ProjectChanged);
                    }
                    if bus.project().settings.agent_budget.max_tokens != scope.max_tokens {
                        return Err(AgentError::ProjectChanged);
                    }
                }
                Ok(())
            }
            crate::project::LoadState::Ready(_) => Err(AgentError::ProjectChanged),
            _ => Err(AgentError::ProjectUnavailable),
        }
    }
    fn model(&self) -> &str {
        self.inner.model()
    }
    fn complete(
        &mut self,
        input: &[serde_json::Value],
        tools: &[serde_json::Value],
        max: u64,
        text: &mut dyn FnMut(&str),
    ) -> Result<incant_agent::provider::Completion, AgentError> {
        self.inner.complete(input, tools, max, text)
    }
}

#[cfg(test)]
pub(super) fn host(
    bus: &Mutex<crate::project::LoadState>,
    root: Option<PathBuf>,
) -> Result<Box<dyn EngineHost>, AgentError> {
    host_with_module_jobs(bus, root, None)
}
fn host_with_module_jobs(
    bus: &Mutex<crate::project::LoadState>,
    root: Option<PathBuf>,
    jobs: Option<Arc<incant_module_build::Service>>,
) -> Result<Box<dyn EngineHost>, AgentError> {
    let Some(root) = root else {
        return Ok(Box::new(incant_agent_native::GpuPerception::new(None)));
    };
    // An owned authoring snapshot is taken once to bind the file capability.
    // Native cooking, simulation and screenshots happen after releasing it.
    let project = match &*bus.lock().map_err(|_| AgentError::ProjectUnavailable)? {
        crate::project::LoadState::Ready(bus) => bus.project().clone(),
        _ => return Err(AgentError::ProjectUnavailable),
    };
    let viewport = incant_agent_native::GpuPerception::new(Some(root.clone()));
    let native = incant_agent_native::Host::new(root.clone(), viewport, || {
        incant_script::CompiledScript::compile("exports.default={update(){}};")
    });
    let host = ProjectHost::new(&project, &root, native)?;
    Ok(Box::new(if let Some(jobs) = jobs {
        host.with_module_jobs(jobs)
    } else {
        host
    }))
}

fn execute(
    agent: &mut Agent,
    turn: &TurnContext,
    context: &mut Context,
    bus: &Mutex<crate::project::LoadState>,
    host: &mut dyn EngineHost,
    provider: &mut dyn Provider,
) -> Result<incant_agent::TurnReport, AgentError> {
    // Session IDs restart on launch; provenance must remain globally distinct.
    let provenance = ulid::Ulid::new().to_string();
    agent.run_shared_in_conversation_with_view(
        provider,
        bus,
        host,
        &turn.request.prompt,
        &provenance,
        turn.cancellation(),
        &mut |name, arguments| turn.approve(name, arguments),
        &mut |text| {
            let _ = turn.stream(text);
        },
        &mut context.conversation,
        turn.request.view_context.as_ref(),
    )
}
