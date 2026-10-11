#![cfg(feature = "embedded-fixture")]
use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, NativeWorld, StableId, StructuralCommand,
};
use incant_types::{Transform, Velocity};
use incant_wasm_runtime::{StaticModule, embedded};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            id(99),
            64,
            (1..=2)
                .map(|n| CookedEntity {
                    id: id(n),
                    parent: None,
                    transform: Transform::default(),
                    velocity: None,
                })
                .collect(),
        )
        .unwrap(),
    )
}
fn host(name: &str) -> StaticModule {
    StaticModule::new(embedded::linked(name).unwrap()).unwrap()
}
#[test]
fn actual_static_rust_module_reacquires_tables_and_preserves_identity() {
    let mut world = world();
    let mut host = host("bulk");
    host.tick(&mut world, 1. / 64.).unwrap();
    let mut commands = FrameCommands::default();
    for n in 3..=92 {
        commands
            .push(StructuralCommand::Spawn(CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: Some(Velocity {
                    linear: [1., 0., 0.],
                }),
            }))
            .unwrap();
    }
    world.apply(&mut commands).unwrap();
    host.tick(&mut world, 1. / 64.).unwrap();
    assert_eq!(
        world.inspect(id(1)).unwrap().transform.translation[0],
        2. / 64.
    );
    assert_eq!(
        world.inspect(id(92)).unwrap().transform.translation[0],
        1. / 64.
    );
    assert_eq!(world.tick(), 0);
    assert!(host.tick(&mut super_world(), 1. / 64.).is_err());
    assert!(!host.is_failed());
    assert!(host.tick(&mut world, f64::NAN).is_err());
    assert!(!host.is_failed());
    host.tick(&mut world, 1. / 64.).unwrap();
}
fn super_world() -> NativeWorld {
    world()
}
#[test]
fn numeric_trap_domains_and_structural_graph_restore_values_atomically() {
    for name in ["trap", "nan", "quaternion", "scale", "cycle"] {
        let mut world = world();
        let before = world.inspect(id(1)).unwrap();
        let mut host = host(name);
        assert!(host.tick(&mut world, 1. / 64.).is_err(), "{name}");
        assert!(host.is_failed());
        assert_eq!(world.inspect(id(1)).unwrap(), before, "{name}");
        assert!(world.inspect(id(3)).is_none());
        assert_eq!(world.inspect(id(2)).unwrap().parent, None);
        assert_eq!(world.tick(), 0);
        assert!(host.tick(&mut world, 1. / 64.).is_err());
    }
}
#[test]
fn finite_hierarchy_overflow_restores_committed_global_values() {
    let mut world = world();
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetParent {
            entity: id(2),
            parent: Some(id(1)),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    let before = world.inspect(id(2)).unwrap();
    let mut host = host("hierarchy");
    assert!(host.tick(&mut world, 1. / 64.).is_err());
    assert_eq!(world.inspect(id(2)).unwrap(), before);
}
#[test]
fn c_traps_import_errors_and_recursion_never_cross_rust_or_resume() {
    for name in [
        "oob",
        "import-oob",
        "epoch",
        "loop",
        "stack",
        "indirect-loop",
        "calls",
        "commands",
    ] {
        let mut world = world();
        let before = world.inspect(id(1)).unwrap();
        let mut host = host(name);
        let error = host.tick(&mut world, 1. / 64.).unwrap_err();
        if name == "loop" || name == "indirect-loop" {
            assert!(matches!(error, incant_wasm_runtime::StaticError::Fuel));
        }
        if name == "calls" {
            assert!(error.to_string().contains("host call quota"));
        }
        if name == "commands" {
            assert!(error.to_string().contains("command quota"));
        }
        assert!(host.is_failed());
        assert_eq!(world.inspect(id(1)).unwrap(), before);
        assert!(host.tick(&mut world, 1. / 64.).is_err());
        // A trapped instance must leave the same thread's trap/stack state usable.
        let mut healthy = StaticModule::new(embedded::linked("bulk").unwrap()).unwrap();
        healthy.tick(&mut world, 1. / 64.).unwrap();
    }
}
#[test]
fn startup_is_metered_and_has_no_world_capability() {
    for name in ["start-import", "start-loop", "bad-version"] {
        assert!(
            StaticModule::new(embedded::linked(name).unwrap()).is_err(),
            "{name}"
        );
    }
}
#[test]
fn independent_instances_have_separate_state_and_parallel_trap_targets() {
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                for _ in 0..5 {
                    let mut w = world();
                    let mut a = host("bulk");
                    let mut b = host("loop");
                    assert!(b.tick(&mut w, 1. / 64.).is_err());
                    a.tick(&mut w, 1. / 64.).unwrap();
                    assert_eq!(w.inspect(id(1)).unwrap().transform.translation[0], 1. / 64.);
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
}

#[test]
fn separate_wasm_multiply_and_add_are_not_fused_by_the_c_compiler() {
    let mut w = world();
    w.with_numeric_columns(|cols| {
        cols.for_each_chunk(|c| {
            c.transforms[0] = 1. + 2f64.powi(-27);
            c.transforms[1] = 1. - 2f64.powi(-27);
        })
    });
    host("rounding").tick(&mut w, 1. / 64.).unwrap();
    assert_eq!(w.inspect(id(1)).unwrap().transform.translation[0], 0.);
}
