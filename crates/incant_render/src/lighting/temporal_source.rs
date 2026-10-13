//! Exact guarded source shared by build-time cooking and portable runtime.
pub(crate) fn temporal_grid_source() -> std::result::Result<String, &'static str> {
    let mut source = include_str!("grid.wgsl").to_owned();
    for (from, to) in [
        (
            "let lo=vec2f(id.xy)*64.0/grid.viewport.zw;",
            "let lo=(vec2f(id.xy)*64.0-vec2f(1.0))/grid.viewport.zw;",
        ),
        (
            "let hi=min(vec2f(id.xy+vec2u(1u))*64.0/grid.viewport.zw,vec2f(1.0));",
            "let hi=(min(vec2f(id.xy+vec2u(1u))*64.0,grid.viewport.zw)+vec2f(1.0))/grid.viewport.zw;",
        ),
    ] {
        if source.matches(from).count() != 1 {
            return Err("temporal cluster source substitution");
        }
        source = source.replacen(from, to, 1);
    }
    Ok(source)
}
