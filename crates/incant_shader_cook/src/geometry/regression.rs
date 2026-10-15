//! Shared emitter changes must not perturb the established ordinary catalog.
use super::*;

#[test]
fn ordinary_material_artifacts_are_byte_identical_to_main346_emitter() {
    // This fixture is the exact material emitter at main346/25f648144cc2.
    // Run both emitters over the same source, IR, pinned Naga and options.
    assert_eq!(
        <[u8; 32]>::from(Sha256::digest(include_bytes!(
            "../../tests/fixtures/material_emit_main346.rs"
        ))),
        [
            151, 111, 118, 223, 220, 135, 157, 207, 45, 114, 175, 89, 103, 62, 123, 95, 174, 176,
            162, 137, 148, 220, 222, 205, 182, 236, 39, 64, 62, 124, 76, 114
        ]
    );
    let source = [
        include_str!("../../../incant_render/src/shadows/shade.wgsl"),
        include_str!("../../../incant_render/src/lighting/shade.wgsl"),
        include_str!("../../../incant_render/src/model_material.wgsl"),
    ]
    .join("\n");
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    let info = valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
        .validate(&module)
        .unwrap();
    interpolation::validate(Kind::Material, &module).unwrap();
    contract::validate(Kind::Material, &module, &info).unwrap();
    for stage in [ShaderStage::Vertex, ShaderStage::Fragment] {
        let entry = if stage == ShaderStage::Vertex {
            "vertex"
        } else {
            "fragment"
        };
        let (compact, compact_info) = pipeline_constants::process_overrides(
            &module,
            &info,
            Some((stage, entry)),
            &Default::default(),
        )
        .unwrap();
        for backend in [Backend::Metal, Backend::Vulkan, Backend::Dx12] {
            assert_eq!(
                material_emit::emit(&compact, &compact_info, &module, stage, entry, backend)
                    .unwrap(),
                prior_material_emit::emit(&compact, &compact_info, &module, stage, entry, backend)
                    .unwrap(),
                "ordinary {backend:?} {stage:?} artifact/entry changed"
            );
        }
    }
}
