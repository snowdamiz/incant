use super::Options;
fn args(tail: &[&str]) -> Vec<String> {
    [
        "scene",
        "cache",
        "clips",
        "actions",
        "models",
        "textures",
        "0123456789abcdef0123456789abcdef",
    ]
    .iter()
    .chain(tail)
    .map(|s| (*s).to_owned())
    .collect()
}
#[test]
fn compilation_capture_needs_explicit_feature_and_complete_host_bounds() {
    assert!(Options::parse(&args(&[])).is_ok());
    let valid = Options::parse(&args(&[
        "--stop-after",
        "6",
        "--compilation-output",
        "trusted.json",
        "--compilation-capacity",
        "8",
    ]));
    assert_eq!(
        valid.is_ok(),
        cfg!(all(feature = "native-compilation", target_os = "macos"))
    );
    for tail in [
        vec!["--compilation-output", "x"],
        vec!["--stop-after", "6", "--compilation-output", "x"],
        vec!["--stop-after", "6", "--compilation-capacity", "8"],
        vec![
            "--stop-after",
            "6",
            "--compilation-output",
            "",
            "--compilation-capacity",
            "8",
        ],
        vec![
            "--stop-after",
            "6",
            "--compilation-output",
            "a\nb",
            "--compilation-capacity",
            "8",
        ],
        vec![
            "--stop-after",
            "6",
            "--compilation-output",
            "x",
            "--compilation-capacity",
            "0",
        ],
        vec![
            "--stop-after",
            "6",
            "--compilation-output",
            "x",
            "--compilation-capacity",
            "4097",
        ],
        vec![
            "--stop-after",
            "6",
            "--compilation-output",
            "x",
            "--compilation-capacity",
            "8",
            "--lifecycle-probe",
        ],
        vec![
            "--stop-after",
            "6",
            "--compilation-output",
            "x",
            "--compilation-output",
            "y",
            "--compilation-capacity",
            "8",
        ],
    ] {
        assert!(Options::parse(&args(&tail)).is_err(), "{tail:?}");
    }
}
