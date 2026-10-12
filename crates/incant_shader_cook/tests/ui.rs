use incant_shader_cook::{
    Backend, Stage,
    geometry::{Kind, cook},
};
const SOURCE: &str = include_str!("../../incant_render/src/ui/ui.wgsl");
#[test]
fn ui_contract_refuses_changed_frame_index_interpolation_and_visibility() {
    for changed in [
        SOURCE.replace("@builtin(vertex_index)", "@builtin(instance_index)"),
        SOURCE.replace("@interpolate(flat) rect", "rect"),
        SOURCE.replace("@interpolate(flat) color", "color"),
        SOURCE.replace("encode: vec4f,", "encode: vec4f, extra: vec4f,"),
        SOURCE.replace("scale: vec2f,", "@size(16) scale: vec2f,"),
        SOURCE.replace("@group(1) @binding(1)", "@group(1) @binding(2)"),
        SOURCE.replace("var<uniform> frame", "var<storage,read> frame"),
        SOURCE.replace(
            "out.color = quad.color;",
            "out.color = quad.color * textureLoad(atlas, vec2i(0), 0);",
        ),
        SOURCE.replace("if frame.encode.x == 0.0", "if input.color.x == 0.0"),
    ] {
        assert_ne!(changed, SOURCE);
        assert!(
            cook(Kind::Ui, &changed).is_err(),
            "accepted changed UI contract"
        );
    }
    assert!(cook(Kind::Sprite, SOURCE).is_err());
}
#[test]
fn ui_native_artifacts_preserve_instance_fetch_uniform_and_sampler_contracts() {
    let bundle = cook(Kind::Ui, SOURCE).unwrap();
    for a in &bundle.artifacts {
        if a.backend == Backend::Vulkan {
            continue;
        }
        let source = std::str::from_utf8(&a.bytes).unwrap();
        let required: &[&str] = match (a.backend, a.stage) {
            (Backend::Metal, Stage::Vertex) => &[
                "[[buffer(0)]]",
                "[[buffer(1)]]",
                "[[buffer(15)]]",
                "[[vertex_id]]",
                "[[instance_id]]",
                "buffer_size15 / 64",
            ],
            (Backend::Metal, Stage::Fragment) => &[
                "[[buffer(0)]]",
                "[[texture(0)]]",
                "[[sampler(0)]]",
                "discard_fragment",
                "get_width",
            ],
            (Backend::Dx12, Stage::Vertex) => &["register(b0)", "SV_VertexID", "nointerpolation"],
            (Backend::Dx12, Stage::Fragment) => &[
                "register(b0)",
                "register(t0)",
                "register(t1, space0)",
                "register(s0, space0)",
                "GetDimensions",
                "nointerpolation",
                "discard",
            ],
            _ => unreachable!(),
        };
        for token in required {
            assert!(source.contains(token), "{token}\n{source}");
        }
        if a.backend == Backend::Metal && a.stage == Stage::Vertex {
            assert!(!source.contains("[[stage_in]]"));
        }
    }
}
