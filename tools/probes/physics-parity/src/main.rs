use incant_physics_parity::run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: incant-physics-parity SCENARIO WORKERS OUTPUT".into());
    }
    let result = run(args[1].parse()?, args[2].parse()?)?;
    std::fs::write(&args[3], &result.bytes)?;
    println!(
        "{{\"workers\":{},\"bodies\":{},\"bytes\":{},\"max_contacts\":{},\"max_sleeping\":{},\"wake_transitions\":{},\"joints\":{}}}",
        result.workers,
        result.bodies,
        result.bytes.len(),
        result.max_contacts,
        result.max_sleeping,
        result.wake_transitions,
        result.joints
    );
    Ok(())
}
