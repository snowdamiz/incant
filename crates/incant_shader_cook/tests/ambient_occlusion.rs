//! Five complete trusted SSAO families; host compilation only, no GPU claim.
use incant_shader_cook::{
    Backend, Stage,
    ambient_occlusion::{self as ao, Kind},
};
use sha2::{Digest, Sha256};
#[path = "../../incant_render/src/ambient_occlusion/source.rs"]
mod source;
const KINDS: [Kind; 5] = [
    Kind::Prepass,
    Kind::Forward,
    Kind::Sample,
    Kind::Horizontal,
    Kind::Vertical,
];
fn program(kind: Kind) -> String {
    match kind {
        Kind::Prepass => source::material(true).unwrap(),
        Kind::Forward => source::material(false).unwrap(),
        Kind::Sample => include_str!("../../incant_render/src/ambient_occlusion/ao.wgsl").into(),
        Kind::Horizontal | Kind::Vertical => {
            include_str!("../../incant_render/src/ambient_occlusion/filter.wgsl").into()
        }
    }
}

#[test]
fn all_five_programs_emit_deterministic_native_stages_with_exact_resources() {
    let mut total = 0;
    for kind in KINDS {
        let source = program(kind);
        assert_eq!(
            <[u8; 32]>::from(Sha256::digest(&source)),
            kind.source_hash()
        );
        let bundle = ao::cook(kind, &source).unwrap();
        assert_eq!(bundle, ao::cook(kind, &source).unwrap());
        assert_eq!(bundle.artifacts.len(), 6);
        total += bundle.artifacts.len();
        for a in &bundle.artifacts {
            assert_eq!(
                bundle
                    .artifacts
                    .iter()
                    .filter(|b| a.backend == b.backend && a.stage == b.stage)
                    .count(),
                1
            );
            if a.backend == Backend::Vulkan {
                let module = naga::front::spv::parse_u8_slice(
                    &a.bytes,
                    &naga::front::spv::Options {
                        adjust_coordinate_space: false,
                        strict_capabilities: true,
                        block_ctx_dump_prefix: None,
                    },
                )
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
                    if matches!(kind, Kind::Prepass | Kind::Forward) {
                        vec![(0, 0)]
                    } else {
                        vec![]
                    }
                } else {
                    match kind {
                        Kind::Prepass => (0..=2).map(|n| (1, n)).collect(),
                        Kind::Forward => [(0, 0), (0, 1)]
                            .into_iter()
                            .chain((0..=10).map(|n| (1, n)))
                            .chain((0..=3).map(|n| (2, n)))
                            .chain((0..=6).map(|n| (3, n)))
                            .collect(),
                        Kind::Sample => (0..=2).map(|n| (0, n)).collect(),
                        Kind::Horizontal | Kind::Vertical => (0..=3).map(|n| (0, n)).collect(),
                    }
                };
                assert_eq!(bindings, expected, "{kind:?} {:?}", a.stage);
                continue;
            }
            let text = std::str::from_utf8(&a.bytes).unwrap();
            assert!(text.contains(&a.entry));
            if a.backend == Backend::Dx12 {
                let special: Vec<_> = text
                    .lines()
                    .filter(|l| l.starts_with("ConstantBuffer<"))
                    .collect();
                let register = match kind {
                    Kind::Prepass => 2,
                    Kind::Forward => 5,
                    _ => 1,
                };
                assert_eq!(
                    special,
                    [format!(
                        "ConstantBuffer<NagaConstants> _NagaConstants: register(b{register});"
                    )]
                );
            }
            let tokens: &[&str] = match (a.backend, a.stage, kind) {
                (Backend::Metal, Stage::Vertex, Kind::Prepass | Kind::Forward) => &[
                    "[[buffer(0)]]",
                    "[[buffer(1)]]",
                    "[[buffer(15)]]",
                    "[[buffer(14)]]",
                    "buffer_size15 / 48",
                    "buffer_size14 / 112",
                ],
                (Backend::Metal, Stage::Fragment, Kind::Prepass) => &[
                    "[[buffer(1)]]",
                    "[[texture(0)]]",
                    "[[sampler(0)]]",
                    "[[front_facing]]",
                ],
                (Backend::Metal, Stage::Fragment, Kind::Forward) => &[
                    "[[texture(0)]]",
                    "[[texture(9)]]",
                    "[[buffer(7)]]",
                    "[[sampler(6)]]",
                    "_mslBufferSizes",
                ],
                (Backend::Metal, Stage::Fragment, Kind::Sample) => {
                    &["[[buffer(0)]]", "[[texture(0)]]", "[[texture(1)]]"]
                }
                (Backend::Metal, Stage::Fragment, Kind::Horizontal | Kind::Vertical) => &[
                    "[[buffer(0)]]",
                    "[[texture(0)]]",
                    "[[texture(1)]]",
                    "[[texture(2)]]",
                ],
                (Backend::Dx12, Stage::Fragment, Kind::Prepass) => {
                    &["register(b1)", "register(t0)", "register(t5, space0)"]
                }
                (Backend::Dx12, Stage::Fragment, Kind::Forward) => &[
                    "register(b0)",
                    "register(b1)",
                    "register(b2)",
                    "register(b3)",
                    "register(t0)",
                    "register(t1)",
                    "register(t6, space0)",
                    "register(t10, space0)",
                    "register(t15, space0)",
                    "register(s2048, space0)",
                ],
                (Backend::Dx12, Stage::Fragment, Kind::Sample) => {
                    &["register(b0)", "register(t0)", "register(t1)"]
                }
                (Backend::Dx12, Stage::Fragment, Kind::Horizontal | Kind::Vertical) => &[
                    "register(b0)",
                    "register(t0)",
                    "register(t1)",
                    "register(t2)",
                ],
                _ => &[],
            };
            for token in tokens {
                assert!(
                    text.contains(token),
                    "{kind:?} {:?}: missing {token}\n{text}",
                    a.stage
                );
            }
        }
    }
    assert_eq!(total, 30);
}

#[test]
fn wrong_kind_source_membership_and_hash_refuse_before_writing() {
    let temp = tempfile::tempdir().unwrap();
    let horizontal = ao::cook(Kind::Horizontal, &program(Kind::Horizontal)).unwrap();
    let path = temp.path().join("same-source-wrong-kind");
    assert!(ao::write_bundle(Kind::Vertical, &horizontal, &path).is_err());
    assert!(!path.exists());
    for kind in KINDS {
        let source = program(kind);
        assert!(ao::cook(kind, &format!("{source}\n")).is_err());
        for mutation in 0..4 {
            let mut bundle = ao::cook(kind, &source).unwrap();
            match mutation {
                0 => {
                    bundle.artifacts.pop();
                }
                1 => bundle.artifacts[1] = bundle.artifacts[0].clone(),
                2 => bundle.source_hash[0] ^= 1,
                _ => bundle.producer_hash = [0; 32],
            }
            let path = temp.path().join(format!("{kind:?}-{mutation}"));
            assert!(ao::write_bundle(kind, &bundle, &path).is_err());
            assert!(!path.exists());
        }
    }
}

#[test]
fn all_five_audits_bind_each_generated_stage_and_manifest() {
    let temp = tempfile::tempdir().unwrap();
    for kind in KINDS {
        let bundle = ao::cook(kind, &program(kind)).unwrap();
        let path = temp.path().join(format!("{kind:?}"));
        ao::write_bundle(kind, &bundle, &path).unwrap();
        let raw = std::fs::read(path.join("audit.json")).unwrap();
        ao::write_bundle(kind, &bundle, &path).unwrap();
        assert_eq!(raw, std::fs::read(path.join("audit.json")).unwrap());
        let receipt: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(receipt["contract"], kind.contract());
        assert_eq!(
            receipt["source_sha256"],
            format!("{:x}", Sha256::digest(program(kind)))
        );
        let manifest = std::fs::read(path.join("manifest.rs")).unwrap();
        assert_eq!(
            receipt["manifest_sha256"],
            format!("{:x}", Sha256::digest(manifest))
        );
        let rows = receipt["artifacts"].as_array().unwrap();
        assert_eq!(rows.len(), 6);
        for row in rows {
            let bytes = std::fs::read(path.join(row["file"].as_str().unwrap())).unwrap();
            assert_eq!(row["bytes"], bytes.len());
            assert_eq!(row["sha256"], format!("{:x}", Sha256::digest(bytes)));
        }
    }
}
