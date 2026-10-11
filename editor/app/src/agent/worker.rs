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
    context: Arc<Mutex<Context>>,
    bus: Arc<Mutex<crate::project::LoadState>>,
    root: Option<PathBuf>,
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
        let store = AccountStore::open()?;
        if Connection::from_store(&store).as_ref() != Some(&connection) {
            return Err(AgentError::Provider("The active account changed".into()));
        }
        let selection = store.selection().ok_or(AgentError::ProjectUnavailable)?;
        let mut provider = store.provider(turn.request.model.clone())?;
        let mut host = host(&bus, root)?;
        execute(
            &mut agent,
            &turn,
            &context,
            ContextKey::Account(selection),
            &bus,
            host.as_mut(),
            &mut provider,
        )
    })();
    TurnOutcome {
        budget: agent.budget,
        result,
    }
}

pub(super) fn host(
    bus: &Mutex<crate::project::LoadState>,
    root: Option<PathBuf>,
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
    Ok(Box::new(ProjectHost::new(&project, &root, native)?))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute(
    agent: &mut Agent,
    turn: &TurnContext,
    context: &Mutex<Context>,
    selection: ContextKey,
    bus: &Mutex<crate::project::LoadState>,
    host: &mut dyn EngineHost,
    provider: &mut dyn Provider,
) -> Result<incant_agent::TurnReport, AgentError> {
    let mut context = context.lock().map_err(|_| AgentError::ProjectUnavailable)?;
    if context.selection.as_ref() != Some(&selection) {
        context.conversation.clear();
        context.selection = Some(selection);
    }
    // Session IDs restart on launch; provenance must remain globally distinct.
    let provenance = ulid::Ulid::new().to_string();
    agent.run_shared_in_conversation(
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
    )
}
