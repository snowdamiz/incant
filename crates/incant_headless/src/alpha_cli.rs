//! CLI grammar only; shared import tests exercise actual checked texture,
//! command, physics-query and save/journal behavior without any native window.
use super::{Args, Cli};
use clap::Parser;

#[test]
fn static_alpha_command_requires_explicit_selection_and_bounded_threshold() {
    let args = Args::try_parse_from([
        "incant",
        "collider-from-alpha",
        "project.incant",
        "scene",
        "entity",
    ])
    .unwrap();
    let Cli::ColliderFromAlpha {
        project,
        scene_id,
        entity_id,
        threshold,
        cache,
    } = args.command
    else {
        panic!("alpha command")
    };
    assert_eq!(project.to_str(), Some("project.incant"));
    assert_eq!(
        (scene_id.as_str(), entity_id.as_str(), threshold),
        ("scene", "entity", 128)
    );
    assert!(cache.is_none());
    for threshold in ["0", "256", "-1"] {
        assert!(
            Args::try_parse_from([
                "incant",
                "collider-from-alpha",
                "project.incant",
                "scene",
                "entity",
                "--threshold",
                threshold
            ])
            .is_err()
        );
    }
    assert!(
        Args::try_parse_from(["incant", "collider-from-alpha", "project.incant", "scene"]).is_err()
    );
}
