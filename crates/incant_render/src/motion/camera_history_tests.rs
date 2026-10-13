use super::owner::{Extent, History};
use crate::camera::CameraView;

fn extent(camera: CameraView, size: [u32; 2]) -> (Extent, [f32; 4]) {
    let rect = [0., 0., size[0] as f32, size[1] as f32];
    (
        Extent::new([64, 64], size, rect, camera).upscaled([0., 0., 64., 64.]),
        rect,
    )
}

#[test]
fn static_camera_ignores_jitter_phase_but_requires_accepted_same_framing() {
    let camera = CameraView::preview();
    let mut history = History::default();
    let (e, rect) = extent(camera, [64, 64]);
    let first = history.prepare_temporal(e.clone(), camera, rect).unwrap();
    assert!(!first.frame.camera_static());
    history.accept(first);
    for _ in 0..16 {
        let next = history.prepare_temporal(e.clone(), camera, rect).unwrap();
        assert!(next.frame.camera_static());
        history.accept(next);
    }
    // The square projection and output stay identical across this DRS change.
    let (reduced, reduced_rect) = extent(camera, [32, 32]);
    let next = history
        .prepare_temporal(reduced.clone(), camera, reduced_rect)
        .unwrap();
    assert!(next.frame.has_previous());
    assert!(!next.frame.camera_static());
    drop(next);
    assert!(
        history
            .prepare_temporal(e, camera, rect)
            .unwrap()
            .frame
            .camera_static()
    );
    history.accept(
        history
            .prepare_temporal(reduced.clone(), camera, reduced_rect)
            .unwrap(),
    );
    assert!(
        history
            .prepare_temporal(reduced, camera, reduced_rect)
            .unwrap()
            .frame
            .camera_static()
    );
}

#[test]
fn camera_proof_checks_shading_eye_separately_and_cuts_invalidate_it() {
    for orthographic in [false, true] {
        let mut camera = CameraView::preview();
        if orthographic {
            camera.orthographic_half_height = Some(2.);
        }
        let (e, rect) = extent(camera, [64, 64]);
        let mut history = History::default();
        history.accept(history.prepare_temporal(e.clone(), camera, rect).unwrap());
        // Deliberately vary the actual shader eye without changing the matrix.
        // This is an internal contract control, not an authored camera mutation.
        let mut changed = camera;
        if orthographic {
            changed.forward.x += 0.25;
        } else {
            changed.eye.x += 0.25;
        }
        assert_eq!(camera.matrix(1.).unwrap(), changed.matrix(1.).unwrap());
        let pending = history.prepare_temporal(e.clone(), changed, rect).unwrap();
        assert!(pending.frame.has_previous());
        assert!(!pending.frame.camera_static());
        drop(pending);
        assert!(
            history
                .prepare_temporal(e.clone(), camera, rect)
                .unwrap()
                .frame
                .camera_static()
        );
        history.invalidate();
        let reset = history.prepare_temporal(e, camera, rect).unwrap();
        assert!(!reset.frame.has_previous());
        assert!(!reset.frame.camera_static());
    }
}

#[test]
fn static_camera_excludes_plain_motion_and_changed_shadow_inputs() {
    let camera = CameraView::preview();
    let (e, rect) = extent(camera, [64, 64]);
    let mut history = History::default();
    history.accept(history.prepare_temporal(e.clone(), camera, rect).unwrap());
    for changed in [
        CameraView {
            forward: camera.forward + glam::Vec3::X * 0.1,
            ..camera
        },
        CameraView {
            jitter_guard: [0.1, 0.1],
            ..camera
        },
    ] {
        assert_eq!(camera.matrix(1.).unwrap(), changed.matrix(1.).unwrap());
        assert!(
            !history
                .prepare_temporal(e.clone(), changed, rect)
                .unwrap()
                .frame
                .camera_static()
        );
    }
    let plain = history
        .prepare(e.clone(), camera.matrix(1.).unwrap())
        .unwrap();
    assert!(plain.frame.has_previous());
    assert!(!plain.frame.camera_static());
    history.accept(plain);
    // Mixing ordinary motion and temporal history still has the pre-existing
    // topology refusal; an absent camera signature must never certify static.
    assert!(matches!(
        history.prepare_temporal(e, camera, rect),
        Err(super::MotionError::Topology)
    ));
}
