//! Invoke the same production host cooker as incant_render/build.rs, without a GPU.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args_os()
        .nth(1)
        .ok_or("usage: output_bundle <output-directory>")?;
    let source = [
        include_str!("../../incant_render/src/output.wgsl"),
        include_str!("../../incant_render/src/tone_map.wgsl"),
    ]
    .join("\n");
    incant_shader_cook::write_bundle(
        &incant_shader_cook::cook_output(&source)?,
        std::path::Path::new(&directory),
    )?;
    Ok(())
}
