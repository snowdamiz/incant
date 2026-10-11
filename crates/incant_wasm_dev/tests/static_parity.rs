#![cfg(feature = "static-conformance")]
use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, NativeWorld, StableId, StructuralCommand,
};
use incant_types::{Transform, Velocity};
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
#[test]
fn same_rust_wasm_has_exact_static_and_development_state_streams() {
    let scene = CookedScene::new(
        StableId([0; 16]),
        64,
        (1u128..=4)
            .map(|n| CookedEntity {
                id: StableId(n.to_be_bytes()),
                parent: (n == 2).then_some(StableId(1u128.to_be_bytes())),
                transform: Transform::default(),
                velocity: (n % 2 == 1).then_some(Velocity {
                    linear: [1., 0., 0.],
                }),
            })
            .collect(),
    )
    .unwrap();
    let mut a = NativeWorld::new(scene.clone());
    let mut b = NativeWorld::new(scene);
    let mut static_host = StaticModule::new(embedded::linked("bulk").unwrap()).unwrap();
    let mut dev_host = DevModuleHost::new(include_bytes!(concat!(
        env!("INCANT_WASM_BUNDLE"),
        "/m0.original.wasm"
    )))
    .unwrap();
    for tick in 0..16 {
        if tick == 4 {
            for world in [&mut a, &mut b] {
                let mut commands = FrameCommands::default();
                for n in 5u128..=94 {
                    commands
                        .push(StructuralCommand::Spawn(CookedEntity {
                            id: StableId(n.to_be_bytes()),
                            parent: None,
                            transform: Transform::default(),
                            velocity: None,
                        }))
                        .unwrap();
                }
                world.apply(&mut commands).unwrap();
            }
        }
        a.step().unwrap();
        b.step().unwrap();
        static_host.tick(&mut a, 1. / 64.).unwrap();
        dev_host.tick(&mut b, 1. / 64.).unwrap();
        for id in 1u128..=(if tick >= 4 { 94 } else { 4 }) {
            assert_eq!(
                a.inspect(StableId(id.to_be_bytes())),
                b.inspect(StableId(id.to_be_bytes())),
                "tick {tick}, id {id}"
            );
        }
    }
}
