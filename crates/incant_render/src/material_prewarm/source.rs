//! Keep the exact material vertex/UV interface and alpha inputs. Only fragment
//! RGB shading changes while a real pipeline is pending.
pub fn fallback() -> Option<String> {
    let source = include_str!("../model_material.wgsl");
    if source.matches("@fragment fn fragment").count() != 1 {
        return None;
    }
    let (vertex, _) = source.split_once("@fragment fn fragment")?;
    Some(format!("{vertex}\n{}", include_str!("fallback.wgsl")))
}
