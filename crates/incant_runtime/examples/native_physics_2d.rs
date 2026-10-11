//! Explicit non-rendering planar conformance player. It consumes v7 tile collision
//! geometry without loading textures; it does not present a game/visual session.
//! Failure before writing publishes no stream; output I/O is not atomic.
use incant_runtime::NativeWorld;
use std::io::{Read, Write};
#[path = "../../../tools/probes/native-physics/src/planar.rs"]
mod planar;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: native_physics_2d COOKED_SCENE OUTPUT".into());
    }
    const LIMIT: u64 = 128 * 1024 * 1024;
    let mut bytes = Vec::new();
    std::fs::File::open(&args[0])?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err("scene exceeds cooked byte limit".into());
    }
    let stream = planar::run(NativeWorld::from_bytes(&bytes)?)?;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    output.write_all(&stream)?;
    println!("tick=640 bytes={}", stream.len());
    Ok(())
}
