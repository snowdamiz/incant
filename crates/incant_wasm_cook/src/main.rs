use std::{
    fs,
    io::{Read, Write},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
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
