fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "--fixture-2d" {
        std::fs::write(
            &args[1],
            incant_native_physics_parity::planar::scene()?.to_bytes(),
        )?;
        return Ok(());
    }
    let planar = args.len() == 3 && args[2] == "--2d";
    if args.len() != 2 && !planar {
        return Err(
            "usage: incant-native-physics-parity WORKERS OUTPUT [--2d] | --fixture-2d OUTPUT"
                .into(),
        );
    }
    let workers: usize = args[0].parse()?;
    let bytes = if planar {
        incant_native_physics_parity::run_2d(workers)?
    } else {
        incant_native_physics_parity::run(workers)?
    };
    std::fs::write(&args[1], &bytes)?;
    println!("workers={workers} bytes={}", bytes.len());
    Ok(())
}
