use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_types::Velocity;
use std::{hint::black_box, time::Instant};

fn id(n: usize) -> StableId {
    StableId((n as u128).to_be_bytes())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let count: usize = args[1].parse()?;
    let mode = args[2].as_str();
    assert!(matches!(mode, "static" | "moving" | "parented"));
    assert!((1..=100_000).contains(&count));
    let entities = (1..=count)
        .map(|n| {
            let mut e = CookedEntity {
                id: id(n),
                parent: (mode == "parented" && n % 8 != 1).then(|| id(n - 1)),
                transform: Default::default(),
                velocity: (mode != "static").then_some(Velocity { linear: [0.; 3] }),
            };
            e.transform.translation = [(n % 32) as f64, 0., (n / 32) as f64];
            if let Some(v) = &mut e.velocity {
                v.linear = [0.25, -0.125, 0.0625];
            }
            e
        })
        .collect();
    let mut world = NativeWorld::new(CookedScene::new(id(0), 60, entities)?);
    for _ in 0..20 {
        world.step()?;
    }
    let mut ns = Vec::with_capacity(120);
    for _ in 0..120 {
        let start = Instant::now();
        world.step()?;
        ns.push(start.elapsed().as_nanos());
        black_box(world.tick());
    }
    ns.sort_unstable();
    let last = world.inspect(id(count)).unwrap();
    println!("{count},{mode},{},{},{},{},{},{}", ns[59], ns[113], ns[0], ns[119], world.tick(), last.world_transform[3][0]);
    Ok(())
}
