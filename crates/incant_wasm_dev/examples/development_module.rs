//! Source-free *development* JIT probe, not a shipping/wasm2c executable.
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_types::{Transform, Velocity};
use incant_wasm_dev::DevModuleHost;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or("expected Wasm file")?;
    let mut file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    use std::io::Read;
    (&mut file).take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    let mut host = DevModuleHost::new(&bytes)?;
    let mut world = NativeWorld::new(CookedScene::new(
        StableId([0; 16]),
        64,
        (1u128..=4)
            .map(|id| CookedEntity {
                id: StableId(id.to_be_bytes()),
                parent: None,
                transform: Transform::default(),
                velocity: (id % 2 == 1).then_some(Velocity {
                    linear: [1., 0., 0.],
                }),
            })
            .collect(),
    )?);
    for _ in 0..8 {
        world.step()?;
        host.tick(&mut world, 1. / 64.)?;
    }
    for id in 1u128..=4 {
        let actual = world
            .inspect(StableId(id.to_be_bytes()))
            .unwrap()
            .transform
            .translation[0];
        let expected = if id % 2 == 1 { 0.25 } else { 0.125 };
        if actual != expected {
            return Err("module did not publish expected native motion".into());
        }
        println!("{id} {actual}");
    }
    Ok(())
}
