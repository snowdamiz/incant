//! Standalone static-module proof. No compiler/JIT/module file loading.
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_types::{Transform, Velocity};
use incant_wasm_runtime::{StaticModule, embedded};
use std::io::{Read, Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "--make-scene" {
        let scene = CookedScene::new(
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
        )?;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[1])?
            .write_all(&scene.to_bytes())?;
        return Ok(());
    }
    if args.len() != 3 {
        return Err(
            "expected LINKED_MODULE COOKED_SCENE TICKS; no source/module loading supported".into(),
        );
    }
    let ticks: u32 = args[2].parse()?;
    if !(1..=1024).contains(&ticks) {
        return Err("tick limit".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&args[1])?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("scene size limit".into());
    }
    let scene = CookedScene::from_bytes(&bytes)?;
    if !scene.audio().is_empty() || !scene.spatial().is_empty() || !scene.visual_assets().is_empty()
    {
        return Err("no audio/spatial host".into());
    }
    let mut host = StaticModule::new(embedded::linked(&args[0]).ok_or("unknown linked module")?)?;
    let mut world = NativeWorld::new(scene);
    let dt = 1. / f64::from(world.tick_rate());
    for _ in 0..ticks {
        world.step()?;
        host.tick(&mut world, dt)?;
    }
    // Only a fully successful run publishes its final state. The disposable world
    // is dropped on failure; this does not roll back already accepted simulation.
    for id in 1u128..=4 {
        let row = world
            .inspect(StableId(id.to_be_bytes()))
            .ok_or("fixture entity missing")?;
        println!("{id} {}", row.transform.translation[0]);
    }
    Ok(())
}
