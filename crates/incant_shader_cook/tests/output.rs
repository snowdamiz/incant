use incant_shader_cook::{Backend, Stage, cook_output};
use naga::{Binding, BuiltIn, ShaderStage};
fn source() -> String {
    [
        include_str!("../../incant_render/src/output.wgsl"),
        include_str!("../../incant_render/src/tone_map.wgsl"),
    ]
    .join("\n")
}
#[test]
fn real_output_cooks_deterministically_for_every_backend_and_stage() {
    let a = cook_output(&source()).unwrap();
    let b = cook_output(&source()).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.artifacts.len(), 6);
    for backend in [Backend::Metal, Backend::Vulkan, Backend::Dx12] {
        for stage in [Stage::Vertex, Stage::Fragment] {
            assert_eq!(
                a.artifacts
                    .iter()
                    .filter(|x| x.backend == backend && x.stage == stage)
                    .count(),
                1
            );
        }
    }
}
#[test]
fn binding_layout_and_unsupported_metadata_changes_refuse() {
    for altered in [
        source().replace("@binding(1)", "@binding(2)"),
        source().replace("@group(0)", "@group(1)"),
        source().replace("pixel_grid: vec4f", "pixel_grid: vec4f, extra: vec4f"),
        source()
            .replace("pixel_grid: vec4f", "renamed: vec4f")
            .replace("display.pixel_grid", "display.renamed"),
        source().replace("fn vertex(", "fn another_vertex("),
        format!("{}\nvar<workgroup> scratch: u32;", source()),
        format!("{}\noverride setting: u32 = 1u;", source()),
        format!(
            "{}\n@group(0) @binding(2) var<storage, read> extra: array<f32>;",
            source()
        ),
    ] {
        assert!(cook_output(&altered).is_err(), "accepted changed contract");
    }
}
#[test]
fn shader_arithmetic_changes_invalidate_source_identity() {
    let a = cook_output(&source()).unwrap();
    let b = cook_output(&source().replace("const exposure = 1.0", "const exposure = 0.9")).unwrap();
    assert_ne!(a.source_hash, b.source_hash);
    assert_ne!(a.artifacts, b.artifacts);
}
#[test]
fn metal_and_hlsl_use_the_actual_fixed_hal_slots() {
    let bundle = cook_output(&source()).unwrap();
    for a in &bundle.artifacts {
        if a.backend == Backend::Vulkan {
            continue;
        }
        let text = std::str::from_utf8(&a.bytes).unwrap();
        assert!(text.contains(&a.entry));
        if a.stage == Stage::Fragment {
            match a.backend {
                Backend::Metal => {
                    assert!(text.contains("[[texture(0)]]"));
                    assert!(text.contains("[[buffer(0)]]"));
                    assert!(!text.contains("[[buffer(1)]]"));
                    assert!(!text.contains("threadgroup"));
                }
                Backend::Dx12 => {
                    assert!(text.contains("register(t0)"));
                    assert!(text.contains("register(b0)"));
                    assert!(!text.contains("register(t1)"));
                }
                _ => unreachable!(),
            }
        } else if a.backend == Backend::Dx12 {
            assert!(text.contains("register(b1)"));
            assert!(text.contains("SV_VertexID"));
        }
    }
}
#[test]
fn spirv_validates_and_preserves_hal_viewport_coordinates() {
    let bundle = cook_output(&source()).unwrap();
    for a in bundle
        .artifacts
        .iter()
        .filter(|a| a.backend == Backend::Vulkan)
    {
        let words: Vec<_> = a
            .bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|x| u32::from_le_bytes(*x))
            .collect();
        let mut cursor = 5;
        let mut negate = 0;
        while cursor < words.len() {
            let len = (words[cursor] >> 16) as usize;
            assert!(len > 0 && cursor + len <= words.len());
            if words[cursor] & 0xffff == 127 {
                negate += 1;
            } // OpFNegate
            cursor += len;
        }
        // The fixed vertex performs no runtime negation. Naga's default
        // ADJUST_COORDINATE_SPACE would introduce an extra Y negation here,
        // conflicting with wgpu-hal's already-negative viewport height.
        if a.stage == Stage::Vertex {
            assert_eq!(negate, 0);
        }
        let module = naga::front::spv::Frontend::new(
            words.into_iter(),
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
                ShaderStage::Vertex
            } else {
                ShaderStage::Fragment
            }
        );
        if a.stage == Stage::Fragment {
            let mut bindings: Vec<_> = module
                .global_variables
                .iter()
                .filter_map(|(_, g)| g.binding)
                .map(|b| (b.group, b.binding))
                .collect();
            bindings.sort();
            assert_eq!(bindings, [(0, 0), (0, 1)]);
        } else {
            assert!(
                module.entry_points[0]
                    .function
                    .arguments
                    .iter()
                    .any(|arg| arg.binding == Some(Binding::BuiltIn(BuiltIn::VertexIndex)))
            );
        }
    }
}
