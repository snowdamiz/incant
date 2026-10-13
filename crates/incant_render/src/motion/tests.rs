use super::{
    MotionError,
    owner::{Extent, History, reproject},
    plan,
};
use crate::camera::CameraView;
use glam::{Mat4, Vec3, Vec4};
fn extent(size: [u32; 2]) -> Extent {
    Extent::new(
        size,
        size,
        [0., 0., size[0] as f32, size[1] as f32],
        CameraView::preview(),
    )
}
#[test]
fn history_advances_only_on_accept_and_cuts_survive_failed_preparation() {
    let mut h = History::default();
    let a = h.prepare(extent([64, 64]), Mat4::IDENTITY).unwrap();
    assert_eq!(a.frame.sequence(), 1);
    assert!(!a.frame.has_previous());
    drop(a);
    assert!(h.frame().is_none());
    h.accept(h.prepare(extent([64, 64]), Mat4::IDENTITY).unwrap());
    assert!(
        h.prepare(extent([64, 64]), Mat4::IDENTITY)
            .unwrap()
            .frame
            .has_previous()
    );
    h.invalidate();
    assert!(matches!(
        h.prepare(extent([64, 64]), Mat4::ZERO),
        Err(MotionError::Projection)
    ));
    assert_eq!(h.frame().unwrap().sequence(), 1);
    let a = h.prepare(extent([64, 64]), Mat4::IDENTITY).unwrap();
    assert!(!a.frame.has_previous());
    h.accept(a);
    assert_eq!(h.frame().unwrap().sequence(), 2);
    assert!(
        h.prepare(extent([64, 64]), Mat4::IDENTITY)
            .unwrap()
            .frame
            .has_previous()
    );
}
#[test]
fn fresh_owner_resize_projection_viewport_and_pixel_fence_previous() {
    let mut h = History::default();
    h.accept(h.prepare(extent([65, 33]), Mat4::IDENTITY).unwrap());
    let old = h.frame().unwrap();
    let mut other = History::default();
    other.accept(other.prepare(extent([65, 33]), Mat4::IDENTITY).unwrap());
    assert!(!old.same_owner(&other.frame().unwrap()));
    assert!(
        !h.prepare(extent([66, 33]), Mat4::IDENTITY)
            .unwrap()
            .frame
            .has_previous()
    );
    let mut c = CameraView::preview();
    c.fov += 0.1;
    assert!(
        !h.prepare(
            Extent::new([65, 33], [65, 33], [0., 0., 65., 33.], c),
            Mat4::IDENTITY
        )
        .unwrap()
        .frame
        .has_previous()
    );
    assert!(
        !h.prepare(
            Extent::new(
                [65, 33],
                [65, 33],
                [1., 0., 64., 33.],
                CameraView::preview()
            ),
            Mat4::IDENTITY
        )
        .unwrap()
        .frame
        .has_previous()
    );
    c.pixel_resolution = Some([32, 16]);
    let pixel = h
        .prepare(
            Extent::new([65, 33], [32, 16], [0., 0., 32., 16.], c),
            Mat4::IDENTITY,
        )
        .unwrap();
    assert!(pixel.frame.bypassed());
    assert!(!pixel.frame.has_previous());
    h.accept(pixel);
    assert!(
        !h.prepare(extent([65, 33]), Mat4::IDENTITY)
            .unwrap()
            .frame
            .has_previous()
    );
}
#[test]
fn repeated_simulation_state_still_has_distinct_submitted_sequence() {
    let mut h = History::default();
    for sequence in 1..=4 {
        h.accept(h.prepare(extent([65, 33]), Mat4::IDENTITY).unwrap());
        assert_eq!(h.frame().unwrap().sequence(), sequence);
    }
    assert!(h.frame().unwrap().has_previous());
}
#[test]
fn camera_and_object_motion_use_previous_minus_current_top_left_uv() {
    let project = Mat4::perspective_rh(90f32.to_radians(), 1., 0.1, 100.);
    let old = project * Vec4::new(0., 0., -2., 1.);
    let current = project * Vec4::new(0.5, 0.25, -2., 1.);
    let got = reproject(current, old).unwrap();
    assert!((got[0] + 0.125).abs() < 1e-6);
    assert!((got[1] - 0.0625).abs() < 1e-6);
    let camera = project * Mat4::from_translation(Vec3::new(-0.5, 0., 0.));
    assert!((reproject(camera * Vec4::new(0., 0., -2., 1.), old).unwrap()[0] - 0.125).abs() < 1e-6);
    let ortho = Mat4::orthographic_rh(-2., 2., -1., 1., 0.1, 10.);
    let a = ortho * Vec4::new(0., 0., -2., 1.);
    let b = ortho * Vec4::new(1., 0.5, -2., 1.);
    let got = reproject(b, a).unwrap();
    assert!((got[0] + 0.25).abs() < 1e-6);
    assert!((got[1] - 0.25).abs() < 1e-6);
    // Different local positions: a skinned/IK deformation cannot be replaced by
    // applying two world matrices to the same rest position.
    assert_ne!(reproject(current, old), reproject(current, current));
}
#[test]
fn invalid_or_offscreen_previous_projection_never_fabricates_zero_motion() {
    let p = Vec4::new(0., 0., 0.5, 1.);
    for old in [
        Vec4::NAN,
        Vec4::ZERO,
        Vec4::new(0., 0., 0.5, -1.),
        Vec4::new(2., 0., 0.5, 1.),
        Vec4::new(1., 0., 0.5, 1.),
        Vec4::new(0., -1., 0.5, 1.),
        Vec4::new(0., 0., 1.1, 1.),
    ] {
        assert!(reproject(p, old).is_none());
    }
    assert_eq!(reproject(p, p), Some([0., 0., 0.5, 1.]));
}
#[test]
fn aggregate_target_and_cpu_gpu_payload_admission_is_checked() {
    assert_eq!(
        plan::target_bytes([2048, 2048], 8192).unwrap(),
        96 * 1024 * 1024
    );
    for extent in [[0, 1], [8193, 1], [u32::MAX, u32::MAX], [2049, 2048]] {
        assert_eq!(plan::target_bytes(extent, 8192), Err(MotionError::Capacity));
    }
    let cap = plan::MAX_MOTION_TARGET_BYTES;
    assert!(plan::overlap(cap, cap, cap).is_ok());
    assert!(plan::overlap(cap + 1, 1, cap).is_err());
    assert!(plan::overlap(0, 0, u64::MAX).is_err());
    let mut n = 0;
    assert!(plan::charge(&mut n, usize::MAX, 160).is_err());
}
#[test]
fn portable_motion_shader_validates_and_uses_canonical_mask_contract() {
    let source = include_str!("shader.wgsl");
    let module = naga::front::wgsl::parse_str(source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    let canonical = include_str!("../model_material.wgsl");
    for line in [
        "struct Material { base:vec4f, emissive_roughness:vec4f, factors:vec4f, flags:vec4u }",
        "let base=material.base*textureSample(base_map,base_sampler,in.uv);",
        "if material.flags.x==1u && base.a<material.factors.w { discard; }",
    ] {
        assert!(canonical.contains(line));
        assert!(source.contains(line));
    }
}
#[test]
fn native_cooked_features_do_not_silently_create_uncooked_motion() {
    let cooked = cfg!(all(not(target_arch = "wasm32"), feature = "cooked-output"));
    assert_eq!(
        super::admission(),
        if cooked {
            Err(MotionError::Cooked)
        } else {
            Ok(())
        }
    );
}

#[test]
fn drs_and_offset_viewports_share_exact_internal_coordinates_and_reset_history() {
    use crate::resolution::{FramePlan, ResolutionScale};
    let output = [101, 67];
    let rect = [3., 5., 91., 57.];
    let full = FramePlan::new(output, rect, None, ResolutionScale::FULL).unwrap();
    let scale = ResolutionScale::new(8).unwrap();
    let half = FramePlan::new(output, rect, None, scale).unwrap();
    assert_eq!(half.size, [51, 34]);
    assert_eq!(half.rect, [1.5, 2.5, 45.5, 28.5]);
    assert_eq!(full.rect[2] / full.rect[3], half.rect[2] / half.rect[3]);
    let mut history = History::default();
    history.accept(
        history
            .prepare(
                Extent::new(output, full.size, full.rect, CameraView::preview()),
                Mat4::IDENTITY,
            )
            .unwrap(),
    );
    let candidate = history
        .prepare(
            Extent::new(output, half.size, half.rect, CameraView::preview()),
            Mat4::IDENTITY,
        )
        .unwrap();
    assert!(!candidate.frame.has_previous());
    assert_eq!(history.frame().unwrap().internal_size(), output);
    history.accept(candidate);
    assert_eq!(history.frame().unwrap().internal_size(), half.size);
    assert!(
        history
            .prepare(
                Extent::new(output, half.size, half.rect, CameraView::preview()),
                Mat4::IDENTITY
            )
            .unwrap()
            .frame
            .has_previous()
    );
}
