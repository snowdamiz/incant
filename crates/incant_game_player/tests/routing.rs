#![cfg(all(
    feature = "native-window",
    any(target_os = "macos", target_os = "windows", target_os = "linux")
))]

use std::process::Command;

// The file is valid JSON but has a deliberately unsupported observation format.
// Every other input is absent: neither invocation may reach scene/device setup.
const PLAN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/unsupported-observation.json"
);

fn refusal(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_incant_game_player"))
        .args(args)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn window_observation_refuses_before_startup_but_headless_keeps_observation_route() {
    let window = refusal(&[
        "--window",
        "missing-scene",
        "missing-cache",
        "missing-clips",
        "missing-actions",
        "missing-models",
        "missing-textures",
        "00000000000000000000000000000001",
        "--observe",
        PLAN,
    ]);
    assert!(window.contains("expected --window SCENE"), "{window}");
    let headless = refusal(&[
        "missing-scene",
        "missing-cache",
        "missing-clips",
        "missing-recording",
        "missing-actions",
        "1",
        "48000",
        "unused-output.pcm",
        "--observe",
        PLAN,
    ]);
    assert!(
        headless.contains("unsupported native observation format/version"),
        "{headless}"
    );
}
