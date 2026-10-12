use std::{
    fs,
    io::{Read, Write},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() == 5 && args[1] == "--capture" {
        let root = std::path::Path::new(&args[2]);
        let key = args[3].to_str().ok_or("fingerprint must be UTF-8")?;
        let captured = incant_wasm_cook::CapturedModule::load(
            &incant_wasm_cook::project_cache(root, false)?,
            key,
        )?;
        let output = std::path::Path::new(&args[4]);
        fs::create_dir(output)?;
        fs::write(output.join("original.wasm"), captured.original())?;
        fs::write(output.join("prepared.wasm"), captured.prepared())?;
        println!(
            "{}",
            serde_json::json!({"fingerprint": captured.fingerprint(), "input_sha256": captured.input_sha256(), "prepared_sha256": captured.prepared_sha256(), "registry_sha256": captured.registry_sha256()})
        );
        return Ok(());
    }
    if args.len() != 3 {
        return Err("usage: incant_wasm_cook INPUT.wasm OUTPUT.wasm".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(&args[1])?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    let prepared = incant_wasm_cook::prepare(&bytes)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?
        .write_all(&prepared)?;
    Ok(())
}
