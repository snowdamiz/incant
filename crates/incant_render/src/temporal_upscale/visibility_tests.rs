//! One bounded scene; visibility is recorded, never used to repair radiance.
use super::{
    raster_geometry as geometry,
    raster_readback::{Pixels, Reader},
    raster_tests::{owner, render, write},
    visibility_oracle::{self as oracle, OCCLUDER, Pose, Scene},
};
use crate::{
    Renderer, RetainedScene, RetainedTarget, SceneInput,
    motion::visibility::{Probe, Sample},
};
use glam::{DMat4, DVec3};
use serde_json::json;
use std::{path::Path, sync::Arc};

fn save(root: &Path, name: &str, low: &Pixels, samples: &[Sample]) {
    let raw: Vec<_> = low
        .0
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    write(root, &format!("{name}-low.f32le"), &raw);
    save_samples(root, name, samples);
}
fn save_samples(root: &Path, name: &str, samples: &[Sample]) {
    let raw: Vec<_> = samples
        .iter()
        .flat_map(|s| {
            s.motion
                .iter()
                .chain(s.position_depth.iter())
                .flat_map(|v| v.to_le_bytes())
                .chain(s.owner.iter().flat_map(|v| v.to_le_bytes()))
        })
        .collect();
    write(root, &format!("{name}-visibility.bin"), &raw);
}
fn prepare(r: &Renderer, input: &SceneInput, probe: &Arc<Probe>) -> RetainedScene {
    let mut s = owner(r, input);
    s.set_visibility_test_probe(Arc::clone(probe));
    s
}
fn apply(input: &mut SceneInput, pose: Pose) {
    input.nodes[0].world = DMat4::from_translation(DVec3::new(0., pose.mesh_y, 0.));
    input.nodes[1].world = DMat4::from_translation(DVec3::new(0., pose.camera_y, 10.));
    input.nodes[2].world =
        DMat4::from_translation(DVec3::new(0., pose.occluder_y, pose.occluder_z));
}
fn report(
    root: &Path,
    name: &str,
    retained: &RetainedScene,
    samples: &[Sample],
    pattern: [[f64; 2]; 4],
    current: Pose,
    previous: Pose,
) {
    let frame = retained.motion_frame().unwrap();
    let changes = retained.motion_changes().unwrap();
    assert!(changes.frame().same_owner(&frame));
    assert_eq!(changes.frame().epoch(), frame.epoch());
    assert_eq!(changes.frame().sequence(), frame.sequence());
    let ids = [
        retained.visibility_test_owner(0),
        retained.visibility_test_owner(2),
    ];
    let words = ids.map(|id| id.map_or(0, |id| oracle::word(id, changes.changed(id).unwrap())));
    for (id, moved) in ids.into_iter().zip([
        current.mesh_y != previous.mesh_y,
        current.occluder_y != previous.occluder_y || current.occluder_z != previous.occluder_z,
    ]) {
        if let Some(id) = id {
            assert_eq!(changes.changed(id), Some(!frame.has_previous() || moved));
        }
    }
    let result = oracle::inspect(
        samples,
        pattern,
        &Scene {
            current,
            previous,
            words,
            has_previous: frame.has_previous(),
        },
    );
    write(
        root,
        &format!("{name}.json"),
        &serde_json::to_vec_pretty(&json!({
        "phase":frame.jitter_phase(),"sequence":frame.sequence(),"epoch":frame.epoch(),
        "has_previous":frame.has_previous(),"owner_words":words,"oracle":result}))
        .unwrap(),
    );
    oracle::assert_inspection(&result);
}

#[test]
#[ignore = "coordinated GPU only: 64² 4x visibility; no color resolver/appearance/timing acceptance"]
fn output_four_sample_visibility_preserves_actual_owner_depth_and_previous_correspondence() {
    let root = std::path::PathBuf::from(std::env::var_os("INCANT_TAAU_VISIBILITY_OUTPUT").unwrap());
    std::fs::create_dir(&root).unwrap();
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    pollster::block_on(r.enable_temporal_upscale()).unwrap();
    let output = crate::hdr_output::texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, 64, 64);
    let view = output.create_view(&Default::default());
    let reader = Reader::new(&r);
    let probe = Arc::new(Probe::new(&r));
    let calibration = probe.calibrate(&r);
    save_samples(&root, "calibration", &calibration);
    let pattern = oracle::calibration(&calibration).unwrap();
    write(&root,"fixture.json",&serde_json::to_vec_pretty(&json!({
        "input":[48,48],"output":[64,64],"samples":4,"measured_sample_offsets":pattern,
        "adapter":format!("{:?}",r.adapter.get_info()),"edge":geometry::EDGE,"strip":geometry::STRIP,"occluder":OCCLUDER,
        "visibility_layout":"pixel-major then sample; 48 bytes: motion4 f32, sample_position.xy/current_depth/sample_index f32, owner_word+3zero u32; little-endian",
        "attachment_encoding":"lossless u32/f32 split into low/high u16 lanes in 3xRGBA16Uint + RG16Uint; Depth32Float remains native",
        "geometry_tie_distance_pixels":1e-5,"position_epsilon_pixels":2e-5,"depth_motion_epsilon":2e-6,
        "motion":"previous UV minus current UV, both unjittered output; z expected previous depth, w validity",
        "visibility_only":true,"old_failed_resolver_unchanged":true})).unwrap());
    let mut input = geometry::fixture();
    let mut occluder = geometry::fixture_with(&[OCCLUDER], [0.; 3], 1)
        .nodes
        .remove(0);
    occluder.id = "occluder".into();
    input.nodes.push(occluder);
    let initial = Pose::initial();
    apply(&mut input, initial);
    let mut retained = prepare(&r, &input, &probe);
    let first = render(&r, &mut retained, &input, &view, &reader);
    let first_samples = probe.read(&r);
    save(&root, "phase-0", &first, &first_samples);
    report(
        &root,
        "phase-0",
        &retained,
        &first_samples,
        pattern,
        initial,
        initial,
    );
    let original = retained.motion_frame().unwrap();
    assert!(!original.has_previous());
    assert_eq!(original.jitter_phase(), Some(0));
    let metrics =
        oracle::static_metrics(&first_samples, retained.visibility_test_owner(0).unwrap());
    write(
        &root,
        "static-area.json",
        &serde_json::to_vec_pretty(&metrics).unwrap(),
    );
    assert!(metrics["invented_halo_rows"].as_array().unwrap().is_empty());

    // The rejected candidate must not publish geometry, identity, history or
    // visibility. The next accepted result is compared to an untouched owner.
    apply(
        &mut input,
        Pose {
            mesh_y: 1.,
            occluder_y: 0.,
            ..initial
        },
    );
    let error = retained
        .submit(
            &r,
            &input,
            RetainedTarget {
                view: &view,
                format: wgpu::TextureFormat::R32Float,
                width: 64,
                height: 64,
                viewport: None,
            },
        )
        .unwrap_err();
    write(&root, "rejected.txt", error.to_string().as_bytes());
    assert!(matches!(
        error.downcast_ref::<crate::ResourceError>(),
        Some(crate::ResourceError::OutputFormat(
            wgpu::TextureFormat::R32Float
        ))
    ));
    let after = retained.motion_frame().unwrap();
    assert!(after.same_owner(&original));
    assert_eq!(after.epoch(), original.epoch());
    assert_eq!(after.sequence(), original.sequence());
    let [motion, depth, history] = retained.temporal_upscale_test_views();
    let color = r
        .frames
        .get(&r.device, 48, 48, crate::SceneMsaa::One)
        .unwrap();
    let rejected = reader.read(&r, &color.color, &motion, &depth, &history);
    let rejected_samples = probe.read(&r);
    save(&root, "rejected", &rejected, &rejected_samples);
    assert_eq!(rejected.0, first.0);
    assert_eq!(rejected_samples, first_samples);
    apply(&mut input, initial);
    let next = render(&r, &mut retained, &input, &view, &reader);
    let next_samples = probe.read(&r);
    save(&root, "phase-1", &next, &next_samples);
    report(
        &root,
        "phase-1",
        &retained,
        &next_samples,
        pattern,
        initial,
        initial,
    );
    let mut fresh = prepare(&r, &input, &probe);
    render(&r, &mut fresh, &input, &view, &reader);
    let reference = render(&r, &mut fresh, &input, &view, &reader);
    let reference_samples = probe.read(&r);
    save(&root, "rejected-reference", &reference, &reference_samples);
    assert_eq!(next.0, reference.0);
    assert_eq!(next_samples, reference_samples);
    assert!(!fresh.motion_frame().unwrap().same_owner(&original));
    for phase in 2..8 {
        let low = render(&r, &mut retained, &input, &view, &reader);
        let samples = probe.read(&r);
        // All original low-resolution planes are preserved, independent of the
        // additional visibility pass. Keep only three full static 4x captures.
        let raw: Vec<_> = low
            .0
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        write(&root, &format!("phase-{phase}-low.f32le"), &raw);
        if phase == 7 {
            save_samples(&root, "phase-7", &samples);
        }
        report(
            &root,
            &format!("phase-{phase}"),
            &retained,
            &samples,
            pattern,
            initial,
            initial,
        );
        assert_eq!(retained.motion_frame().unwrap().jitter_phase(), Some(phase));
        assert_eq!(
            samples, next_samples,
            "unjittered static visibility must not cycle with low-res jitter"
        );
    }
    let mut previous = initial;
    for (name, current) in [
        (
            "moving-strip",
            Pose {
                mesh_y: 0.125,
                ..initial
            },
        ),
        (
            "moving-camera",
            Pose {
                mesh_y: 0.125,
                camera_y: -0.125,
                ..initial
            },
        ),
        (
            "occluder",
            Pose {
                occluder_y: 0.,
                ..initial
            },
        ),
        (
            "occluder-depth",
            Pose {
                occluder_y: 0.,
                occluder_z: 2.,
                ..initial
            },
        ),
        (
            "cut",
            Pose {
                occluder_y: 0.,
                occluder_z: 2.,
                ..initial
            },
        ),
    ] {
        apply(&mut input, current);
        let before = retained.motion_frame().unwrap();
        if name == "cut" {
            retained.invalidate_motion_history();
        }
        let low = render(&r, &mut retained, &input, &view, &reader);
        let samples = probe.read(&r);
        save(&root, name, &low, &samples);
        report(&root, name, &retained, &samples, pattern, current, previous);
        let after = retained.motion_frame().unwrap();
        assert!(after.same_owner(&before));
        assert_eq!(after.sequence(), before.sequence() + 1);
        if name == "cut" {
            assert!(!after.has_previous());
            assert!(after.epoch() > before.epoch());
            assert_eq!(after.jitter_phase(), Some(0));
            let mut fresh = prepare(&r, &input, &probe);
            let reference = render(&r, &mut fresh, &input, &view, &reader);
            let reference_samples = probe.read(&r);
            save(&root, "cut-reference", &reference, &reference_samples);
            assert_eq!(samples, reference_samples);
            assert_eq!(low.0, reference.0);
            assert!(!after.same_owner(&fresh.motion_frame().unwrap()));
        } else {
            assert_eq!(after.epoch(), before.epoch());
            assert!(after.has_previous());
        }
        previous = current;
    }
    let old = retained.motion_frame().unwrap();
    input.nodes[0].mesh = None;
    apply(&mut input, initial);
    assert!(
        retained
            .submit(
                &r,
                &input,
                RetainedTarget {
                    view: &view,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    width: 64,
                    height: 64,
                    viewport: None
                }
            )
            .is_err()
    );
    assert_eq!(retained.motion_frame().unwrap().sequence(), old.sequence());
    retained = prepare(&r, &input, &probe);
    let low = render(&r, &mut retained, &input, &view, &reader);
    let samples = probe.read(&r);
    save(&root, "removal", &low, &samples);
    report(
        &root,
        "removal",
        &retained,
        &samples,
        pattern,
        Pose {
            removed: true,
            ..initial
        },
        initial,
    );
    assert!(!old.same_owner(&retained.motion_frame().unwrap()));
    assert!(samples.iter().all(|s| s.owner[0] == 0));
    write(&root,"result.json",&serde_json::to_vec_pretty(&json!({"passed":true,"calibration_draws":1,"accepted_scene_draws":17,
        "rejected_submissions":2,"original_static_lowres_phases":8,"halo_rows":148,"strip_area_pixels":49,
        "missed_slivers_preserved":true,"moving_radiance_reconstruction":false,"appearance_or_performance_accepted":false})).unwrap());
}
