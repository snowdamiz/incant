use incant_shader_cook::{
    Backend, Stage,
    geometry::{Kind, cook, write_bundle},
};

fn source() -> String {
    [
        include_str!("../../incant_render/src/shadows/shade.wgsl"),
        include_str!("../../incant_render/src/lighting/shade.wgsl"),
        include_str!("../../incant_render/src/model_material.wgsl"),
    ]
    .join("\n")
}

#[test]
fn material_preserves_paired_interfaces_and_explicit_storage_lengths() {
    let bundle = cook(Kind::Material, &source()).unwrap();
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
        let tokens: &[&str] = match (artifact.backend, artifact.stage) {
            (Backend::Metal, Stage::Vertex) => &[
                "[[buffer(0)]]",
                "[[buffer(1)]]",
                "[[buffer(15)]]",
                "[[buffer(14)]]",
                "buffer_size15 / 48",
                "buffer_size14 / 112",
            ],
            (Backend::Metal, Stage::Fragment) => &[
                "[[buffer(7)]]",
                "[[texture(8)]]",
                "[[sampler(6)]]",
                "_mslBufferSizes",
                "[[front_facing]]",
            ],
            (Backend::Dx12, Stage::Vertex) => &["register(b0)", "SV_Position"],
            (Backend::Dx12, Stage::Fragment) => &[
                "register(b1)",
                "register(b2)",
                "register(b3)",
                "register(t10)",
                "register(t11)",
                "register(t12)",
                "register(t13)",
                "register(t14, space0)",
                "register(s2048, space0)",
                "GetDimensions",
            ],
            _ => unreachable!(),
        };
        for token in tokens {
            assert!(text.contains(token), "missing {token}\n{text}");
        }
        if artifact.backend == Backend::Metal && artifact.stage == Stage::Fragment {
            let lengths = text
                .split("struct _mslBufferSizes {")
                .nth(1)
                .unwrap()
                .split("};")
                .next()
                .unwrap();
            assert_eq!(
                lengths
                    .lines()
                    .filter(|line| line.trim().starts_with("uint "))
                    .count(),
                3
            );
            assert!(!text.contains("[[buffer(8)]]"));
        }
    }
    if let Some(path) = std::env::var_os("INCANT_MATERIAL_COOK_EVIDENCE") {
        write_bundle(Kind::Material, &bundle, std::path::Path::new(&path)).unwrap();
    }
}

#[test]
fn material_refuses_changed_binding_memory_and_io_contracts() {
    let original = source();
    for changed in [
        original.replace("@group(3) @binding(1)", "@group(3) @binding(7)"),
        original.replace(
            "var<storage,read> punctual_lights",
            "var<storage,read_write> punctual_lights",
        ),
        original.replace(
            "cascades:array<ShadowCascade,16>",
            "cascades:array<ShadowCascade,32>",
        ),
        original.replace(
            "position_range:vec4f, direction_outer",
            "@size(32) position_range:vec4f, direction_outer",
        ),
        original.replace(
            "view_projection: mat4x4f, eye",
            "@size(80) view_projection: mat4x4f, eye",
        ),
        original.replace("@location(10)", "@location(11)"),
        original.replace("@builtin(front_facing)", "@location(7)"),
    ] {
        assert_ne!(original, changed);
        assert!(
            cook(Kind::Material, &changed).is_err(),
            "accepted {changed}"
        );
    }
}
