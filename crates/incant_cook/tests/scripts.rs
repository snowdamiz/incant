#![cfg(feature = "script")]
use incant_cook::{ScriptCookError, cook_script};
use incant_script::artifact;

const SOURCE: &str = "exports.default = {update() {}};";

#[test]
fn cook_is_deterministic_and_retains_separate_symbol_metadata() {
    let map = r#"{"version":3,"sources":["motion.ts"],"names":[],"mappings":"AAAA","sourcesContent":["// original TypeScript"]}"#;
    let first = cook_script("motion.js", SOURCE, Some(map), artifact::TARGET).unwrap();
    let second = cook_script("motion.js", SOURCE, Some(map), artifact::TARGET).unwrap();
    assert_eq!(first.rust_module, second.rust_module);
    assert_eq!(first.symbol_manifest, second.symbol_manifest);
    assert_eq!(first.source_map.as_deref(), Some(map));
    let manifest: serde_json::Value = serde_json::from_str(&first.symbol_manifest).unwrap();
    assert_eq!(manifest["target"], artifact::TARGET);
    assert_eq!(manifest["wrapper_prefix_lines"], 3);
    assert_eq!(manifest["embedded_source"], false);
    assert!(manifest["source_map_sha256"].as_str().unwrap().len() == 64);
    assert!(!first.rust_module.contains("original TypeScript"));
}

#[test]
fn cross_target_maps_and_oversized_source_fail_before_export() {
    assert!(matches!(
        cook_script("a.js", SOURCE, None, "other-target"),
        Err(ScriptCookError::CrossTarget { .. })
    ));
    for map in [
        "null",
        "{}",
        r#"{"version":2,"sources":[],"names":[],"mappings":""}"#,
        r#"{"version":3,"sources":[],"names":[],"mappings":"","sections":[]}"#,
    ] {
        assert!(matches!(
            cook_script("a.js", SOURCE, Some(map), artifact::TARGET),
            Err(ScriptCookError::SourceMap)
        ));
    }
    assert!(cook_script("a.js", &" ".repeat(1_000_001), None, artifact::TARGET).is_err());
    assert!(
        cook_script(
            "a.js",
            SOURCE,
            Some(&" ".repeat(4 * 1024 * 1024 + 1)),
            artifact::TARGET
        )
        .is_err()
    );
    assert!(cook_script("bad\nname", SOURCE, None, artifact::TARGET).is_err());
}

#[test]
fn cook_never_executes_project_code_or_interprets_names_as_rust() {
    let artifact = cook_script(
        "quote\"; panic!(\"injected\"); //",
        "for (;;) {}",
        None,
        artifact::TARGET,
    )
    .unwrap();
    assert!(
        artifact
            .rust_module
            .contains(r#"source_name: "quote\"; panic!(\"injected\"); //".into()"#)
    );
    assert!(cook_script("a.js", "exports.default = {", None, artifact::TARGET).is_err());
}

#[test]
fn reviewed_mobile_destination_is_explicit_and_bound_to_fresh_source() {
    let native = cook_script("mobile.js", SOURCE, None, artifact::TARGET).unwrap();
    let native: serde_json::Value = serde_json::from_str(&native.symbol_manifest).unwrap();
    for target in ["aarch64-apple-ios", "aarch64-apple-ios-sim"] {
        if artifact::TARGET == "aarch64-apple-darwin"
            && artifact::BytecodeAbi::current().default_c_flags
        {
            let cooked = cook_script("mobile.js", SOURCE, None, target).unwrap();
            let m: serde_json::Value = serde_json::from_str(&cooked.symbol_manifest).unwrap();
            assert_eq!(m["target"], target);
            assert_eq!(m["producer_target"], artifact::TARGET);
            assert_eq!(m["bytecode_abi"], native["bytecode_abi"]);
            assert_eq!(m["build_id"], native["build_id"]);
            assert_eq!(m["bytecode_sha256"], native["bytecode_sha256"]);
            assert_eq!(m["hardening_sha256"], native["hardening_sha256"]);
            assert!(
                cooked
                    .rust_module
                    .contains(&format!("target: {target:?}.into()"))
            );
        } else {
            assert!(cook_script("mobile.js", SOURCE, None, target).is_err());
        }
    }
    // Architecture resemblance and source inspection do not admit an untested runtime.
    for target in [
        "aarch64-linux-android",
        "x86_64-apple-ios",
        "aarch64-apple-tvos",
        "aarch64-apple-ios-wrong",
    ] {
        assert!(matches!(
            cook_script("mobile.js", SOURCE, None, target),
            Err(ScriptCookError::CrossTarget { .. })
        ));
    }
}
