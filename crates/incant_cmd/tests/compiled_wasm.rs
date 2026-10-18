use incant_cmd::{Actor, BinaryVersion, Command, CommandBus, PreparedWasmAsset, SourceVersion};
use incant_doc::{Asset, Project, RustModuleSource, new_id};
use std::{fs, path::Path};
const SOURCE: &str = "modules/game/module.rs";
const DEST: &str = "modules/game/module.wasm";
const TEXT: &str = "pub fn update() {}";
const RAW: &[u8] = b"\0asm\x01\0\0\0candidate";
fn initial() -> (Project, String) {
    let mut project = Project::empty("Compiled module");
    let id = new_id();
    project.module_sources.insert(
        id.clone(),
        RustModuleSource {
            id: id.clone(),
            name: "Game".into(),
            path: SOURCE.into(),
        },
    );
    (project, id)
}
fn setup(root: &Path, project: &Project) {
    fs::create_dir_all(root.join("modules/game")).unwrap();
    fs::write(root.join(SOURCE), TEXT).unwrap();
    fs::write(root.join("game.incant"), project.canonical_text().unwrap()).unwrap();
}
fn asset(id: &str) -> Asset {
    Asset {
        id: id.into(),
        name: "Game".into(),
        path: DEST.into(),
        kind: "wasm".into(),
        sha256: "a".repeat(64),
        import_settings: None,
    }
}
fn prepare(bus: &CommandBus, module: &str, id: &str, bytes: &[u8]) -> PreparedWasmAsset {
    bus.prepare_wasm_asset(
        module,
        SourceVersion::of(Some(TEXT)),
        bus.module_binary_version(module).unwrap(),
        asset(id),
        bytes,
    )
    .unwrap()
}
#[test]
fn binary_asset_and_bytes_reopen_undo_redo_without_ephemeral_candidate_or_activation() {
    let root = tempfile::tempdir().unwrap();
    let candidate = tempfile::tempdir().unwrap();
    let (project, module) = initial();
    setup(root.path(), &project);
    let id = new_id();
    let path = root.path().join("game.incant");
    fs::write(candidate.path().join("module.wasm"), RAW).unwrap();
    let tx = {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        let prepared = prepare(
            &bus,
            &module,
            &id,
            &fs::read(candidate.path().join("module.wasm")).unwrap(),
        );
        fs::remove_file(candidate.path().join("module.wasm")).unwrap();
        assert!(!root.path().join(DEST).exists());
        let tx = bus
            .publish_wasm_asset(
                prepared,
                0,
                Actor::agent("model", "install"),
                "Install compiled module",
            )
            .unwrap()
            .clone();
        assert!(tx.binary_edit.is_some());
        assert!(tx.source_edit.is_none());
        assert_eq!(bus.project().scenes, project.scenes);
        bus.save_project_file(1, Actor::user("save")).unwrap();
        tx
    };
    {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        assert_eq!(bus.history(), std::slice::from_ref(&tx));
        assert_eq!(fs::read(root.path().join(DEST)).unwrap(), RAW);
        bus.undo().unwrap();
        assert_eq!(bus.project(), &project);
        assert!(!root.path().join(DEST).exists());
        assert_eq!(fs::read_to_string(root.path().join(SOURCE)).unwrap(), TEXT);
    }
    let mut bus = CommandBus::open_project_file(&path).unwrap();
    bus.redo().unwrap();
    assert_eq!(fs::read(root.path().join(DEST)).unwrap(), RAW);
    assert_eq!(bus.history(), &[tx]);
    bus.undo().unwrap();
    assert!(!root.path().join(DEST).exists());
}
#[test]
fn binary_preparation_refuses_stale_source_destination_revision_and_reopened_bus() {
    for case in ["source", "destination", "revision", "session"] {
        let root = tempfile::tempdir().unwrap();
        let (project, module) = initial();
        setup(root.path(), &project);
        let path = root.path().join("game.incant");
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        let prepared = prepare(&bus, &module, &new_id(), RAW);
        match case {
            "source" => fs::write(root.path().join(SOURCE), "external").unwrap(),
            "destination" => fs::write(root.path().join(DEST), "foreign").unwrap(),
            "revision" => {
                bus.execute(
                    vec![Command::SetMemory {
                        section: "note".into(),
                        text: "changed".into(),
                    }],
                    Actor::user("edit"),
                    "Other edit",
                    Some(0),
                )
                .unwrap();
            }
            "session" => {
                drop(bus);
                bus = CommandBus::open_project_file(&path).unwrap();
            }
            _ => unreachable!(),
        }
        let before = bus.project().clone();
        let revision = bus.revision();
        assert!(
            bus.publish_wasm_asset(prepared, 0, Actor::user("install"), "Refuse")
                .is_err(),
            "{case}"
        );
        assert_eq!(bus.project(), &before);
        assert_eq!(bus.revision(), revision);
        if case == "destination" {
            assert_eq!(fs::read(root.path().join(DEST)).unwrap(), b"foreign");
        } else {
            assert!(!root.path().join(DEST).exists());
        }
    }
}
#[test]
fn binary_updates_undo_to_exact_previous_node_and_refuse_same_content_replacements() {
    let root = tempfile::tempdir().unwrap();
    let (project, module) = initial();
    setup(root.path(), &project);
    let mut bus = CommandBus::open_project_file(root.path().join("game.incant")).unwrap();
    let id = new_id();
    let prepared = prepare(&bus, &module, &id, RAW);
    bus.publish_wasm_asset(prepared, 0, Actor::user("install"), "First")
        .unwrap();
    let raw2 = b"\0asm\x01\0\0\0second";
    let prepared = prepare(&bus, &module, &id, raw2);
    bus.publish_wasm_asset(prepared, 1, Actor::user("install"), "Second")
        .unwrap();
    bus.undo().unwrap();
    assert_eq!(fs::read(root.path().join(DEST)).unwrap(), RAW);
    bus.undo().unwrap();
    assert!(!root.path().join(DEST).exists());
    bus.redo().unwrap();
    bus.redo().unwrap();
    fs::rename(root.path().join(DEST), root.path().join("old.wasm")).unwrap();
    fs::write(root.path().join(DEST), raw2).unwrap();
    let revision = bus.revision();
    assert!(bus.undo().is_err());
    assert_eq!(bus.revision(), revision);
    assert_eq!(fs::read(root.path().join(DEST)).unwrap(), raw2);
    assert_eq!(fs::read(root.path().join("old.wasm")).unwrap(), raw2);
}
#[test]
fn binary_install_cannot_choose_unrelated_paths_or_replace_unregistered_content() {
    let root = tempfile::tempdir().unwrap();
    let (project, module) = initial();
    setup(root.path(), &project);
    let bus = CommandBus::open_project_file(root.path().join("game.incant")).unwrap();
    for path in ["unrelated.wasm", "../outside.wasm", SOURCE] {
        let mut wrong = asset(&new_id());
        wrong.path = path.into();
        assert!(
            bus.prepare_wasm_asset(
                &module,
                SourceVersion::of(Some(TEXT)),
                BinaryVersion::Missing,
                wrong,
                RAW
            )
            .is_err()
        );
    }
    fs::write(root.path().join(DEST), RAW).unwrap();
    assert!(
        bus.prepare_wasm_asset(
            &module,
            SourceVersion::of(Some(TEXT)),
            BinaryVersion::of(Some(RAW)),
            asset(&new_id()),
            RAW
        )
        .is_err()
    );
    assert_eq!(fs::read(root.path().join(DEST)).unwrap(), RAW);
    assert_eq!(bus.revision(), 0);
}

#[test]
fn package_and_binary_ownership_compose_across_reopen_and_candidate_removal() {
    let root = tempfile::tempdir().unwrap();
    let candidate = tempfile::tempdir().unwrap();
    let initial = Project::empty("First generated package");
    let module = new_id();
    let asset_id = new_id();
    let path = root.path().join("game.incant");
    fs::write(&path, initial.canonical_text().unwrap()).unwrap();
    fs::write(candidate.path().join("candidate.wasm"), RAW).unwrap();
    {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        bus.execute(
            vec![
                Command::UpsertModuleSource {
                    module: RustModuleSource {
                        id: module.clone(),
                        name: "Game".into(),
                        path: SOURCE.into(),
                    },
                },
                Command::WriteModuleSource {
                    module_id: module.clone(),
                    expected_source: SourceVersion::Missing,
                    source: TEXT.into(),
                },
            ],
            Actor::user("author"),
            "Create package",
            Some(0),
        )
        .unwrap();
        let prepared = prepare(
            &bus,
            &module,
            &asset_id,
            &fs::read(candidate.path().join("candidate.wasm")).unwrap(),
        );
        fs::remove_file(candidate.path().join("candidate.wasm")).unwrap();
        bus.publish_wasm_asset(prepared, 1, Actor::user("install"), "Install")
            .unwrap();
        bus.save_project_file(2, Actor::user("save")).unwrap();
    }
    {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        bus.undo().unwrap();
        assert!(!root.path().join(DEST).exists());
        assert_eq!(fs::read(root.path().join(SOURCE)).unwrap(), TEXT.as_bytes());
        bus.undo().unwrap();
        assert!(!root.path().join("modules").exists());
        assert_eq!(bus.project(), &initial);
    }
    {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        bus.redo().unwrap();
        assert_eq!(fs::read(root.path().join(SOURCE)).unwrap(), TEXT.as_bytes());
        assert!(!root.path().join(DEST).exists());
        bus.redo().unwrap();
        assert_eq!(fs::read(root.path().join(DEST)).unwrap(), RAW);
        assert_eq!(bus.project().assets[&asset_id].path, DEST);
        assert_eq!(bus.project().scenes, initial.scenes);
        bus.undo().unwrap();
        bus.undo().unwrap();
        assert!(!root.path().join("modules").exists());
    }
}

#[test]
fn text_and_binary_history_preserves_legacy_prefix_and_undo_order() {
    let root = tempfile::tempdir().unwrap();
    let (mut project, module) = initial();
    let script = new_id();
    project.scripts.insert(
        script.clone(),
        incant_doc::ScriptSource {
            id: script.clone(),
            name: "Main".into(),
            path: "main.ts".into(),
        },
    );
    setup(root.path(), &project);
    fs::write(root.path().join("main.ts"), "before").unwrap();
    let journal = root.path().join("legacy.jsonl");
    let open =
        || CommandBus::persistent_with_sources(&journal, project.clone(), root.path()).unwrap();
    {
        let mut bus = open();
        let tx = bus
            .execute(
                vec![Command::WriteScript {
                    script_id: script,
                    expected_source: SourceVersion::of(Some("before")),
                    source: "after".into(),
                }],
                Actor::user("author"),
                "Legacy text transaction",
                Some(0),
            )
            .unwrap();
        let wire = serde_json::to_value(tx).unwrap();
        assert!(wire.get("binary_edit").is_none());
    }
    let prefix = fs::read(&journal).unwrap();
    for line in prefix
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
    {
        let entry: serde_json::Value = serde_json::from_slice(line).unwrap();
        assert!(matches!(entry["version"].as_u64(), Some(1 | 2)));
    }
    {
        let mut bus = open();
        let prepared = prepare(&bus, &module, &new_id(), RAW);
        bus.publish_wasm_asset(prepared, 1, Actor::user("install"), "Binary")
            .unwrap();
    }
    assert!(fs::read(&journal).unwrap().starts_with(&prefix));
    let mut bus = open();
    bus.undo().unwrap();
    assert!(!root.path().join(DEST).exists());
    assert_eq!(fs::read(root.path().join("main.ts")).unwrap(), b"after");
    bus.undo().unwrap();
    assert_eq!(fs::read(root.path().join("main.ts")).unwrap(), b"before");
    bus.redo().unwrap();
    bus.redo().unwrap();
    assert_eq!(fs::read(root.path().join("main.ts")).unwrap(), b"after");
    assert_eq!(fs::read(root.path().join(DEST)).unwrap(), RAW);
}
