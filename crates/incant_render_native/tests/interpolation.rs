#![cfg(feature = "experimental-pose-interpolation")]
mod common;
use common::*;
use glam::DVec3;
use incant_render_native::{
    VisualCaches,
    interpolation::{PoseError, PoseRenderInput, PoseSyncError},
};
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StructuralCommand as Cmd};
use incant_types::{CameraProjection, Transform};
use std::{
    f64::consts::FRAC_PI_2,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
};

fn fixture() -> NativeWorld {
    let nodes = [
        (1, None, [0., 0., 0.], [2., 3., 1.]),
        (2, Some(id(1)), [2., 0., 0.], [1.; 3]),
        (3, Some(id(1)), [0., 1., 0.], [1.; 3]),
    ]
    .map(|(n, parent, translation, scale)| CookedEntity {
        id: id(n),
        parent,
        transform: Transform {
            translation,
            scale,
            ..Default::default()
        },
        velocity: None,
    })
    .to_vec();
    NativeWorld::new(
        CookedScene::new(id(99), 60, nodes)
            .unwrap()
            .with_spatial(vec![], vec![declaration(2), declaration(3)])
            .unwrap(),
    )
}
fn load_pose(w: &NativeWorld) -> PoseRenderInput {
    PoseRenderInput::load(
        &w.spatial_view().unwrap(),
        VisualCaches {
            models: Path::new("absent-model-cache"),
            textures: Path::new("absent-texture-cache"),
        },
        id(2),
        0,
    )
    .unwrap()
}
fn rotate(w: &mut NativeWorld, radians: f64) {
    w.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            for (i, entity) in chunk.ids.iter().enumerate() {
                if *entity == id(1) {
                    chunk.transforms[i * 10 + 3..i * 10 + 7].copy_from_slice(&[
                        0.,
                        0.,
                        (radians / 2.).sin(),
                        (radians / 2.).cos(),
                    ]);
                }
            }
        })
    });
}
fn sync(w: &NativeWorld, p: &mut PoseRenderInput, epoch: u64) {
    p.try_sync(&w.spatial_view().unwrap(), epoch, |_, _, _| Ok::<_, ()>(()))
        .unwrap();
}
fn point(
    p: &mut PoseRenderInput,
    alpha: f64,
) -> (DVec3, incant_render_native::interpolation::PoseReceipt) {
    p.present(alpha, |input, cam, receipt| {
        Ok::<_, ()>((
            input
                .nodes
                .iter()
                .find(|n| n.id == cam)
                .unwrap()
                .world
                .w_axis
                .truncate(),
            receipt,
        ))
    })
    .unwrap()
}

#[test]
fn invisible_parent_local_rotation_preserves_camera_orbit_and_light_offset() {
    let mut w = fixture();
    let mut p = load_pose(&w);
    let before = matrices(p.committed());
    rotate(&mut w, FRAC_PI_2);
    w.step().unwrap();
    sync(&w, &mut p, 0);
    let after = matrices(p.committed());
    assert_eq!(p.committed().input().nodes.len(), 2);
    for alpha in [0., 0.25, 0.5, 0.75, 1.] {
        p.present(alpha, |input, camera, receipt| {
            let angle = FRAC_PI_2 * alpha;
            let cam = input.nodes.iter().find(|n| n.id == camera).unwrap();
            let light = input.nodes.iter().find(|n| n.light.is_some()).unwrap();
            assert!(
                (cam.world.w_axis.truncate() - DVec3::new(4. * angle.cos(), 4. * angle.sin(), 0.))
                    .length()
                    < 1e-12
            );
            assert!(
                (light.world.w_axis.truncate()
                    - DVec3::new(-3. * angle.sin(), 3. * angle.cos(), 0.))
                .length()
                    < 1e-12
            );
            assert_eq!((receipt.previous_tick, receipt.current_tick), (0, 1));
            if alpha == 0. {
                assert_eq!(
                    input
                        .nodes
                        .iter()
                        .map(|n| n.world.to_cols_array())
                        .collect::<Vec<_>>(),
                    before
                );
            }
            if alpha == 1. {
                assert_eq!(
                    input
                        .nodes
                        .iter()
                        .map(|n| n.world.to_cols_array())
                        .collect::<Vec<_>>(),
                    after
                );
            }
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(matrices(p.committed()), after);
    }
    // The midpoint must remain on the radius-four orbit; lerping flattened
    // global positions would shrink it to sqrt(8) and fail this test.
    assert!((point(&mut p, 0.5).0.length() - 4.).abs() < 1e-12);
}

#[test]
fn every_tick_advances_history_but_refresh_never_does_and_gap_snaps() {
    let mut w = fixture();
    let mut p = load_pose(&w);
    for tick in 1..=6 {
        rotate(&mut w, tick as f64 * 0.1);
        w.step().unwrap();
        sync(&w, &mut p, 0);
        if tick == 2 || tick == 6 {
            for fraction in [0.1, 0.4, 0.9] {
                let (_, r) = point(&mut p, fraction);
                assert_eq!((r.previous_tick, r.current_tick), (tick - 1, tick));
            }
            sync(&w, &mut p, 0);
            assert_eq!(point(&mut p, 0.5).1.previous_tick, tick - 1);
        }
    }
    w.step().unwrap();
    w.step().unwrap();
    sync(&w, &mut p, 0);
    let (a, r) = point(&mut p, 0.);
    let b = point(&mut p, 1.).0;
    assert_eq!(a, b);
    assert_eq!((r.previous_tick, r.current_tick), (8, 8));
}

#[test]
fn rejected_and_unwinding_candidates_and_overlays_preserve_accepted_history() {
    let mut w = fixture();
    let mut p = load_pose(&w);
    rotate(&mut w, 0.4);
    w.step().unwrap();
    sync(&w, &mut p, 0);
    let previous = point(&mut p, 0.5);
    let committed = matrices(p.committed());
    rotate(&mut w, 0.8);
    w.step().unwrap();
    assert!(matches!(
        p.try_sync(&w.spatial_view().unwrap(), 1, |_, _, _| Err::<(), _>("no")),
        Err(PoseSyncError::Consumer("no"))
    ));
    assert!(
        catch_unwind(AssertUnwindSafe(|| p.try_sync(
            &w.spatial_view().unwrap(),
            1,
            |_, _, _| -> Result<(), ()> { panic!("candidate") }
        )))
        .is_err()
    );
    assert_eq!(point(&mut p, 0.5), previous);
    assert!(p.present(0.2, |_, _, _| Err::<(), _>(())).is_err());
    assert_eq!(matrices(p.committed()), committed);
    assert!(
        catch_unwind(AssertUnwindSafe(
            || p.present(0.2, |_, _, _| -> Result<(), ()> { panic!("overlay") })
        ))
        .is_err()
    );
    assert_eq!(matrices(p.committed()), committed);
    assert_eq!(point(&mut p, 0.5), previous);
    for bad in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            p.present(bad, |_, _, _| -> Result<(), ()> { panic!("must refuse") }),
            Err(PoseSyncError::Pose(PoseError::Fraction))
        ));
    }
    sync(&w, &mut p, 1);
    assert_eq!(point(&mut p, 0.).0, point(&mut p, 1.).0);
}

#[test]
fn same_tick_teleport_requires_epoch_and_backward_epoch_or_foreign_world_refuses() {
    let mut w = fixture();
    let mut p = load_pose(&w);
    rotate(&mut w, 1.);
    assert!(matches!(
        p.try_sync(&w.spatial_view().unwrap(), 0, |_, _, _| Ok::<_, ()>(())),
        Err(PoseSyncError::Pose(PoseError::SameTickMutation))
    ));
    sync(&w, &mut p, 1);
    assert_eq!(point(&mut p, 0.).0, point(&mut p, 1.).0);
    assert!(matches!(
        p.try_sync(&w.spatial_view().unwrap(), 0, |_, _, _| Ok::<_, ()>(())),
        Err(PoseSyncError::Pose(PoseError::Sequence))
    ));
    let other = fixture();
    assert!(
        p.try_sync(
            &other.spatial_view().unwrap(),
            1,
            |_, _, _| -> Result<(), ()> { panic!("foreign") }
        )
        .is_err()
    );
}

#[test]
fn reparent_and_camera_changes_reset_but_rejected_replacement_does_not() {
    let mut w = fixture();
    let mut p = load_pose(&w);
    rotate(&mut w, 0.5);
    w.step().unwrap();
    sync(&w, &mut p, 0);
    let old = point(&mut p, 0.4);
    w.apply(&mut queue([Cmd::SetParent {
        entity: id(2),
        parent: None,
    }]))
    .unwrap();
    assert!(
        p.try_sync(&w.spatial_view().unwrap(), 0, |_, _, _| Err::<(), _>(()))
            .is_err()
    );
    assert_eq!(point(&mut p, 0.4), old);
    sync(&w, &mut p, 0);
    assert_eq!(point(&mut p, 0.).0, point(&mut p, 1.).0);
    let mut camera = declaration(2);
    camera.camera.as_mut().unwrap().projection = CameraProjection::PixelPerfect {
        pixels_per_unit: 16.,
        reference_resolution: [320, 240],
    };
    w.apply(&mut queue([Cmd::SetSpatial {
        entity: id(2),
        spatial: Some(camera),
    }]))
    .unwrap();
    assert!(matches!(
        p.try_sync(&w.spatial_view().unwrap(), 0, |_, _, _| -> Result<(), ()> {
            panic!("unsupported")
        }),
        Err(PoseSyncError::Pose(PoseError::Unsupported))
    ));
}

#[test]
fn mirrored_scale_transition_snaps_entire_subtree_instead_of_crossing_singularity() {
    let mut w = fixture();
    let mut p = load_pose(&w);
    w.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            for (row, n) in chunk.ids.iter().enumerate() {
                if *n == id(1) {
                    chunk.transforms[row * 10 + 7] = -2.;
                }
            }
        })
    });
    w.step().unwrap();
    sync(&w, &mut p, 0);
    for alpha in [0., 0.25, 0.5, 0.75, 1.] {
        assert_eq!(point(&mut p, alpha).0, DVec3::new(-4., 0., 0.));
    }
}
