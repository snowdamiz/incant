#[path = "support/navigation_queries.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn navigation_loans_preserve_committed_paths_restore_on_error_and_unwind_and_expire() {
    let mut world = NativeWorld::new(linked_scene(true));
    let before = world.navigation_path(id(1), &path()).unwrap().unwrap();
    assert_eq!(before.traversals.len(), 1);
    let grid_before = world.grid_path(id(1), &grid()).unwrap().unwrap();
    let mut retained = None;
    for panic in [false, true] {
        let result = catch_unwind(AssertUnwindSafe(|| {
            world
                .with_numeric_physics(|columns, cap| {
                    assert_eq!(
                        cap.navigation_path(id(1), &path()).unwrap().unwrap(),
                        before
                    );
                    retained = Some(cap.clone());
                    columns.for_each_chunk(|c| {
                        if let Some(i) = c.ids.iter().position(|&e| e == id(2)) {
                            c.transforms[i * TRANSFORM_STRIDE] = 40.;
                            assert_eq!(
                                cap.navigation_path(id(1), &path()).unwrap().unwrap(),
                                before
                            );
                            c.transforms[i * TRANSFORM_STRIDE] = 0.;
                        }
                    });
                    columns.validate().unwrap();
                    if panic {
                        panic!("exercise RAII");
                    }
                    Err::<(), _>("refused")
                })
                .unwrap()
        }));
        if panic {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap(), Err("refused"));
        }
        let old = retained.take().unwrap();
        assert!(matches!(
            old.navigation_path(id(1), &path()),
            Err(QueryError::Expired)
        ));
        assert!(matches!(
            old.grid_path(id(1), &grid()),
            Err(QueryError::Expired)
        ));
        world
            .with_numeric_physics(|_, current| {
                assert!(matches!(
                    old.grid_path(id(1), &grid()),
                    Err(QueryError::Expired)
                ));
                assert_eq!(
                    current.grid_path(id(1), &grid()).unwrap().unwrap(),
                    grid_before
                );
                assert!(current.trigger_events().is_ok());
                assert!(current.trigger_events_2d().is_ok());
            })
            .unwrap();
        world.step().unwrap();
        assert_eq!(
            world.navigation_path(id(1), &path()).unwrap().unwrap(),
            before
        );
    }
}
#[test]
fn stale_numeric_sources_refuse_loan_until_commit_then_publish_new_generation() {
    let mut world = NativeWorld::new(scene());
    let before = world.navigation_path(id(1), &path()).unwrap().unwrap();
    world.with_numeric_columns(|columns| {
        columns.for_each_chunk(|c| {
            if let Some(i) = c.ids.iter().position(|&e| e == id(3)) {
                c.transforms[i * TRANSFORM_STRIDE + 2] = 4.;
            }
        })
    });
    assert!(
        world
            .with_numeric_physics(|_, _| panic!("must not lend stale cache"))
            .is_err()
    );
    world.apply(&mut FrameCommands::default()).unwrap();
    let after = world
        .with_numeric_physics(|_, cap| cap.navigation_path(id(1), &path()).unwrap().unwrap())
        .unwrap();
    assert!(after.generation > before.generation);
    assert_eq!(after.points.len(), 2);
    assert!(
        NativeWorld::new(linked_scene(false))
            .navigation_path(id(1), &path())
            .unwrap()
            .is_none()
    );
}

#[test]
fn restore_advances_grid_generation_without_reactivating_old_query_capability() {
    let mut cfg = config();
    cfg.mesh = None;
    let scene = CookedScene::new(id(0), 60, vec![entity(1), entity(4)])
        .unwrap()
        .with_navigation(vec![cfg])
        .unwrap();
    let mut world = NativeWorld::new(scene);
    let before = world.grid_path(id(1), &grid()).unwrap().unwrap();
    let saved = world.save_bytes().unwrap();
    let old = world.with_numeric_physics(|_, cap| cap).unwrap();
    world.restore_bytes(&saved).unwrap();
    world
        .with_numeric_physics(|_, cap| {
            assert!(matches!(
                old.grid_path(id(1), &grid()),
                Err(QueryError::Expired)
            ));
            let after = cap.grid_path(id(1), &grid()).unwrap().unwrap();
            assert_eq!(after.cells, before.cells);
            assert_eq!(after.cost, before.cost);
            assert!(after.generation > before.generation);
        })
        .unwrap();
}
