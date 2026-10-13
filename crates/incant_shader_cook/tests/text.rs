use incant_shader_cook::{
    Backend, Stage,
    geometry::{Kind, cook},
};

const SOURCE: &str = include_str!("../../incant_render/src/text/text.wgsl");

#[test]
fn text_contract_refuses_changed_uniforms_atlas_io_and_stage_access() {
    for changed in [
        SOURCE.replace("matrix: mat4x4f, color", "@size(80) matrix: mat4x4f, color"),
        SOURCE.replace("@group(1) @binding(0)", "@group(1) @binding(1)"),
        SOURCE.replace("@group(2) @binding(0)", "@group(3) @binding(0)"),
        SOURCE.replace("@interpolate(flat) texels", "texels"),
        SOURCE.replace("@builtin(vertex_index)", "@builtin(instance_index)"),
        SOURCE.replace("frame.matrix * node *", "frame.matrix *"),
        SOURCE.replace("frame.color.rgb * coverage", "vec3f(1) * coverage"),
        SOURCE.replace("var<uniform> node", "var<storage,read> node"),
        SOURCE.replace("var bilinear: sampler", "var bilinear: sampler_comparison"),
    ] {
        assert_ne!(SOURCE, changed);
        assert!(cook(Kind::Text, &changed).is_err(), "accepted {changed}");
    }
    assert!(cook(Kind::Ui, SOURCE).is_err());
    assert!(cook(Kind::Sprite, SOURCE).is_err());
}

#[test]
fn text_native_outputs_use_dynamic_node_frame_and_atlas_bindings() {
    for artifact in cook(Kind::Text, SOURCE).unwrap().artifacts {
        if artifact.backend == Backend::Vulkan {
            continue; // Parsed and reflected in geometry.rs alongside the other families.
        }
        let text = std::str::from_utf8(&artifact.bytes).unwrap();
        let expected: &[&str] = match (artifact.backend, artifact.stage) {
            (Backend::Metal, Stage::Vertex) => &[
                "[[buffer(0)]]",
                "[[buffer(1)]]",
                "[[buffer(2)]]",
                "[[buffer(15)]]",
                "buffer_size15 / 32",
                "[[vertex_id]]",
                "[[instance_id]]",
            ],
            (Backend::Metal, Stage::Fragment) => &[
                "[[buffer(0)]]",
                "[[texture(0)]]",
                "[[sampler(0)]]",
                "get_width",
                "get_height",
            ],
            (Backend::Dx12, Stage::Vertex) => &[
                "register(b0)",
                "register(b1)",
                "SV_VertexID",
                "nointerpolation",
            ],
            (Backend::Dx12, Stage::Fragment) => &[
                "register(b0)",
                "register(t0)",
                "register(t1, space0)",
                "register(s0, space0)",
                "nointerpolation",
            ],
            _ => unreachable!(),
        };
        for token in expected {
            assert!(text.contains(token), "missing {token}\n{text}");
        }
        if artifact.backend == Backend::Metal && artifact.stage == Stage::Vertex {
            assert!(!text.contains("[[stage_in]]"));
        }
    }
}
