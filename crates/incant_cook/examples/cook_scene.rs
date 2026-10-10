fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: cook_scene <project.json> <scene-ulid> <output.scene>".into());
    }
    let project: incant_doc::Project = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let bytes = incant_cook::cook_scene(&project, &args[1])?;
    std::fs::write(&args[2], &bytes)?;
    println!("Cooked {} bytes", bytes.len());
    Ok(())
}
