use incant_shader_cook::{
    Backend, Stage,
    temporal::{Kind, cook, write_bundle},
};
fn source(kind: Kind) -> String {
    match kind {
        Kind::MotionOwner => include_str!("../../incant_render/src/motion/owner.wgsl").into(),
        Kind::Motion | Kind::Depth => {
            include_str!("../../incant_render/src/motion/shader.wgsl").into()
        }
        Kind::Resolve => include_str!("../../incant_render/src/temporal/resolve.wgsl").into(),
        Kind::Copy => include_str!("../../incant_render/src/temporal/copy.wgsl").into(),
    }
}
#[test]
fn fixed_programs_preserve_interfaces_robust_accesses_and_native_binding_maps() {
    for kind in [
        Kind::Motion,
        Kind::MotionOwner,
        Kind::Depth,
        Kind::Resolve,
        Kind::Copy,
    ] {
        let b = cook(kind, &source(kind)).unwrap();
        assert_eq!(b.artifacts.len(), 6);
        assert_eq!(b, cook(kind, &source(kind)).unwrap());
        for a in &b.artifacts {
            if a.backend == Backend::Vulkan {
                let m = naga::front::spv::parse_u8_slice(
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
                .validate(&m)
                .unwrap();
                assert_eq!(m.entry_points.len(), 1);
                continue;
            }
            let text = std::str::from_utf8(&a.bytes).unwrap();
            let required: &[&str] = match (kind, a.backend, a.stage) {
                (Kind::Motion | Kind::Depth | Kind::MotionOwner, Backend::Metal, Stage::Vertex) => {
                    &[
                        "[[buffer(0)]]",
                        "[[buffer(1)]]",
                        "[[buffer(15)]]",
                        "[[buffer(14)]]",
                        "[[buffer(13)]]",
                        "[[buffer(12)]]",
                        "buffer_size15 / 48",
                        "buffer_size14 / 48",
                        "buffer_size13 / 112",
                        "buffer_size12 / 80",
                    ]
                }
                (Kind::Motion | Kind::MotionOwner, Backend::Metal, Stage::Fragment) => &[
                    "[[buffer(0)]]",
                    "[[buffer(1)]]",
                    "[[texture(0)]]",
                    "[[sampler(0)]]",
                ],
                (Kind::Motion | Kind::Depth | Kind::MotionOwner, Backend::Dx12, Stage::Vertex) => {
                    &["register(b0)", "SV_Position"]
                }
                (Kind::Motion | Kind::MotionOwner, Backend::Dx12, Stage::Fragment) => &[
                    "register(b0)",
                    "register(b1)",
                    "register(t0)",
                    "register(t5, space0)",
                ],
                (Kind::Depth, Backend::Metal, Stage::Fragment) => {
                    &["[[buffer(1)]]", "[[texture(0)]]", "[[sampler(0)]]"]
                }
                (Kind::Depth, Backend::Dx12, Stage::Fragment) => {
                    &["register(b1)", "register(t0)", "register(t5, space0)"]
                }
                (_, Backend::Metal, Stage::Fragment) => &["[[texture(0)]]", "get_width"],
                (_, Backend::Dx12, Stage::Fragment) => &["register(t0)", ".Load("],
                _ => &[],
            };
            for token in required {
                assert!(
                    text.contains(token),
                    "{kind:?} {:?} {:?} missing {token}\n{text}",
                    a.backend,
                    a.stage
                );
            }
        }
        if kind == Kind::MotionOwner {
            for a in &b.artifacts {
                if a.backend == Backend::Vulkan {
                    continue;
                }
                let text = std::str::from_utf8(&a.bytes).unwrap();
                if a.stage == Stage::Fragment {
                    assert!(text.contains(if a.backend == Backend::Metal {
                        "[[color(1)]]"
                    } else {
                        "SV_Target1"
                    }));
                    assert!(text.contains("uint"));
                }
            }
        }
        if let Some(root) = std::env::var_os("INCANT_TEMPORAL_COOK_EVIDENCE") {
            write_bundle(
                kind,
                &b,
                &std::path::PathBuf::from(root).join(format!("{kind:?}")),
            )
            .unwrap();
        }
    }
}
#[test]
fn changed_math_interface_and_program_identity_are_not_trusted() {
    let s = source(Kind::Motion);
    for changed in [
        s.replace("@location(2)", "@location(12)"),
        s.replace("previous:mat4x4f", "previous:mat3x3f"),
        s.replace("@binding(2)", "@binding(10)"),
        s.replace("old.z,1", "old.z,0"),
        format!("{s}\n"),
    ] {
        assert!(cook(Kind::Motion, &changed).is_err());
    }
    for kind in [Kind::Resolve, Kind::Copy] {
        assert!(cook(kind, &s).is_err());
    }
}

#[test]
fn coverage_resolve_rejects_previous_program_and_modified_coverage_math() {
    let current = source(Kind::Resolve);
    let previous = include_str!("fixtures/temporal-resolve-single-depth.wgsl");
    assert_ne!(current, previous);
    assert!(cook(Kind::Resolve, previous).is_err());
    let changed = current.replace("color.a>1.0", "color.a!=1.0");
    assert_ne!(current, changed);
    assert!(cook(Kind::Resolve, &changed).is_err());
    let bundle = cook(Kind::Resolve, &current).unwrap();
    assert_eq!(bundle.artifacts.len(), 6);
}
