fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: incant-native-physics-parity WORKERS OUTPUT".into());
    }
    let workers: usize = args[0].parse()?;
    let bytes = incant_native_physics_parity::run(workers)?;
    std::fs::write(&args[1], &bytes)?;
    println!("workers={workers} bytes={}", bytes.len());
    Ok(())
}
