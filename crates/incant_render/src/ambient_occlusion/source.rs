//! Exact engine-owned SSAO source assembly shared by host cook and renderer.
pub fn material(prepass: bool) -> Option<String> {
    let base = include_str!("../model_material.wgsl");
    if prepass {
        let (vertex, _) = base.split_once("@fragment fn fragment")?;
        return Some(format!("{vertex}\n{}", include_str!("prepass.wgsl")));
    }
    let old = "(1.0-reflectance)*(1.0-metallic)*base.rgb*irradiance";
    if base.matches(old).count() != 1 {
        return None;
    }
    // Only diffuse irradiance is occluded: specular visibility needs a different
    // angular integral. Material-authored AO keeps its existing independent role.
    Some(format!(
        "{}\n{}\n@group(0) @binding(1) var diffuse_accessibility:texture_2d<f32>;\n{}",
        include_str!("../shadows/shade.wgsl"),
        include_str!("../lighting/shade.wgsl"),
        base.replace(
            old,
            &format!("({old})*textureLoad(diffuse_accessibility,vec2i(in.position.xy),0).r")
        )
    ))
}
