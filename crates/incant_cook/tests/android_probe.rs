#![cfg(feature = "script")]

use incant_script::artifact;

#[test]
fn android_probe_child() {
    let Ok(expected) = std::env::var("INCANT_TEST_ANDROID_PROBE_ERROR") else {
        return;
    };
    let result = incant_cook::embed_script("unused-default.js");
    if expected.is_empty() {
        result.unwrap();
    } else {
        assert_eq!(result.unwrap_err().to_string(), expected);
    }
}

#[test]
fn candidate_fixture_is_explicit_and_never_accepts_an_alternative_source_or_map() {
    let root = tempfile::tempdir().unwrap();
    let fixture = include_str!("../../incant_script/tests/fixtures/mobile_compat.js");
    let accepted = artifact::TARGET == "aarch64-apple-darwin"
        && cfg!(feature = "probe-android-bytecode")
        && artifact::compatibility::admits(
            artifact::TARGET,
            "aarch64-linux-android",
            artifact::BytecodeAbi::current(),
        );
    let admission = "embedded script producer/destination bytecode ABI is not admitted";
    let fixture_error = "Android bytecode probe requires the exact trusted numeric fixture";
    for (index, (source, name, map)) in [
        (fixture.to_owned(), "mobile-compat.js", false),
        (format!("{fixture}\n"), "mobile-compat.js", false),
        (fixture.to_owned(), "alternative.js", false),
        (fixture.to_owned(), "mobile-compat.js", true),
    ]
    .into_iter()
    .enumerate()
    {
        let path = root.path().join(format!("source-{index}.js"));
        std::fs::write(&path, source).unwrap();
        let output = root.path().join(format!("out-{index}"));
        std::fs::create_dir(&output).unwrap();
        let expected = if !accepted {
            admission
        } else if index == 0 {
            ""
        } else {
            fixture_error
        };
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "android_probe_child", "--nocapture"])
            .env("INCANT_TEST_ANDROID_PROBE_ERROR", expected)
            .env("INCANT_NATIVE_SCRIPT_SOURCE", path)
            .env("INCANT_NATIVE_SCRIPT_NAME", name)
            .env_remove("INCANT_NATIVE_SCRIPT_MAP")
            .env("HOST", artifact::TARGET)
            .env("TARGET", "aarch64-linux-android")
            .env("OUT_DIR", &output);
        if map {
            let map = root.path().join("map.json");
            std::fs::write(&map, "{}").unwrap();
            command.env("INCANT_NATIVE_SCRIPT_MAP", map);
        }
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        if expected.is_empty() {
            let metadata: serde_json::Value =
                serde_json::from_slice(&std::fs::read(output.join("script-symbols.json")).unwrap())
                    .unwrap();
            assert_eq!(metadata["target"], "aarch64-linux-android");
            assert_eq!(metadata["producer_target"], artifact::TARGET);
            assert_eq!(metadata["source_name"], "mobile-compat.js");
            assert_eq!(metadata["bytecode_abi"]["version"], 28);
            assert!(output.join("behavior.rs").is_file());
        } else {
            assert_eq!(std::fs::read_dir(output).unwrap().count(), 0);
        }
    }
}
