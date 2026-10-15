//! Real host cooking and disk evidence; no driver or native renderer is started.
use incant_shader_cook::{compute, cook_output, geometry, write_bundle};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn check(directory: &Path, source: &str, count: usize) -> Vec<u8> {
    let raw = std::fs::read(directory.join("audit.json")).unwrap();
    let receipt: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(receipt["format"], "incant-fixed-shader-build");
    assert_eq!(receipt["version"], 1);
    assert_eq!(receipt["source_sha256"], hex(&Sha256::digest(source)));
    assert_eq!(receipt["producer_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(
        receipt["manifest_sha256"],
        hex(&Sha256::digest(
            std::fs::read(directory.join("manifest.rs")).unwrap()
        ))
    );
    let artifacts = receipt["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), count);
    let mut identities = std::collections::BTreeSet::new();
    for artifact in artifacts {
        let backend = artifact["backend"].as_str().unwrap();
        let stage = artifact["stage"].as_str().unwrap();
        assert!(identities.insert((backend, stage)));
        let file = artifact["file"].as_str().unwrap();
        assert_eq!(file, format!("{backend}-{stage}.shader"));
        let bytes = std::fs::read(directory.join(file)).unwrap();
        assert_eq!(artifact["sha256"], hex(&Sha256::digest(&bytes)));
        assert_eq!(artifact["bytes"], bytes.len());
    }
    raw
}

#[test]
fn every_selected_real_family_writes_deterministic_artifact_bound_evidence() {
    let temporary = tempfile::tempdir().unwrap();
    let source = [
        include_str!("../../incant_render/src/output.wgsl"),
        include_str!("../../incant_render/src/tone_map.wgsl"),
    ]
    .join("\n");
    let output = temporary.path().join("output");
    let bundle = cook_output(&source).unwrap();
    write_bundle(&bundle, &output).unwrap();
    let initial = check(&output, &source, 6);
    write_bundle(&bundle, &output).unwrap();
    assert_eq!(initial, check(&output, &source, 6));

    for (kind, source) in [
        (
            geometry::Kind::Diagnostic,
            include_str!("../../incant_render/src/fixture.wgsl").to_owned(),
        ),
        (
            geometry::Kind::Shadow,
            include_str!("../../incant_render/src/shadows/caster.wgsl").to_owned(),
        ),
        (
            geometry::Kind::Sprite,
            include_str!("../../incant_render/src/sprites/sprite.wgsl").to_owned(),
        ),
        (
            geometry::Kind::Ui,
            include_str!("../../incant_render/src/ui/ui.wgsl").to_owned(),
        ),
        (
            geometry::Kind::Text,
            include_str!("../../incant_render/src/text/text.wgsl").to_owned(),
        ),
        (
            geometry::Kind::Material,
            [
                include_str!("../../incant_render/src/shadows/shade.wgsl"),
                include_str!("../../incant_render/src/lighting/shade.wgsl"),
                include_str!("../../incant_render/src/model_material.wgsl"),
            ]
            .join("\n"),
        ),
    ] {
        let directory = temporary.path().join(format!("{kind:?}"));
        let bundle = geometry::cook(kind, &source).unwrap();
        geometry::write_bundle(kind, &bundle, &directory).unwrap();
        let initial = check(&directory, &source, 6);
        geometry::write_bundle(kind, &bundle, &directory).unwrap();
        assert_eq!(initial, check(&directory, &source, 6));
    }
    let source = include_str!("../../incant_render/src/lighting/grid.wgsl");
    let directory = temporary.path().join("cluster");
    let bundle = compute::cook(source).unwrap();
    compute::write_bundle(&bundle, &directory).unwrap();
    let initial = check(&directory, source, 3);
    compute::write_bundle(&bundle, &directory).unwrap();
    assert_eq!(initial, check(&directory, source, 3));
}

#[test]
fn invalid_bundle_cannot_publish_a_success_receipt() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path().join("not-published");
    let mut bundle =
        compute::cook(include_str!("../../incant_render/src/lighting/grid.wgsl")).unwrap();
    bundle.artifacts.pop();
    assert!(compute::write_bundle(&bundle, &directory).is_err());
    assert!(!directory.exists());
}
