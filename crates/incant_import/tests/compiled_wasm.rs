use incant_cmd::{Actor, BinaryVersion, CommandBus, SourceVersion};
use incant_doc::{Project, RustModuleSource, new_id};
use incant_import::{CompiledWasmRequest, ImportRequest, ImportSnapshot, PreparedCompiledWasm};
use std::{fs, path::Path};
const SOURCE: &str = "modules/game/module.rs";
const TEXT: &str = "pub fn update() {}";
fn open(root: &Path) -> (CommandBus, String) {
    let mut p = Project::empty("Prepared Wasm");
    let id = new_id();
    p.module_sources.insert(
        id.clone(),
        RustModuleSource {
            id: id.clone(),
            name: "Game".into(),
            path: SOURCE.into(),
        },
    );
    fs::create_dir_all(root.join("modules/game")).unwrap();
    fs::write(root.join(SOURCE), TEXT).unwrap();
    (
        CommandBus::persistent_with_sources(root.join("journal"), p, root).unwrap(),
        id,
    )
}
fn raw() -> Vec<u8> {
    wat::parse_str(r#"(module (memory (export "memory") 1) (func (export "incant_abi") (result i32) i32.const 1) (func (export "update") (param i64 f64)))"#).unwrap()
}
fn request(module: &str, revision: u64, destination: BinaryVersion) -> CompiledWasmRequest {
    CompiledWasmRequest {
        module_id: module.into(),
        expected_revision: revision,
        expected_source: SourceVersion::of(Some(TEXT)),
        expected_destination: destination,
    }
}
#[test]
fn prepared_bytes_use_ordinary_wasm_admission_and_remain_reimportable_after_reopen() {
    let root = tempfile::tempdir().unwrap();
    let candidate = tempfile::tempdir().unwrap();
    let (mut bus, module) = open(root.path());
    fs::write(candidate.path().join("module.wasm"), raw()).unwrap();
    let prepared = PreparedCompiledWasm::prepare(
        &bus,
        root.path(),
        request(&module, 0, BinaryVersion::Missing),
        &fs::read(candidate.path().join("module.wasm")).unwrap(),
    )
    .unwrap();
    let asset = prepared.outcome().asset.clone();
    assert!(!root.path().join(&asset.path).exists());
    fs::remove_file(candidate.path().join("module.wasm")).unwrap();
    prepared
        .commit(&mut bus, Actor::user("install"), "Install compiled bytes")
        .unwrap();
    let ordinary = ImportSnapshot::capture(&bus)
        .prepare(
            root.path(),
            None,
            &[ImportRequest {
                source: asset.path.clone(),
                texture_usage: None,
            }],
        )
        .unwrap();
    assert_eq!(ordinary.outcomes()[0].asset, asset);
    assert!(!ordinary.outcomes()[0].changed);
    let initial = bus.history()[0].before.clone();
    drop(bus);
    let mut bus =
        CommandBus::persistent_with_sources(root.path().join("journal"), initial, root.path())
            .unwrap();
    bus.undo().unwrap();
    assert!(!root.path().join(&asset.path).exists());
    bus.redo().unwrap();
    assert_eq!(fs::read(root.path().join(&asset.path)).unwrap(), raw());
    assert_eq!(bus.project().assets[&asset.id], asset);
}
#[test]
fn invalid_raw_module_or_root_never_publishes_an_asset_or_source() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let (bus, module) = open(root.path());
    for bytes in [
        b"not wasm".to_vec(),
        b"\0asm\x01\0\0\0".to_vec(),
        vec![0; 1024 * 1024 + 1],
    ] {
        assert!(
            PreparedCompiledWasm::prepare(
                &bus,
                root.path(),
                request(&module, 0, BinaryVersion::Missing),
                &bytes
            )
            .is_err()
        );
    }
    assert!(
        PreparedCompiledWasm::prepare(
            &bus,
            other.path(),
            request(&module, 0, BinaryVersion::Missing),
            &raw()
        )
        .is_err()
    );
    assert!(!root.path().join("modules/game/module.wasm").exists());
    assert!(bus.project().assets.is_empty());
    assert_eq!(bus.revision(), 0);
}
#[test]
fn prepared_import_refuses_changed_source_and_destination_without_history() {
    for source_changed in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let (mut bus, module) = open(root.path());
        let prepared = PreparedCompiledWasm::prepare(
            &bus,
            root.path(),
            request(&module, 0, BinaryVersion::Missing),
            &raw(),
        )
        .unwrap();
        let path = if source_changed {
            SOURCE
        } else {
            "modules/game/module.wasm"
        };
        fs::write(root.path().join(path), b"external").unwrap();
        assert!(
            prepared
                .commit(&mut bus, Actor::user("install"), "Stale")
                .is_err()
        );
        assert_eq!(fs::read(root.path().join(path)).unwrap(), b"external");
        assert!(bus.history().is_empty());
        assert!(bus.project().assets.is_empty());
    }
}

#[test]
fn abandoning_preparation_is_side_effect_free_for_authored_state() {
    let root = tempfile::tempdir().unwrap();
    let (bus, module) = open(root.path());
    let before = bus.project().clone();
    let prepared = PreparedCompiledWasm::prepare(
        &bus,
        root.path(),
        request(&module, 0, BinaryVersion::Missing),
        &raw(),
    )
    .unwrap();
    drop(prepared); // The caller cancelled before the only publication boundary.
    assert_eq!(bus.project(), &before);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    assert!(!root.path().join("modules/game/module.wasm").exists());
    assert_eq!(fs::read(root.path().join(SOURCE)).unwrap(), TEXT.as_bytes());
}
