#[path = "support/navigation_queries.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
#[test]
fn immutable_navigation_snapshots_retain_exact_committed_generation_after_motion_and_world_drop() {
    let mut w = NativeWorld::new(scene());
    let snapshot = w.navigation_snapshot().unwrap();
    let original = snapshot.navigation_path(id(1), &path()).unwrap().unwrap();
    assert!(original.points.len() > 2);
    w.with_numeric_columns(|cols| {
        cols.for_each_chunk(|c| {
            if let Some(i) = c.ids.iter().position(|&x| x == id(3)) {
                c.transforms[i * TRANSFORM_STRIDE + 2] = 4.;
            }
        })
    });
    assert!(w.navigation_snapshot().is_err());
    assert_eq!(
        snapshot.navigation_path(id(1), &path()).unwrap().unwrap(),
        original
    );
    w.apply(&mut FrameCommands::default()).unwrap();
    let current = w.navigation_snapshot().unwrap();
    let after = current.navigation_path(id(1), &path()).unwrap().unwrap();
    assert_eq!(after.points.len(), 2);
    assert!(after.generation > original.generation);
    drop(w);
    assert_eq!(
        snapshot.navigation_path(id(1), &path()).unwrap().unwrap(),
        original
    );
    assert_eq!(
        current.navigation_path(id(1), &path()).unwrap().unwrap(),
        after
    );
    assert_eq!(
        snapshot.grid_path(id(1), &grid()).unwrap(),
        current.grid_path(id(1), &grid()).unwrap()
    );
    // Exercise the shared fixture's linked route through the same accessor.
    let linked = NativeWorld::new(linked_scene(true))
        .navigation_snapshot()
        .unwrap();
    assert_eq!(
        linked
            .navigation_path(id(1), &path())
            .unwrap()
            .unwrap()
            .traversals
            .len(),
        1
    );
}
