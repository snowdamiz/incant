//! Typed v1/v4 refusal fixtures. This tool creates no authoring project or art.
#[path = "../tests/support/spatial.rs"]
mod spatial;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).ok_or("expected output directory")?);
    std::fs::create_dir_all(&out)?;
    for (name, scene) in ["plain", "camera-only", "manifest-only"]
        .into_iter()
        .zip(spatial::scenes())
    {
        std::fs::write(out.join(format!("{name}.scene")), scene.to_bytes())?;
    }
    Ok(())
}
