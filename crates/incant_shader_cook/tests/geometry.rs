use incant_shader_cook::{
    Backend, Stage,
    geometry::{Kind, cook},
};
fn source(kind: Kind) -> &'static str {
    match kind {
        Kind::Diagnostic => include_str!("../../incant_render/src/fixture.wgsl"),
        Kind::Shadow => include_str!("../../incant_render/src/shadows/caster.wgsl"),
        Kind::Text => include_str!("../../incant_render/src/text/text.wgsl"),
        Kind::Ui => include_str!("../../incant_render/src/ui/ui.wgsl"),
        Kind::Sprite => include_str!("../../incant_render/src/sprites/sprite.wgsl"),
        Kind::Material => concat!(
            include_str!("../../incant_render/src/shadows/shade.wgsl"),
            "\n",
            include_str!("../../incant_render/src/lighting/shade.wgsl"),
            "\n",
            include_str!("../../incant_render/src/model_material.wgsl")
        ),
    }
}
#[test]
fn fixed_geometry_artifacts_are_deterministic_and_spirv_interfaces_validate() {
    for kind in [
        Kind::Diagnostic,
        Kind::Shadow,
        Kind::Sprite,
        Kind::Ui,
        Kind::Text,
        Kind::Material,
    ] {
        let a = cook(kind, source(kind)).unwrap();
        assert_eq!(a, cook(kind, source(kind)).unwrap());
        assert_eq!(a.artifacts.len(), 6);
        for artifact in a.artifacts.iter().filter(|a| a.backend == Backend::Vulkan) {
            let words = artifact
                .bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|w| u32::from_le_bytes(*w));
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
            assert_eq!(module.entry_points[0].name, artifact.entry);
            assert_eq!(
                module.entry_points[0].stage,
                if artifact.stage == Stage::Vertex {
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
            assert_eq!(
                bindings,
                match (kind, artifact.stage) {
                    (Kind::Diagnostic, _) => vec![],
                    (Kind::Shadow, Stage::Vertex) => vec![(0, 0)],
                    (Kind::Shadow, Stage::Fragment) => vec![(1, 0), (1, 1), (1, 2)],
                    (Kind::Sprite | Kind::Ui, Stage::Vertex) => vec![(0, 0)],
                    (Kind::Sprite, Stage::Fragment) => vec![(1, 0), (1, 1)],
                    (Kind::Ui, Stage::Fragment) => vec![(0, 0), (1, 0), (1, 1)],
                    (Kind::Text, Stage::Vertex) => vec![(0, 0), (1, 0)],
                    (Kind::Text, Stage::Fragment) => vec![(0, 0), (2, 0), (2, 1)],
                    (Kind::Material, Stage::Vertex) => vec![(0, 0)],
                    (Kind::Material, Stage::Fragment) => std::iter::once((0, 0))
                        .chain((0..=10).map(|b| (1, b)))
                        .chain((0..=3).map(|b| (2, b)))
                        .chain((0..=6).map(|b| (3, b)))
                        .collect(),
                }
            );
        }
    }
}
#[test]
fn geometry_contract_rejects_changed_bindings_io_storage_and_stage_usage() {
    for kind in [
        Kind::Diagnostic,
        Kind::Shadow,
        Kind::Sprite,
        Kind::Ui,
        Kind::Text,
        Kind::Material,
    ] {
        for changed in [
            source(kind).replace("@location(0)", "@location(11)"),
            source(kind).replace("fn vertex(", "fn other("),
            format!("{}\nvar<workgroup> scratch:u32;", source(kind)),
            format!("{}\noverride option:u32=1u;", source(kind)),
            format!(
                "{}\n@group(3) @binding(0) var<storage,read> extra:array<f32>;",
                source(kind)
            ),
        ] {
            assert!(
                cook(kind, &changed).is_err(),
                "accepted changed {kind:?} contract"
            );
        }
    }
    for changed in [
        source(Kind::Shadow).replace("@binding(2)", "@binding(3)"),
        source(Kind::Shadow).replace("flags:vec4u", "flags:vec4u,extra:vec4f"),
        source(Kind::Shadow).replace("out.uv=uv", "out.uv=uv*material.base.xy"),
        source(Kind::Shadow).replace(
            "var base_map:texture_2d<f32>",
            "var base_map:texture_cube<f32>",
        ),
    ] {
        assert!(cook(Kind::Shadow, &changed).is_err());
    }
    assert!(cook(Kind::Diagnostic, source(Kind::Shadow)).is_err());
    assert!(cook(Kind::Shadow, source(Kind::Diagnostic)).is_err());
}
#[test]
fn metal_uses_bounded_vertex_and_instance_pulling_with_exact_host_strides() {
    for kind in [Kind::Diagnostic, Kind::Shadow] {
        let bundle = cook(kind, source(kind)).unwrap();
        let vertex = bundle
            .artifacts
            .iter()
            .find(|a| a.backend == Backend::Metal && a.stage == Stage::Vertex)
            .unwrap();
        let text = std::str::from_utf8(&vertex.bytes).unwrap();
        assert!(text.contains("[[buffer(15)]]"));
        assert!(text.contains("[[vertex_id]]"));
        assert!(!text.contains("[[stage_in]]"));
        if kind == Kind::Diagnostic {
            assert!(text.contains("[[buffer(0)]]"));
            assert!(text.contains("buffer_size15 / 28"));
            assert!(!text.contains("[[instance_id]]"));
        } else {
            assert!(text.contains("[[buffer(0)]]") && text.contains("[[buffer(1)]]"));
            assert!(text.contains("[[buffer(14)]]") && text.contains("[[instance_id]]"));
            assert!(text.contains("buffer_size15 / 48") && text.contains("buffer_size14 / 112"));
            let fragment = bundle
                .artifacts
                .iter()
                .find(|a| a.backend == Backend::Metal && a.stage == Stage::Fragment)
                .unwrap();
            let text = std::str::from_utf8(&fragment.bytes).unwrap();
            for required in [
                "[[buffer(0)]]",
                "[[texture(0)]]",
                "[[sampler(0)]]",
                "discard_fragment",
            ] {
                assert!(text.contains(required), "{required}");
            }
        }
    }
}
#[test]
fn hlsl_shadow_uses_full_material_layout_sampler_indirection() {
    let bundle = cook(Kind::Shadow, source(Kind::Shadow)).unwrap();
    for artifact in bundle
        .artifacts
        .iter()
        .filter(|a| a.backend == Backend::Dx12)
    {
        let text = std::str::from_utf8(&artifact.bytes).unwrap();
        if artifact.stage == Stage::Vertex {
            assert!(text.contains("register(b0)"));
        } else {
            for required in [
                "register(b1)",
                "register(t0)",
                "register(t5, space0)",
                "register(s0, space0)",
                "discard",
            ] {
                assert!(text.contains(required), "{required}\n{text}");
            }
            assert!(!text.contains("register(t1)"));
        }
    }
}

#[test]
fn shader_math_and_family_are_in_artifact_identity() {
    let a = cook(Kind::Diagnostic, source(Kind::Diagnostic)).unwrap();
    let b = cook(
        Kind::Diagnostic,
        &source(Kind::Diagnostic).replace("0.25 + 0.65", "0.20 + 0.65"),
    )
    .unwrap();
    assert_ne!(a.source_hash, b.source_hash);
    assert_ne!(a.artifacts, b.artifacts);
    assert_ne!(Kind::Diagnostic.contract(), Kind::Shadow.contract());
}

#[test]
fn sprite_contract_refuses_sampling_interpolation_and_resource_changes() {
    for changed in [
        source(Kind::Sprite)
            .replace("sampling:u32", "sampling:i32")
            .replace("==0u", "==0"),
        source(Kind::Sprite).replace("@interpolate(flat) uv_bounds", "uv_bounds"),
        source(Kind::Sprite).replace("@interpolate(flat) texel_base", "texel_base"),
        source(Kind::Sprite).replace("@binding(1) var filtering", "@binding(2) var filtering"),
        source(Kind::Sprite).replace(
            "var<uniform> camera: mat4x4f",
            "var<storage,read> camera: mat4x4f",
        ),
        source(Kind::Sprite).replace(
            "out.tint=input.tint",
            "out.tint=input.tint*textureLoad(image,vec2i(0),0)",
        ),
    ] {
        assert_ne!(changed, source(Kind::Sprite));
        assert!(cook(Kind::Sprite, &changed).is_err());
    }
    assert!(cook(Kind::Sprite, source(Kind::Shadow)).is_err());
    assert!(cook(Kind::Shadow, source(Kind::Sprite)).is_err());
}

#[test]
fn sprite_native_outputs_preserve_both_vertex_bounds_and_sampler_heap_contract() {
    let bundle = cook(Kind::Sprite, source(Kind::Sprite)).unwrap();
    for artifact in &bundle.artifacts {
        if artifact.backend == Backend::Vulkan {
            continue;
        }
        let text = std::str::from_utf8(&artifact.bytes).unwrap();
        let required: &[&str] = match (artifact.backend, artifact.stage) {
            (Backend::Metal, Stage::Vertex) => &[
                "[[buffer(0)]]",
                "[[buffer(1)]]",
                "[[buffer(15)]]",
                "[[buffer(14)]]",
                "buffer_size15 / 12",
                "buffer_size14 / 52",
                "[[vertex_id]]",
            ],
            (Backend::Metal, Stage::Fragment) => &[
                "[[texture(0)]]",
                "[[sampler(0)]]",
                "get_width",
                "get_height",
            ],
            (Backend::Dx12, Stage::Vertex) => &["register(b0)", "nointerpolation"],
            (Backend::Dx12, Stage::Fragment) => &[
                "register(t0)",
                "register(t1, space0)",
                "register(s0, space0)",
                "image.Load(",
                "nointerpolation",
            ],
            _ => unreachable!(),
        };
        for token in required {
            assert!(text.contains(token), "{token}\n{text}");
        }
        if artifact.backend == Backend::Metal && artifact.stage == Stage::Vertex {
            assert!(!text.contains("[[instance_id]]"));
            assert!(!text.contains("[[stage_in]]"));
        }
    }
}
