//! Private chart-detail hypothesis, not a production sampling policy.
//! Apple WWDC22/10103 recommends log2(render/output)-1 for temporal sampling.
//! At half dimensions this is -2; high-frequency flicker must still be reviewed.
pub(crate) fn source() -> String {
    let mut source = include_str!("../model_material.wgsl").to_owned();
    for map in ["base", "emissive"] {
        let before = format!("textureSample({map}_map,{map}_sampler,in.uv)");
        let after = format!("textureSampleBias({map}_map,{map}_sampler,in.uv,-2.0)");
        assert_eq!(source.matches(&before).count(), 1);
        source = source.replace(&before, &after);
    }
    format!(
        "{}\n{}\n{}",
        include_str!("../shadows/shade.wgsl"),
        include_str!("../lighting/shade.wgsl"),
        source
    )
}

#[test]
fn biased_chart_shader_validates_without_changing_other_material_samples() {
    let source = source();
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    assert_eq!(source.matches("textureSampleBias(").count(), 2);
    for map in ["mr", "normal", "occlusion"] {
        assert!(source.contains(&format!("textureSample({map}_map,{map}_sampler,in.uv)")));
    }
}
