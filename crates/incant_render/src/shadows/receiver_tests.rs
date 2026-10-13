use super::*;

fn camera() -> CameraView {
    CameraView::authored(
        &incant_types::Camera {
            fov_degrees: 50.,
            near: 0.1,
            far: 60.,
            projection: Default::default(),
        },
        DMat4::IDENTITY,
    )
    .unwrap()
}

fn bits(cascade: Cascade) -> [u32; 20] {
    let mut values = [0.; 20];
    values[..16].copy_from_slice(&cascade.matrix.to_cols_array());
    values[16..].copy_from_slice(&[
        cascade.far,
        cascade.blend_start,
        cascade.texel_world,
        cascade.inverse_depth,
    ]);
    values.map(f32::to_bits)
}

#[test]
fn shadow_projection_is_identical_with_temporal_off_or_any_phase() {
    let direction = Vec3::new(0.6824, -0.6157, 0.394);
    for ortho in [false, true] {
        let mut camera = camera();
        if ortho {
            camera.orthographic_half_height = Some(3.);
        }
        for rect in [
            [0., 0., 960., 540.],
            [37., 19., 885., 501.],
            [18.5, 9.5, 442.5, 250.5],
            [0., 0., 1., 1.],
        ] {
            let off = fit_for_viewport(camera, rect, direction, 60., &[])
                .unwrap()
                .unwrap()
                .map(bits);
            for phase in 0..8 {
                let mut temporal = camera;
                temporal.jitter = crate::temporal::jitter::ndc(phase, rect);
                temporal.jitter_guard = crate::temporal::jitter::guard(rect);
                let on = fit_for_viewport(temporal, rect, direction, 60., &[])
                    .unwrap()
                    .unwrap()
                    .map(bits);
                assert_eq!(off, on, "ortho={ortho} rect={rect:?} phase={phase}");
            }
        }
    }
}

#[test]
fn odd_viewport_regression_requires_the_guard_before_radius_quantization() {
    let camera = camera();
    let rect = [37., 19., 885., 501.];
    let direction = Vec3::new(0.6824, -0.6157, 0.394);
    let unguarded = fit(camera, rect[2] / rect[3], direction, 60., &[])
        .unwrap()
        .unwrap();
    let guarded = fit_for_viewport(camera, rect, direction, 60., &[])
        .unwrap()
        .unwrap();
    // The original path fits a smaller radius when temporal is off. A MASK
    // caster then samples different alpha texels despite an identical camera.
    assert!(guarded[1].texel_world > unguarded[1].texel_world);
}

#[test]
fn invalid_receiver_viewports_refuse_before_fitting() {
    for rect in [
        [0., 0., 0., 10.],
        [0., 0., 10., -1.],
        [0., 0., f32::INFINITY, 10.],
        [0., f32::NAN, 10., 10.],
    ] {
        assert!(fit_for_viewport(camera(), rect, Vec3::NEG_Z, 60., &[]).is_err());
    }
}
