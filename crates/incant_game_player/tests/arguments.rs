//! Unknown/manual inputs refuse before scene, audio or graphics setup.
use std::process::Command;

#[test]
fn manual_and_unknown_options_refuse_before_startup_without_output() {
    let root = std::env::temp_dir().join(format!(
        "incant-player-arguments-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    for mode in [None, Some("--cpu-render-input"), Some("--gpu-offscreen")] {
        for option in ["--input", "--unknown"] {
            let dest = root.join("must-not-exist.pcm");
            let mut command = Command::new(env!("CARGO_BIN_EXE_incant_game_player"));
            command.current_dir(&root).args([
                "missing-scene",
                "missing-cache",
                "missing-clips",
                "missing-recording",
                "missing-actions",
                "1",
                "48000",
            ]);
            command.arg(&dest);
            if let Some(mode) = mode {
                command.args([
                    mode,
                    "missing-models",
                    "missing-textures",
                    "00000000000000000000000000000001",
                ]);
            }
            let result = command.args([option, "KeyD"]).output().unwrap();
            assert!(!result.status.success(), "{mode:?} {option}");
            assert!(result.stdout.is_empty(), "{mode:?} {option}");
            assert!(!dest.exists(), "{mode:?} {option}");
            let error = String::from_utf8(result.stderr).unwrap();
            let expected = if mode.is_some() {
                "unknown or duplicate spatial option"
            } else {
                "expected SCENE CACHE CLIPS RECORDING ACTIONS TICKS SAMPLE_RATE OUTPUT"
            };
            assert!(error.contains(expected), "{mode:?} {option}: {error}");
        }
    }
    std::fs::remove_dir(root).unwrap();
}
