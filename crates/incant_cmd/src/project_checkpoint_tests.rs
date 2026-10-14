//! Real opened files and fresh-process checkpoint recovery; no application mock
//! counts as filesystem durability evidence.
use super::*;
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
    process::Command as Process,
};

fn legacy() -> Project {
    let mut p = Project::empty("Application");
    p.engine_version = None;
    p
}
fn edit(bus: &mut CommandBus, text: &str) {
    bus.execute(
        vec![Command::SetMemory {
            section: "note".into(),
            text: text.into(),
        }],
        Actor::user("test"),
        "note",
        Some(bus.revision()),
    )
    .unwrap();
}
fn journal_bytes(bus: &mut CommandBus) -> Vec<u8> {
    let f = bus.journal.as_mut().unwrap().test_file();
    let pos = f.stream_position().unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = vec![];
    f.read_to_end(&mut bytes).unwrap();
    f.seek(SeekFrom::Start(pos)).unwrap();
    bytes
}
fn save(bus: &mut CommandBus) -> Result<ProjectSaveReceipt, CommandError> {
    bus.save_project_file(bus.revision(), Actor::user("test"))
}
#[test]
fn legacy_custom_journal_save_pin_history_and_reopen_share_one_capability() {
    if crate::test_isolation::isolated(
        "project_checkpoint_tests::legacy_custom_journal_save_pin_history_and_reopen_share_one_capability",
    ) {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let path = root.path().join("game.incant.json");
    let journal = logs.path().join("custom.jsonl");
    let p = legacy();
    fs::write(&path, p.canonical_text().unwrap()).unwrap();
    let mut old = CommandBus::persistent_with_sources(&journal, p, root.path()).unwrap();
    edit(&mut old, "before pin");
    let initial = journal_bytes(&mut old);
    drop(old);
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    assert!(journal_bytes(&mut bus).starts_with(&initial));
    assert_eq!(bus.revision(), 1);
    assert!(
        bus.review_project_file_pin().is_err(),
        "save unsaved logical edits first"
    );
    let causal = bus.journal.as_ref().unwrap().checkpoint_bytes().unwrap();
    assert!(save(&mut bus).unwrap().changed);
    assert_eq!(bus.revision(), 1);
    assert_eq!(bus.history().len(), 1);
    assert_eq!(
        bus.journal.as_ref().unwrap().checkpoint_bytes().unwrap(),
        causal
    );
    let before_pin = fs::read(&path).unwrap();
    let review = bus.review_project_file_pin().unwrap();
    bus.apply_project_file_pin(&review.review_id, Actor::user("test"))
        .unwrap();
    edit(&mut bus, "after pin");
    save(&mut bus).unwrap();
    bus.undo().unwrap(); // logical undo does not secretly save
    assert_ne!(
        Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap(),
        *bus.project()
    );
    bus.undo().unwrap(); // pin undo checkpoints its own target despite newer disk
    assert_eq!(fs::read(&path).unwrap(), before_pin);
    assert_eq!(bus.history().len(), 1);
    assert!(bus.can_redo());
    let rev = bus.revision();
    drop(bus);
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    assert_eq!(bus.revision(), rev);
    bus.redo().unwrap();
    bus.redo().unwrap();
    save(&mut bus).unwrap();
    assert_eq!(
        Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap(),
        *bus.project()
    );
    let bytes = journal_bytes(&mut bus);
    let crdt = bus.journal.as_ref().unwrap().checkpoint_bytes().unwrap();
    assert!(!save(&mut bus).unwrap().changed);
    assert_eq!(journal_bytes(&mut bus), bytes);
    assert_eq!(
        bus.journal.as_ref().unwrap().checkpoint_bytes().unwrap(),
        crdt
    );
    assert!(!path.with_extension("journal").exists());
    assert!(!path.with_extension("journal.jsonl").exists());
}
#[test]
fn save_refuses_stale_revision_external_bytes_and_same_byte_replacement_before_effects() {
    if crate::test_isolation::isolated(
        "project_checkpoint_tests::save_refuses_stale_revision_external_bytes_and_same_byte_replacement_before_effects",
    ) {
        return;
    }
    for kind in ["revision", "bytes", "node"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("game.incant");
        fs::write(&path, legacy().canonical_text().unwrap()).unwrap();
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        edit(&mut bus, "unsaved");
        let expected = if kind == "revision" {
            0
        } else {
            bus.revision()
        };
        if kind == "bytes" {
            fs::write(&path, "external").unwrap();
        }
        if kind == "node" {
            let bytes = fs::read(&path).unwrap();
            fs::rename(&path, dir.path().join("retired")).unwrap();
            fs::write(&path, bytes).unwrap();
        }
        let bytes = fs::read(&path).unwrap();
        let log = journal_bytes(&mut bus);
        let revision = bus.revision();
        assert!(
            bus.save_project_file(expected, Actor::user("test"))
                .is_err()
        );
        assert_eq!(bus.revision(), revision);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(journal_bytes(&mut bus), log);
        assert!(
            !dir.path()
                .join(".incant-source-transactions/project.pending")
                .exists()
        );
    }
}
#[test]
fn file_requests_are_closed_and_no_capability_is_deserialized() {
    if crate::test_isolation::isolated(
        "project_checkpoint_tests::file_requests_are_closed_and_no_capability_is_deserialized",
    ) {
        return;
    }
    for value in [
        serde_json::json!({"operation":"save","expected_revision":0,"path":"evil"}),
        serde_json::json!({"operation":"apply_pin","expected_revision":0,"review_id":new_id(),"candidate_text":"{}"}),
        serde_json::json!({"operation":"review_pin","expected_revision":0,"backup":"evil"}),
        serde_json::json!({"operation":"save"}),
    ] {
        assert!(serde_json::from_value::<ProjectFileRequest>(value).is_err());
    }
    let mut bus = CommandBus::new(legacy()).unwrap();
    assert!(
        bus.project_file_request(
            ProjectFileRequest::ReviewPin {
                expected_revision: 0
            },
            Actor::user("test")
        )
        .is_err()
    );
    assert!(save(&mut bus).is_err());
    assert_eq!(bus.revision(), 0);
}
#[test]
fn checkpoint_crash_child() {
    let Some(root) = std::env::var_os("INCANT_CHECKPOINT_CHILD") else {
        return;
    };
    let mut bus = CommandBus::open_project_file(Path::new(&root).join("game.incant")).unwrap();
    let log = journal_bytes(&mut bus);
    let project = bus.project().clone();
    let revision = bus.revision();
    let result = save(&mut bus);
    if let Ok(phase) = std::env::var("INCANT_PROJECT_FAIL") {
        assert!(result.is_err());
        assert_eq!(bus.project(), &project);
        assert_eq!(bus.revision(), revision);
        if phase.starts_with("poison_") {
            assert!(bus.ensure_sources_ready().is_err());
        } else {
            assert!(journal_bytes(&mut bus).starts_with(&log));
        }
        std::process::exit(74);
    }
    panic!("crash phase not reached: {result:?}");
}
fn prepared(root: &Path) -> Vec<u8> {
    let path = root.join("game.incant");
    let before = legacy().canonical_text().unwrap();
    fs::write(&path, &before).unwrap();
    let mut bus = CommandBus::open_project_file(&path).unwrap();
    edit(&mut bus, "durable unsaved note");
    before.into_bytes()
}
fn child(root: &Path) -> Process {
    let mut p = Process::new(std::env::current_exe().unwrap());
    p.args([
        "--exact",
        "project_checkpoint_tests::checkpoint_crash_child",
        "--nocapture",
    ])
    .env("INCANT_CHECKPOINT_CHILD", root)
    .env("INCANT_PROJECT_TEST_ROOT", root);
    p
}
#[test]
fn checkpoint_save_crashes_and_partial_errors_recover_without_logical_advance() {
    if crate::test_isolation::isolated(
        "project_checkpoint_tests::checkpoint_save_crashes_and_partial_errors_recover_without_logical_advance",
    ) {
        return;
    }
    for (env, phases, exit) in [
        (
            "INCANT_PROJECT_PROTOCOL_CRASH",
            &[
                "marker",
                "partial_prepare",
                "prepare",
                "displaced",
                "installed",
                "partial_commit",
                "commit",
                "finish",
            ][..],
            73,
        ),
        (
            "INCANT_PROJECT_FAIL",
            &["prepare", "commit", "poison_prepare", "poison_commit"][..],
            74,
        ),
    ] {
        for phase in phases {
            let root = tempfile::tempdir().unwrap();
            let before = prepared(root.path());
            assert_eq!(
                child(root.path()).env(env, phase).status().unwrap().code(),
                Some(exit),
                "{phase}"
            );
            let path = root.path().join("game.incant");
            if *phase == "displaced" {
                assert!(!path.exists());
            }
            let mut bus = CommandBus::open_project_file(&path).unwrap();
            assert_eq!(bus.revision(), 1);
            assert_eq!(bus.history().len(), 1);
            let committed = matches!(*phase, "commit" | "finish") && exit == 73;
            assert_eq!(
                fs::read(&path).unwrap(),
                if committed {
                    bus.project().canonical_text().unwrap().into_bytes()
                } else {
                    before
                }
            );
            bus.check_project_file().unwrap();
            save(&mut bus).unwrap();
            bus.undo().unwrap();
            assert_eq!(bus.revision(), 2);
            save(&mut bus).unwrap();
        }
    }
}
#[test]
fn checkpoint_save_cannot_replace_causal_state_with_equal_projection() {
    if crate::test_isolation::isolated(
        "project_checkpoint_tests::checkpoint_save_cannot_replace_causal_state_with_equal_projection",
    ) {
        return;
    }
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let root = tempfile::tempdir().unwrap();
    prepared(root.path());
    let path = root.path().join("game.incant");
    assert_eq!(
        child(root.path())
            .env("INCANT_PROJECT_PROTOCOL_CRASH", "prepare")
            .status()
            .unwrap()
            .code(),
        Some(73)
    );
    let logpath = root.path().join("game.journal");
    let bytes = fs::read(&logpath).unwrap();
    let mut entries: Vec<journal::Entry> = bytes
        .split(|b| *b == b'\n')
        .filter(|s| !s.is_empty())
        .map(|s| serde_json::from_slice(s).unwrap())
        .collect();
    let prepare = entries.last_mut().unwrap();
    let journal::Action::CheckpointPrepare { edit, .. } = &prepare.action else {
        panic!("checkpoint")
    };
    prepare.crdt = STANDARD.encode(
        CollaborativeDocument::new(&Project::from_text(&edit.after).unwrap())
            .unwrap()
            .export()
            .unwrap(),
    );
    prepare.digest = journal::digest(
        prepare.version,
        prepare.sequence,
        &prepare.previous,
        &prepare.action,
        &prepare.crdt,
    )
    .unwrap();
    let forged: Vec<u8> = entries
        .iter()
        .flat_map(|e| {
            let mut b = serde_json::to_vec(e).unwrap();
            b.push(b'\n');
            b
        })
        .collect();
    fs::write(&logpath, &forged).unwrap();
    let original = fs::read(&path).unwrap();
    assert!(CommandBus::open_project_file(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read(logpath).unwrap(), forged);
    assert!(
        root.path()
            .join(".incant-source-transactions/project.pending")
            .exists()
    );
}

#[test]
fn legacy_pending_script_recovers_under_exact_custom_journal_binding() {
    if crate::test_isolation::isolated(
        "project_checkpoint_tests::legacy_pending_script_recovers_under_exact_custom_journal_binding",
    ) {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let path = root.path().join("game.incant.json");
    let journal = logs.path().join("legacy.jsonl");
    let mut p = legacy();
    let script = incant_doc::ScriptSource {
        id: new_id(),
        name: "Script".into(),
        path: "script.ts".into(),
    };
    p.scripts.insert(script.id.clone(), script.clone());
    fs::write(&path, p.canonical_text().unwrap()).unwrap();
    fs::write(root.path().join("script.ts"), "before").unwrap();
    let mut old = CommandBus::persistent_with_sources(&journal, p.clone(), root.path()).unwrap();
    let operation = new_id();
    let edit = incant_source::SourceEdit {
        script_id: script.id.clone(),
        path: script.path.clone(),
        before: Some("before".into()),
        after: Some("after".into()),
    };
    let transaction = Transaction {
        id: new_id(),
        description: "interrupted script".into(),
        actor: Actor::user("test"),
        commands: vec![Command::WriteScript {
            script_id: script.id.clone(),
            expected_source: SourceVersion::of(Some("before")),
            source: "after".into(),
        }],
        before: p.clone(),
        after: p,
        source_edit: Some(edit.clone()),
        project_edit: None,
    };
    old.sources
        .as_ref()
        .unwrap()
        .begin(&operation, &edit)
        .unwrap();
    let crdt = old.export_crdt().unwrap();
    old.journal
        .as_mut()
        .unwrap()
        .append(
            journal::Action::Prepare {
                operation: operation.clone(),
                edit: edit.clone(),
                change: journal::Change::Apply {
                    transaction: Box::new(transaction),
                },
            },
            &crdt,
        )
        .unwrap();
    old.sources
        .as_ref()
        .unwrap()
        .install(&operation, &edit)
        .unwrap();
    drop(old);
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    assert_eq!(
        fs::read_to_string(root.path().join("script.ts")).unwrap(),
        "before"
    );
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    assert_eq!(
        bus.read_script(&script.id).unwrap().source.as_deref(),
        Some("before")
    );
    let review = bus.review_project_file_pin().unwrap();
    bus.apply_project_file_pin(&review.review_id, Actor::user("test"))
        .unwrap();
    save(&mut bus).unwrap();
    drop(bus);
    let bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    assert_eq!(bus.revision(), 1);
    assert!(bus.project().engine_version.is_some());
}

#[test]
fn checkpoint_capacity_and_custom_journal_replacement_refuse_before_backup() {
    if crate::test_isolation::isolated(
        "project_checkpoint_tests::checkpoint_capacity_and_custom_journal_replacement_refuse_before_backup",
    ) {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let path = root.path().join("game.incant.json");
    let journal = logs.path().join("custom.jsonl");
    fs::write(&path, legacy().canonical_text().unwrap()).unwrap();
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    edit(&mut bus, "new");
    let before = fs::read(&path).unwrap();
    // Sparse logical size exercises the cap without allocating a 256MiB input.
    let file = bus.journal.as_mut().unwrap().test_file();
    let size = file.metadata().unwrap().len();
    file.set_len(journal::MAX_PROJECT_JOURNAL_BYTES as u64)
        .unwrap();
    assert!(save(&mut bus).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(bus.revision(), 1);
    bus.journal
        .as_mut()
        .unwrap()
        .test_file()
        .set_len(size)
        .unwrap();
    assert!(
        !root
            .path()
            .join(".incant-source-transactions/project.pending")
            .exists()
    );
    // Windows intentionally prevents replacing the exclusively locked journal;
    // Unix can rename it, so test that the separate retained directory detects it.
    #[cfg(unix)]
    {
        let bytes = journal_bytes(&mut bus);
        fs::rename(&journal, logs.path().join("retired")).unwrap();
        fs::write(&journal, &bytes).unwrap();
        assert!(save(&mut bus).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read(&journal).unwrap(), bytes);
    }
}
