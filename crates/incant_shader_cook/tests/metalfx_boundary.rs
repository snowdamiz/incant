use incant_shader_cook::{
    Backend, Stage,
    metalfx_boundary::{Kind, cook, write_bundle},
};
#[path = "../../incant_render/src/metalfx/source.rs"]
mod source;

#[test]
fn both_fixed_programs_emit_deterministic_native_stages_with_exact_fragment_maps() {
    for kind in [Kind::Pack, Kind::Normalize] {
        let source = source::source(kind == Kind::Normalize);
        let bundle = cook(kind, &source).unwrap();
        assert_eq!(bundle, cook(kind, &source).unwrap());
        assert_eq!(bundle.artifacts.len(), 6);
        for artifact in &bundle.artifacts {
            if artifact.backend == Backend::Vulkan {
                let module = naga::front::spv::parse_u8_slice(
                    &artifact.bytes,
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
                continue;
            }
            let text = std::str::from_utf8(&artifact.bytes).unwrap();
            assert!(!artifact.entry.is_empty());
            if artifact.stage == Stage::Fragment {
                let tokens: &[&str] = if artifact.backend == Backend::Metal {
                    &[
                        "[[buffer(0)]]",
                        "[[texture(0)]]",
                        "[[texture(1)]]",
                        "[[texture(2)]]",
                        "get_width",
                        ".read(",
                    ]
                } else {
                    &[
                        "register(b0)",
                        "register(t0)",
                        "register(t1)",
                        "register(t2)",
                        ".Load(",
                    ]
                };
                for token in tokens {
                    assert!(
                        text.contains(token),
                        "{kind:?} {:?} missing {token}\n{text}",
                        artifact.backend
                    );
                }
                for location in 0..if kind == Kind::Pack { 4 } else { 1 } {
                    let token = if artifact.backend == Backend::Metal {
                        format!("[[color({location})]]")
                    } else {
                        format!("SV_Target{location}")
                    };
                    assert!(
                        text.contains(&token),
                        "{kind:?} {:?} missing {token}\n{text}",
                        artifact.backend
                    );
                }
            }
        }
    }
}

#[test]
fn changed_trusted_equations_and_cross_kind_source_refuse() {
    for kind in [Kind::Pack, Kind::Normalize] {
        let source = source::source(kind == Kind::Normalize);
        assert!(cook(kind, &format!("{source}\n// changed trusted bytes")).is_err());
        assert!(cook(kind, &source.replace("65504.0", "32752.0")).is_err());
        let other = if kind == Kind::Pack {
            Kind::Normalize
        } else {
            Kind::Pack
        };
        assert!(cook(other, &source).is_err());
    }
}

#[test]
fn sealed_kind_exact_source_producer_and_complete_stages_refuse_before_writes() {
    for kind in [Kind::Pack, Kind::Normalize] {
        let bundle = cook(kind, &source::source(kind == Kind::Normalize)).unwrap();
        let root = tempfile::tempdir().unwrap();
        let mut missing = bundle.clone();
        missing.artifacts.pop();
        let mut duplicate = bundle.clone();
        duplicate.artifacts[0] = duplicate.artifacts[1].clone();
        let mut source = bundle.clone();
        source.source_hash[0] ^= 1;
        let mut producer = bundle.clone();
        producer.producer_hash[0] ^= 1;
        let mut absent = bundle.clone();
        absent.producer_hash = [0; 32];
        for (index, invalid) in [missing, duplicate, source, producer, absent]
            .iter()
            .enumerate()
        {
            let path = root.path().join(format!("invalid{index}"));
            assert!(write_bundle(kind, invalid, &path).is_err());
            assert!(!path.exists());
        }
        let other = if kind == Kind::Pack {
            Kind::Normalize
        } else {
            Kind::Pack
        };
        assert!(write_bundle(other, &bundle, &root.path().join("wrong-kind")).is_err());
        assert!(!root.path().join("wrong-kind").exists());
        let valid = root.path().join("valid");
        write_bundle(kind, &bundle, &valid).unwrap();
        let receipt: serde_json::Value =
            serde_json::from_slice(&std::fs::read(valid.join("audit.json")).unwrap()).unwrap();
        assert_eq!(receipt["contract"], kind.contract());
        assert_eq!(receipt["artifacts"].as_array().unwrap().len(), 6);
        for row in receipt["artifacts"].as_array().unwrap() {
            use sha2::{Digest, Sha256};
            let data = std::fs::read(valid.join(row["file"].as_str().unwrap())).unwrap();
            assert_eq!(row["bytes"].as_u64().unwrap(), data.len() as u64);
            assert_eq!(row["sha256"], format!("{:x}", Sha256::digest(data)));
        }
    }
}
