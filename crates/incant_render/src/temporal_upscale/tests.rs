use super::*;
use glam::{Mat4, Vec2, Vec3};
fn uv(camera: crate::camera::CameraView, position: Vec3, aspect: f32) -> Vec3 {
    let p = camera.matrix(aspect).unwrap().project_point3(position);
    Vec3::new(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5, p.z)
}
#[test]
fn actual_projected_camera_and_object_motion_unjitter_once_in_output_uv() {
    // Uses the production projection, separately moving object and camera. Both
    // axes, perspective/orthographic and odd dimensions distinguish the sign.
    for ortho in [false, true] {
        for size in [[720., 405.], [65., 33.]] {
            let rect = [0., 0., size[0], size[1]];
            let mut previous = crate::camera::CameraView::preview();
            if ortho {
                previous.orthographic_half_height = Some(4.);
            }
            for (camera_delta, object_delta) in [
                (Vec3::ZERO, Vec3::new(0.2, 0.1, 0.3)),
                (Vec3::new(0.1, -0.2, 0.3), Vec3::ZERO),
            ] {
                let mut current = previous;
                current.view = previous.view * Mat4::from_translation(-camera_delta);
                let point = Vec3::new(0.3, 0.1, -0.2);
                let old = uv(previous, point, size[0] / size[1]);
                let now = uv(current, point + object_delta, size[0] / size[1]);
                for phase in 0..8 {
                    let prior = (phase + 7) % 8;
                    let mut a = previous;
                    let mut b = current;
                    a.jitter = crate::temporal::jitter::ndc(prior, rect);
                    b.jitter = crate::temporal::jitter::ndc(phase, rect);
                    let old_jittered = uv(a, point, size[0] / size[1]);
                    let new_jittered = uv(b, point + object_delta, size[0] / size[1]);
                    let motion = old_jittered.truncate() - new_jittered.truncate();
                    let jitter_now = Vec2::from(crate::temporal::jitter::SAMPLES[phase as usize])
                        / Vec2::from(size);
                    let jitter_old = Vec2::from(crate::temporal::jitter::SAMPLES[prior as usize])
                        / Vec2::from(size);
                    let corrected = motion + jitter_now - jitter_old;
                    assert!((now.truncate() + corrected - old.truncate()).length() < 2e-6);
                    assert!(
                        (new_jittered.truncate() - now.truncate() - jitter_now).length() < 2e-6
                    );
                    assert!((old_jittered.z - old.z).abs() < 1e-6);
                    assert!(
                        (old.z - now.z).abs() > 1e-5,
                        "previous depth cannot replace current depth"
                    );
                }
            }
        }
    }
}
#[test]
fn source_shader_validation_and_distinct_center_material_contract() {
    for source in [
        system::center_source(),
        include_str!("resolve.wgsl").to_string(),
        super::raster_readback::COPY_SHADER.to_string(),
    ] {
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
    let old = include_str!("../model_material.wgsl");
    assert_eq!(
        old.matches("@interpolate(perspective, centroid)").count(),
        4
    );
    assert!(system::center_source().ends_with(&old.replace(
        "@interpolate(perspective, centroid)",
        "@interpolate(perspective, center)"
    )));
    // Motion UV is already default perspective/center; both MASK paths sample
    // the same base texture before the same alpha predicate.
    let motion = include_str!("../motion/shader.wgsl");
    assert!(motion.contains("@location(1) uv:vec2f"));
    for s in [old, motion] {
        assert!(
            s.find("textureSample(base_map,base_sampler,in.uv)")
                .unwrap()
                < s.find("discard;").unwrap()
        );
        assert!(s.contains("base.a<material.factors.w"));
    }
}
#[test]
fn history_capacity_is_checked_before_extent_creation() {
    assert_eq!(owner::bytes([960, 540], 8192).unwrap(), 16_588_800);
    assert_eq!(owner::bytes([2048, 1024], 8192).unwrap(), MAX_HISTORY_BYTES);
    for size in [[0, 1], [u32::MAX, u32::MAX], [2048, 1025], [8193, 1]] {
        assert!(owner::bytes(size, 8192).is_err());
    }
    assert!(owner::bytes([65, 33], 64).is_err());
}
#[test]
fn reconstruction_kernels_preserve_constant_and_signed_history_support() {
    fn cubic(x: f64) -> f64 {
        let a = x.abs();
        if a < 1. {
            1.5 * a * a * a - 2.5 * a * a + 1.
        } else if a < 2. {
            -0.5 * a * a * a + 2.5 * a * a - 4. * a + 2.
        } else {
            0.
        }
    }
    for f in [0., 0.125, 0.5, 0.875, 0.999] {
        let weights = (-1..=2)
            .map(|i| cubic(f - f64::from(i)))
            .collect::<Vec<_>>();
        assert!((weights.iter().sum::<f64>() - 1.).abs() < 1e-12);
        if f > 0. {
            assert!(weights[0] < 0.);
            assert!(weights[3] < 0.);
        }
        let constant = weights.iter().map(|w| w * 0.37).sum::<f64>();
        assert!((constant - 0.37).abs() < 1e-12);
    }
}
