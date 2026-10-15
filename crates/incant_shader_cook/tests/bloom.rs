//! Actual host emission/IR validation, without a GPU or native player.
use incant_shader_cook::{
    Backend, Stage,
    bloom::{self, Kind},
};
use sha2::{Digest, Sha256};
#[path = "../../incant_render/src/bloom/source.rs"]
mod source;

fn program(kind: Kind) -> String {
    if kind == Kind::Display {
        source::display()
    } else {
        include_str!("../../incant_render/src/bloom/filter.wgsl").to_owned()
    }
}

#[test]
fn four_programs_emit_deterministic_native_stages_and_valid_spirv() {
    for kind in [
        Kind::Prefilter,
        Kind::Downsample,
        Kind::Upsample,
        Kind::Display,
    ] {
        let source = program(kind);
        assert_eq!(
            <[u8; 32]>::from(Sha256::digest(&source)),
            kind.source_hash()
        );
        let bundle = bloom::cook(kind, &source).unwrap();
        assert_eq!(bundle, bloom::cook(kind, &source).unwrap());
        assert_eq!(bundle.artifacts.len(), 6);
        for a in &bundle.artifacts {
            assert_eq!(
                bundle
                    .artifacts
                    .iter()
                    .filter(|b| b.backend == a.backend && b.stage == a.stage)
                    .count(),
                1
            );
            if a.backend == Backend::Vulkan {
                let words = a
                    .bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|b| u32::from_le_bytes(*b));
                let module = naga::front::spv::Frontend::new(
                    words,
                    &naga::front::spv::Options {
                        adjust_coordinate_space: false,
                        ..Default::default()
                    },
                )
                .parse()
                .unwrap();
                naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .unwrap();
                assert_eq!(module.entry_points.len(), 1);
                assert_eq!(
                    module.entry_points[0].stage,
                    if a.stage == Stage::Vertex {
                        naga::ShaderStage::Vertex
                    } else {
                        naga::ShaderStage::Fragment
                    }
                );
                let mut bindings: Vec<_> = module
                    .global_variables
                    .iter()
                    .filter_map(|(_, g)| g.binding)
                    .map(|b| (b.group, b.binding))
                    .collect();
                bindings.sort();
                let expected = if a.stage == Stage::Vertex {
                    vec![]
                } else {
                    match kind {
                        Kind::Prefilter | Kind::Downsample => vec![(0, 0), (0, 2)],
                        Kind::Upsample => vec![(0, 0), (0, 1), (0, 2)],
                        Kind::Display => vec![(0, 0), (0, 1), (0, 2), (0, 3)],
                    }
                };
                assert_eq!(bindings, expected);
            } else {
                let text = std::str::from_utf8(&a.bytes).unwrap();
                assert!(text.contains(&a.entry));
                if a.stage == Stage::Fragment {
                    if a.backend == Backend::Metal {
                        assert!(text.contains("[[texture(0)]]") && text.contains("[[buffer(0)]]"));
                        assert_eq!(
                            text.contains("[[texture(1)]]"),
                            matches!(kind, Kind::Upsample | Kind::Display)
                        );
                        assert_eq!(text.contains("[[buffer(1)]]"), kind == Kind::Display);
                    } else {
                        assert!(text.contains("register(t0)") && text.contains("register(b0)"));
                        assert_eq!(
                            text.contains("register(t1)"),
                            matches!(kind, Kind::Upsample | Kind::Display)
                        );
                        assert_eq!(text.contains("register(b1)"), kind == Kind::Display);
                    }
                } else if a.backend == Backend::Dx12 {
                    assert!(text.contains(if kind == Kind::Display {
                        "register(b2)"
                    } else {
                        "register(b1)"
                    }));
                }
            }
        }
    }
}

#[test]
fn identity_and_bundle_membership_fail_before_publication() {
    let temporary = tempfile::tempdir().unwrap();
    let prefilter = bloom::cook(Kind::Prefilter, &program(Kind::Prefilter)).unwrap();
    let mismatched = temporary.path().join("same-source-wrong-program");
    assert!(bloom::write_bundle(Kind::Downsample, &prefilter, &mismatched).is_err());
    assert!(!mismatched.exists());
    for kind in [
        Kind::Prefilter,
        Kind::Downsample,
        Kind::Upsample,
        Kind::Display,
    ] {
        let source = program(kind);
        assert!(bloom::cook(kind, &format!("{source}\n")).is_err());
        let mut bundle = bloom::cook(kind, &source).unwrap();
        let directory = temporary.path().join(format!("{kind:?}"));
        bundle.artifacts[1] = bundle.artifacts[0].clone();
        assert!(bloom::write_bundle(kind, &bundle, &directory).is_err());
        assert!(!directory.exists());
    }
}

#[test]
fn all_four_families_publish_artifact_bound_audits() {
    let temporary = tempfile::tempdir().unwrap();
    for kind in [
        Kind::Prefilter,
        Kind::Downsample,
        Kind::Upsample,
        Kind::Display,
    ] {
        let bundle = bloom::cook(kind, &program(kind)).unwrap();
        let directory = temporary.path().join(format!("{kind:?}"));
        bloom::write_bundle(kind, &bundle, &directory).unwrap();
        let first = std::fs::read(directory.join("audit.json")).unwrap();
        bloom::write_bundle(kind, &bundle, &directory).unwrap();
        assert_eq!(first, std::fs::read(directory.join("audit.json")).unwrap());
        let receipt: serde_json::Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(receipt["contract"], kind.contract());
        let rows = receipt["artifacts"].as_array().unwrap();
        assert_eq!(rows.len(), 6);
        for row in rows {
            let bytes = std::fs::read(directory.join(row["file"].as_str().unwrap())).unwrap();
            assert_eq!(row["bytes"], bytes.len());
            let hash: String = Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(row["sha256"], hash);
        }
    }
}
