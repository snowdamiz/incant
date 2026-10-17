//! Cold admission controls; fake tool bytes are never executed.
use serde as serde_support;
use serde_json as json;
#[path = "../../../third_party/rquickjs-sys/build_support/diagnostic_pgo.rs"]
mod pgo;
use pgo::{Content, Descriptor, DigestFile, Mode, Recipe};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

fn file(path: &Path, data: &[u8]) -> DigestFile {
    fs::write(path, data).unwrap();
    DigestFile {
        path: path.into(),
        bytes: data.len() as u64,
        sha256: format!("{:x}", Sha256::digest(data)),
    }
}
fn fixture(root: &Path) -> Descriptor {
    fs::create_dir(root.join("quickjs")).unwrap();
    fs::create_dir(root.join("sdk")).unwrap();
    fs::create_dir(root.join("resource")).unwrap();
    let sources = pgo::SOURCES
        .into_iter()
        .map(|name| {
            let f = file(&root.join("quickjs").join(name), name.as_bytes());
            (
                name.into(),
                Content {
                    bytes: f.bytes,
                    sha256: f.sha256,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    Descriptor {
        version: 1,
        mode: Mode::Use,
        target: pgo::TARGET.into(),
        compiler: file(&root.join("clang"), b"compiler"),
        archiver: file(&root.join("ar"), b"archiver"),
        profdata_tool: file(&root.join("profdata"), b"profdata"),
        profile_runtime: file(&root.join("libclang_rt.profile_osx.a"), b"runtime"),
        sdk: root.join("sdk"),
        resource_directory: root.join("resource"),
        tool_inputs: vec![
            file(&root.join("sdk/metadata"), b"SDK"),
            file(&root.join("resource/metadata"), b"resource"),
        ],
        sources,
        recipe: Recipe {
            optimization: 3,
            debug: true,
            base_args: vec!["-O3".into(), "-g".into(), "-D_GNU_SOURCE".into()],
        },
        profile: Some(file(&root.join("actual.profdata"), b"profile A")),
        raw_directory: None,
    }
}
fn parse(v: &Descriptor) -> Descriptor {
    Descriptor::parse(&json::to_vec(v).unwrap(), v.mode).unwrap()
}

#[test]
fn pgo_session_cc_evidence_setting_does_not_admit_compiler_overrides() {
    use std::ffi::OsStr;
    pgo::compiler_environment("CC_ENABLE_DEBUG_OUTPUT", OsStr::new("1")).unwrap();
    pgo::compiler_environment("CC_ENABLE_DEBUG_OUTPUT", OsStr::new("")).unwrap();
    for value in ["0", "true", "1 ", "-O3"] {
        assert!(pgo::compiler_environment("CC_ENABLE_DEBUG_OUTPUT", OsStr::new(value)).is_err());
    }
    for key in [
        "CC_ENABLE_DEBUG_OUTPUT_EXTRA",
        "CC",
        "CC_aarch64_apple_darwin",
        "AR",
        "CFLAGS",
        "HOST_CFLAGS",
        "CPPFLAGS",
        "LLVM_PROFILE_FILE",
        "RUSTFLAGS",
        "CARGO_PROFILE_SHIPPING_OPT_LEVEL",
    ] {
        assert!(
            pgo::compiler_environment(key, OsStr::new("1")).is_err(),
            "{key}"
        );
        pgo::compiler_environment(key, OsStr::new("")).unwrap();
    }
}

#[test]
fn pgo_exact_profile_bytes_change_compatibility_while_relocation_does_not() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let a = fixture(&root);
    parse(&a).verify(&root).unwrap();
    let mut b = a.clone();
    b.profile = Some(file(&root.join("relocated.profdata"), b"profile A"));
    assert_eq!(a.identity().unwrap(), b.identity().unwrap());
    b.profile = Some(file(&root.join("changed.profdata"), b"profile B"));
    b.verify(&root).unwrap();
    assert_ne!(a.identity().unwrap(), b.identity().unwrap());
    fs::write(&a.profile.as_ref().unwrap().path, b"profile B").unwrap();
    assert!(
        a.verify(&root).is_err(),
        "claimed digest cannot admit changed bytes"
    );
}
#[test]
fn pgo_mode_tool_and_source_recipes_have_distinct_identities_and_refusals() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let a = fixture(&root);
    let before = a.identity().unwrap();
    let mut b = a.clone();
    b.compiler = file(&root.join("other-clang"), b"other compiler");
    assert_ne!(before, b.identity().unwrap());
    b = a.clone();
    fs::create_dir(root.join("raw")).unwrap();
    b.mode = Mode::Generate;
    b.profile = None;
    b.raw_directory = Some(root.join("raw"));
    parse(&b).verify(&root).unwrap();
    assert_ne!(before, b.identity().unwrap());
    assert!(Descriptor::parse(&json::to_vec(&b).unwrap(), Mode::Use).is_err());
    fs::write(root.join("quickjs/quickjs.c"), b"changed source").unwrap();
    assert!(a.verify(&root).is_err());
}
#[test]
fn pgo_closed_schema_rejects_duplicate_unknown_missing_and_oversized_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let a = fixture(&root);
    let mut v = json::to_value(&a).unwrap();
    v["unknown"] = json::json!(true);
    assert!(Descriptor::parse(&json::to_vec(&v).unwrap(), Mode::Use).is_err());
    let raw =
        json::to_string(&a)
            .unwrap()
            .replacen("\"version\":1", "\"version\":1,\"version\":1", 1);
    assert!(Descriptor::parse(raw.as_bytes(), Mode::Use).is_err());
    let source = json::to_string(&a.sources["quickjs.c"]).unwrap();
    let key = format!("\"quickjs.c\":{source}");
    let raw = json::to_string(&a)
        .unwrap()
        .replacen(&key, &format!("{key},{key}"), 1);
    assert!(Descriptor::parse(raw.as_bytes(), Mode::Use).is_err());
    v = json::to_value(&a).unwrap();
    v.as_object_mut().unwrap().remove("raw_directory");
    assert!(Descriptor::parse(&json::to_vec(&v).unwrap(), Mode::Use).is_err());
    assert!(Descriptor::parse(&vec![b' '; pgo::MAX_DESCRIPTOR as usize + 1], Mode::Use).is_err());
    let large = root.join("large");
    fs::write(&large, [0u8; 65]).unwrap();
    assert!(pgo::bounded(&large, 64).is_err());
    let mut b = a.clone();
    b.target = "aarch64-linux-android".into();
    assert!(Descriptor::parse(&json::to_vec(&b).unwrap(), Mode::Use).is_err());
}

#[test]
fn pgo_missing_profile_changed_header_and_tool_and_oversize_profile_refuse() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let a = fixture(&root);
    let mut b = a.clone();
    b.profile = None;
    assert!(Descriptor::parse(&json::to_vec(&b).unwrap(), Mode::Use).is_err());
    let mut b = a.clone();
    b.profile.as_mut().unwrap().bytes = pgo::MAX_PROFILE + 1;
    assert!(b.verify(&root).is_err());
    fs::write(&a.compiler.path, b"changed compiler").unwrap();
    assert!(a.verify(&root).is_err());
    fs::write(&a.compiler.path, b"compiler").unwrap();
    fs::write(root.join("quickjs/quickjs.h"), b"changed header").unwrap();
    assert!(a.verify(&root).is_err());
}

#[test]
fn pgo_canonical_identity_matches_the_cold_python_protocol() {
    // This fixed vector is also asserted by tools.tests.test_quickjs_pgo.
    // No path in it is executed or used as input authority.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut a = fixture(&root);
    a.sdk = "/pinned/sdk".into();
    a.resource_directory = "/pinned/resource".into();
    for f in [
        &mut a.compiler,
        &mut a.archiver,
        &mut a.profdata_tool,
        &mut a.profile_runtime,
    ] {
        f.bytes = 5;
        f.sha256 = "1".repeat(64);
    }
    a.profile.as_mut().unwrap().bytes = 7;
    a.profile.as_mut().unwrap().sha256 = "2".repeat(64);
    a.tool_inputs = vec![
        DigestFile {
            path: "/pinned/sdk/SDKSettings.json".into(),
            bytes: 3,
            sha256: "3".repeat(64),
        },
        DigestFile {
            path: "/pinned/resource/metadata".into(),
            bytes: 8,
            sha256: "4".repeat(64),
        },
    ];
    assert_eq!(
        pgo::hex(&a.identity().unwrap()),
        "f38a676e755b1f1cde6fc258e1d622304e2f68689642a0cbea11064d7fd58c63"
    );
}
#[test]
fn pgo_ordered_flag_admission_preserves_strict_warnings_and_rejects_suppression() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let a = fixture(&root);
    let mut actual = a.recipe.base_args.clone();
    let end = actual.len() - 1;
    actual.splice(end..end, a.additions());
    a.check_args(&actual, true).unwrap();
    assert!(actual.contains(&"-Werror=backend-plugin".into()));
    assert!(actual.contains(&"-Werror=unknown-warning-option".into()));
    actual.swap(0, 1);
    assert!(a.check_args(&actual, true).is_err());
    let mut b = a.clone();
    b.recipe.base_args.insert(0, "-w".into());
    assert!(Descriptor::parse(&json::to_vec(&b).unwrap(), Mode::Use).is_err());
    actual = a.recipe.base_args.clone();
    actual.extend(a.additions());
    assert!(
        a.check_args(&actual, true).is_err(),
        "PGO flags must occur at the observed cc slot"
    );
}
#[cfg(unix)]
#[test]
fn pgo_linked_profile_and_linked_parent_are_refused() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let a = fixture(&root);
    let path = root.join("link.profdata");
    symlink(a.profile.as_ref().unwrap().path.clone(), &path).unwrap();
    let mut b = a.clone();
    b.profile.as_mut().unwrap().path = path;
    assert!(b.verify(&root).is_err());
    symlink(&root, root.join("parent-link")).unwrap();
    b.profile.as_mut().unwrap().path = root.join("parent-link/actual.profdata");
    assert!(b.verify(&root).is_err());
}

#[test]
fn pgo_nondefault_c_recipe_keeps_cross_target_bytecode_admission_closed() {
    use incant_script::artifact::compatibility;
    let mut abi = incant_script::artifact::BytecodeAbi::current();
    abi.default_c_flags = false;
    for target in [
        "aarch64-apple-ios",
        "aarch64-apple-ios-sim",
        "aarch64-linux-android",
    ] {
        assert!(!compatibility::admits(pgo::TARGET, target, abi));
    }
    assert!(compatibility::admits(pgo::TARGET, pgo::TARGET, abi));
}
