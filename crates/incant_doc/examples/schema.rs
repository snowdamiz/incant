//! Export the document registry without building the editor/headless host.
//! Host-only input/save/query schemas remain owned by the headless schema command.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = std::path::PathBuf::from(args.next().ok_or("usage: schema DIRECTORY")?);
    if args.next().is_some() {
        return Err("usage: schema DIRECTORY".into());
    }
    std::fs::create_dir_all(&output)?;
    for (name, schema) in incant_doc::schema_registry() {
        std::fs::write(
            output.join(format!("{name}.schema.json")),
            serde_json::to_string_pretty(&schema)? + "\n",
        )?;
    }
    Ok(())
}
