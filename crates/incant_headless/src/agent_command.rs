//! CLI inference shares the editor's per-project durable accounting. No shell tool.
#[cfg(test)]
mod tests;

use crate::{Result, native_play, print, read_project, save_derived};
use incant_agent::{
    Agent, AgentError, ApprovalMode, Budget, EngineHost, ProjectHost, TurnReport,
    ledger::{Ledger, LedgerError, MAX_TOKEN_CAP},
    provider::Provider,
    session::{
        ApprovalChoice, Failure, Session, SessionError, Status, TurnContext, TurnOutcome,
        TurnRequest,
    },
};
use incant_agent_native::GpuPerception;
use incant_cmd::CommandBus;
use incant_doc::{Project, new_id};
use serde::Serialize;
use serde_json::Value;
use std::{
    io,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};
use thiserror::Error;

#[derive(clap::Args)]
pub(super) struct Options {
    project: PathBuf,
    prompt: String,
    #[arg(long)]
    model: String,
    /// Additional tokens for this invocation, bounded by the project's remaining cap (0 disables).
    #[arg(long, default_value_t = 64000)]
    max_tokens: u64,
    #[arg(long, default_value_t = 25000)]
    max_output_tokens: u64,
    #[arg(long)]
    journal: Option<PathBuf>,
}
#[derive(Debug, Error)]
enum CommandError {
    #[error(transparent)]
    Ledger(#[from] LedgerError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error("project or invocation agent token allowance is exhausted")]
    Allowance,
    #[error("agent turn failed: {0:?}")]
    Turn(Failure),
    #[error("agent turn interrupted")]
    Interrupted,
    #[error("agent output could not be prepared; usage remains charged")]
    OutputPrepare,
    #[error("agent output could not be saved; usage remains charged")]
    OutputSave,
}
#[derive(Serialize)]
struct Receipt {
    report: Value,
    output: PathBuf,
    journal: PathBuf,
    /// Effective absolute invocation ceiling, with cumulative project usage.
    budget: Budget,
    project_max_tokens: u64,
    accounting_uncertain: bool,
}
struct Prepared {
    report: Value,
    text: String,
}
struct Work {
    bus: CommandBus,
    budget: Budget,
    result: std::result::Result<TurnReport, AgentError>,
}

pub(super) fn run(options: Options) -> Result<()> {
    let max_output_tokens = options.max_output_tokens;
    let receipt = execute(
        options,
        Ledger::open,
        move |turn, bus, root| {
            agent_work(
                turn,
                bus,
                root,
                |project, root, turn| {
                    // Construction performs no inference. A failure here is a known
                    // zero-work receipt, preserving all previous ledger charges.
                    let provider = crate::provider(turn.request.model.clone()).map_err(|_| {
                        AgentError::Provider("Account connection is unavailable".into())
                    })?;
                    let viewport = incant_agent_native::Host::new(
                        root.to_path_buf(),
                        GpuPerception::new(Some(root.to_path_buf())),
                        native_play::embedded::load,
                    );
                    Ok((provider, ProjectHost::new(project, root, viewport)?))
                },
                max_output_tokens,
                |name, args| {
                    eprintln!("Approve {name}: {args}\nType yes to apply:");
                    let mut answer = String::new();
                    io::stdin().read_line(&mut answer).is_ok() && answer.trim() == "yes"
                },
                |text| eprint!("{text}"),
            )
        },
        save_derived,
    )?;
    print(receipt)
}

/// The CLI limit is additional allowance, never an edit or replacement of the
/// authored project cap. Saturation cannot make prior consumption reusable.
fn invocation_ceiling(project_cap: u64, used: u64, limit: u64) -> u64 {
    project_cap
        .min(used.saturating_add(limit))
        .min(MAX_TOKEN_CAP)
}

fn execute(
    options: Options,
    open: impl FnOnce(&str) -> std::result::Result<Ledger, LedgerError>,
    worker: impl FnOnce(TurnContext, CommandBus, PathBuf) -> Work + Send + 'static,
    write: impl FnOnce(&Path, &str) -> Result<()>,
) -> Result<Receipt> {
    let initial = read_project(&options.project)?;
    let journal = options
        .journal
        .unwrap_or_else(|| options.project.with_extension("journal.jsonl"));
    let root = options
        .project
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    // Journal replay enforces the genesis project identity. Acquire accounting
    // ownership first so a predecessor cannot finish after our journal read.
    let ledger = open(&initial.id)?;
    let bus = CommandBus::open_project_file_with_journal(&options.project, &journal)?;
    if bus.project().id != initial.id {
        return Err("opened project identity changed".into());
    }
    // The journal may contain user cap edits/Undo; the input is only its genesis.
    let project_cap = bus.project().settings.agent_budget.max_tokens;
    let restored = ledger.snapshot(project_cap)?;
    let ceiling = invocation_ceiling(project_cap, restored.budget.used_tokens, options.max_tokens);
    if ceiling <= restored.budget.used_tokens {
        return Err(CommandError::Allowance.into());
    }
    let session = Session::new(restored.budget);
    let (prepared, result) = mpsc::sync_channel(1);
    session.start_durable(
        TurnRequest {
            prompt: options.prompt,
            model: options.model,
            approval: ApprovalChoice::Destructive,
        },
        ledger,
        ceiling,
        move |turn| {
            // Session reserves the complete effective allowance before this job.
            // Construct !Send native hosts here; never move them between threads.
            let work = worker(turn, bus, root);
            let output = work.result.as_ref().ok().map(|report| {
                Ok::<_, CommandError>(Prepared {
                    report: serde_json::to_value(report)
                        .map_err(|_| CommandError::OutputPrepare)?,
                    text: work
                        .bus
                        .project()
                        .canonical_text()
                        .map_err(|_| CommandError::OutputPrepare)?,
                })
            });
            let _ = prepared.send(output);
            TurnOutcome {
                budget: work.budget,
                result: work.result,
            }
        },
    )?;
    let prepared = result.recv().ok().flatten();
    // The channel is only a work result, not an accounting receipt. Wait for
    // durable settlement and actual worker reap before saving any output.
    let snapshot = loop {
        let snapshot = session.snapshot()?;
        if !snapshot.worker_running {
            break snapshot;
        }
        thread::sleep(Duration::from_millis(1));
    };
    let turn = snapshot.turns.last().ok_or(SessionError::Unavailable)?;
    if let Some(failure) = turn.failure {
        return Err(CommandError::Turn(failure).into());
    }
    if turn.status != Status::Completed {
        return Err(CommandError::Interrupted.into());
    }
    let prepared = prepared.ok_or(CommandError::OutputPrepare)??;
    // Keep the input file as journal genesis. Usage is already durable even if
    // publishing this derived output fails or stdout is closed by the caller.
    let output = options.project.with_extension("agent-result.json");
    write(&output, &prepared.text).map_err(|_| CommandError::OutputSave)?;
    Ok(Receipt {
        report: prepared.report,
        output,
        journal,
        budget: snapshot.budget,
        project_max_tokens: project_cap,
        accounting_uncertain: snapshot.accounting_uncertain,
    })
}

fn agent_work<P: Provider, H: EngineHost>(
    turn: TurnContext,
    mut bus: CommandBus,
    root: PathBuf,
    construct: impl FnOnce(&Project, &Path, &TurnContext) -> std::result::Result<(P, H), AgentError>,
    max_output_tokens: u64,
    mut approve: impl FnMut(&str, &Value) -> bool,
    mut stream: impl FnMut(&str),
) -> Work {
    let mut agent = Agent {
        budget: turn.budget.clone(),
        approval: ApprovalMode::Destructive,
        max_steps: 30,
        max_output_tokens,
    };
    let result = (|| {
        let (mut provider, mut host) = construct(bus.project(), &root, &turn)?;
        agent.run(
            &mut provider,
            &mut bus,
            &mut host,
            &turn.request.prompt,
            &new_id(),
            turn.cancellation(),
            &mut approve,
            &mut stream,
        )
    })();
    Work {
        bus,
        budget: agent.budget,
        result,
    }
}
