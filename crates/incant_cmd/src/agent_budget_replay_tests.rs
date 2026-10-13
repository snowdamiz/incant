use super::*;
use incant_doc::{AgentBudgetSettings, DEFAULT_AGENT_TOKEN_CAP};

// Keep journal ownership inside one test child. A sibling process-recovery test
// can otherwise fork while this thread owns the lock, briefly retaining that
// descriptor after the bus is dropped and before the child exec closes it.
fn isolated_replay(case: &str) -> bool {
    let test = format!("journal::agent_budget_replay_tests::{case}");
    if std::env::var("INCANT_CMD_REPLAY_TEST").as_deref() == Ok(test.as_str()) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &test, "--test-threads=1", "--nocapture"])
        .env("INCANT_CMD_REPLAY_TEST", &test)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{test}: {stdout}\n{stderr}");
    assert!(
        stdout.contains("test result: ok. 1 passed; 0 failed;"),
        "exact replay case did not execute: {stdout}\n{stderr}"
    );
    true
}

/// Re-hash a real journal after changing the recorded authority. This proves
/// refusal comes from authorization, rather than merely the integrity chain.
fn rewrite_authority(path: &Path, actor: Actor, strip_commands: bool) {
    let text = std::fs::read_to_string(path).unwrap();
    let mut previous = String::new();
    let mut rewritten = Vec::new();
    for line in text.lines() {
        let mut entry: Entry = serde_json::from_str(line).unwrap();
        if let Action::Apply { transaction } = &mut entry.action {
            transaction.actor = actor.clone();
            if strip_commands {
                transaction.commands.clear();
            }
        }
        entry.previous = previous;
        entry.digest = digest(
            entry.version,
            entry.sequence,
            &entry.previous,
            &entry.action,
            &entry.crdt,
        )
        .unwrap();
        previous = entry.digest.clone();
        rewritten.extend(serde_json::to_vec(&entry).unwrap());
        rewritten.push(b'\n');
    }
    std::fs::write(path, rewritten).unwrap();
}

#[test]
fn replay_refuses_nonuser_cap_changes_even_without_budget_commands() {
    if isolated_replay("replay_refuses_nonuser_cap_changes_even_without_budget_commands") {
        return;
    }
    for actor in [
        Actor::agent("test-model", "journal-test"),
        Actor::import("importer"),
        Actor {
            origin: Origin::Script,
            ..Actor::user("script")
        },
    ] {
        for strip_commands in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("journal.jsonl");
            let initial = Project::empty("Replay authority");
            let mut bus = CommandBus::persistent(&path, initial.clone()).unwrap();
            bus.execute(
                vec![Command::SetAgentBudget {
                    settings: AgentBudgetSettings { max_tokens: 0 },
                }],
                Actor::user("author"),
                "User-approved cap",
                None,
            )
            .unwrap();
            drop(bus);
            rewrite_authority(&path, actor.clone(), strip_commands);
            let journal = std::fs::read(&path).unwrap();
            let actual = CommandBus::persistent(&path, initial).err();
            assert!(
                matches!(actual, Some(CommandError::AgentBudgetUserOnly)),
                "actual replay error: {actual:?}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), journal);
        }
    }
}

#[test]
fn replay_refuses_nonuser_budget_commands_that_do_not_change_the_projection() {
    if isolated_replay("replay_refuses_nonuser_budget_commands_that_do_not_change_the_projection") {
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.jsonl");
    let initial = Project::empty("No-op authority");
    let mut bus = CommandBus::persistent(&path, initial.clone()).unwrap();
    bus.execute(
        vec![Command::SetAgentBudget {
            settings: AgentBudgetSettings {
                max_tokens: DEFAULT_AGENT_TOKEN_CAP,
            },
        }],
        Actor::user("author"),
        "Confirm cap",
        None,
    )
    .unwrap();
    drop(bus);
    rewrite_authority(&path, Actor::agent("test-model", "journal-test"), false);
    let journal = std::fs::read(&path).unwrap();
    let actual = CommandBus::persistent(&path, initial).err();
    assert!(
        matches!(actual, Some(CommandError::AgentBudgetUserOnly)),
        "actual replay error: {actual:?}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), journal);
}
