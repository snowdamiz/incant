//! Real retained GPU route; analytic geometry and known emitted radiance form
//! the oracle. No CPU color/visibility decision feeds the next submitted frame.
use super::{
    raster_geometry as geometry,
    raster_tests::write,
    visibility_oracle::{self as oracle, Pose, Scene},
    visibility_tests::apply,
};
use crate::{Renderer, RetainedScene, RetainedTarget, SceneInput, motion::visibility::Probe};
use serde_json::json;
#[path = "visibility_reconstruction_fixture.rs"]
mod fixture;
use fixture::scene;
#[path = "visibility_reconstruction_readback.rs"]
mod readback;
use readback::{Pixel, Reader};

#[test]
fn visibility_reconstruction_shader_and_capacity_validate() {
    let module = naga::front::wgsl::parse_str(&super::visibility_reconstruction::source()).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
    assert_eq!(
        super::visibility_reconstruction::bytes([64, 64]).unwrap(),
        753664
    );
    for size in [[0, 64], [64, 0], [129, 64], [u32::MAX, u32::MAX]] {
        assert!(super::visibility_reconstruction::bytes(size).is_err());
    }
}
fn retained(r: &Renderer, input: &SceneInput, size: [u32; 2]) -> RetainedScene {
    let mut s = super::raster_tests::owner(r, input);
    s.prepare_visibility_reconstruction(r, size).unwrap();
    s
}
fn submit(
    r: &Renderer,
    s: &mut RetainedScene,
    input: &SceneInput,
    view: &wgpu::TextureView,
    size: [u32; 2],
) {
    s.submit(
        r,
        input,
        RetainedTarget {
            view,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            width: size[0],
            height: size[1],
            viewport: None,
        },
    )
    .unwrap();
}
fn metrics(
    pixels: &[Pixel],
    samples: &[crate::motion::visibility::Sample],
    ids: [u32; 2],
) -> serde_json::Value {
    assert_eq!(pixels.len(), 64 * 64);
    assert_eq!(samples.len(), 64 * 64 * 4);
    let mut missing = Vec::new();
    let mut area_error = 0.;
    let mut rgb_error = 0.;
    let mut mixed = 0;
    for (i, (pixel, raw)) in pixels.iter().zip(samples.as_chunks::<4>().0).enumerate() {
        assert!(
            pixel
                .color
                .iter()
                .chain(&pixel.support)
                .all(|v| v.is_finite())
        );
        let counts = ids.map(|id| raw.iter().filter(|r| r.owner[0] & 0x7fffffff == id).count());
        let covered = counts.iter().sum::<usize>();
        let a = covered as f32 / 4.;
        assert!(
            (0. ..=a).contains(&pixel.color[3]),
            "invented coverage at {i}"
        );
        if covered == 0 {
            assert_eq!(pixel.color, [0.; 4], "vacated/halo pixel {i}");
            assert_eq!(pixel.owner, 0);
        }
        if counts.iter().all(|n| *n > 0) {
            mixed += 1;
            assert_eq!(
                pixel.owner, 0,
                "mixed-owner composite must not seed history"
            );
        }
        if pixel.owner != 0 {
            assert!(ids.contains(&pixel.owner));
            assert_eq!(pixel.color[3], a);
            assert!((1. ..=8.).contains(&pixel.support[1]));
        }
        let expected = [a, counts[0] as f32 / 4., counts[0] as f32 / 4., a];
        // Every represented sample has known white/red radiance. Missing color
        // is reported, but cannot justify a different owner's excess color.
        let missing_alpha = a - pixel.color[3];
        for (actual, expected) in pixel.color[..3].iter().zip(expected) {
            assert!(*actual >= 0. && *actual <= expected + 0.001);
            assert!(
                (expected - *actual).abs() <= missing_alpha + 0.001,
                "wrong radiance at {i}: actual={actual} expected={expected}"
            );
        }
        area_error += f64::from((pixel.color[3] - a).abs());
        rgb_error += pixel.color[..3]
            .iter()
            .zip(&expected)
            .map(|(v, e)| f64::from((v - e).abs()))
            .sum::<f64>();
        if pixel.color[3] < a {
            missing.push([i % 64, i / 64]);
        }
    }
    // Actual4x is not exact analytic area: retain the original49 strip pixels,
    // including boundary slivers no sample observes. Never hide them in a count.
    let strip:Vec<_>=(6..55).map(|x|json!({"x":x,"analytic_area":geometry::area(&geometry::STRIP,x,44),"alpha":pixels[44*64+x].color[3]})).collect();
    json!({"missing_current_visible_radiance":missing,"sample_area_error":area_error,"sample_RGB_error":rgb_error,"mixed_owner_pixels":mixed,"analytic_strip":strip})
}
#[test]
fn reconstruction_oracle_rejects_invented_coverage_and_mixed_owner_seed() {
    let clear = Pixel {
        color: [0.; 4],
        support: [1., 0.],
        owner: 0,
        reserved: 0,
    };
    let raw = crate::motion::visibility::Sample {
        motion: [0.; 4],
        position_depth: [0., 0., 1., 0.],
        owner: [0; 4],
    };
    let mut pixels = vec![clear; 4096];
    let mut samples = vec![raw; 16384];
    metrics(&pixels, &samples, [1, 2]);
    pixels[0].color = [0.25; 4];
    assert!(std::panic::catch_unwind(|| metrics(&pixels, &samples, [1, 2])).is_err());
    pixels[0].color = [0.5, 0.25, 0.25, 0.5];
    pixels[0].owner = 1;
    pixels[0].support = [0.2, 8.];
    samples[0].owner[0] = 1;
    samples[1].owner[0] = 2;
    assert!(std::panic::catch_unwind(|| metrics(&pixels, &samples, [1, 2])).is_err());
}

#[test]
#[ignore = "coordinated offscreen buffers only: visibility-admitted radiance; no OS screen/window access"]
fn visibility_admitted_reconstruction_motion_reversal_depth_resize_and_refusal() {
    let root = std::path::PathBuf::from(
        std::env::var_os("INCANT_VISIBILITY_RECONSTRUCTION_OUTPUT").unwrap(),
    );
    std::fs::create_dir(&root).unwrap();
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    pollster::block_on(r.enable_temporal_upscale()).unwrap();
    pollster::block_on(r.enable_motion_owner()).unwrap();
    let pattern = oracle::calibration(&Probe::new(&r).calibrate(&r)).unwrap();
    let reader = Reader::new(&r);
    let size = [64, 64];
    let output = crate::hdr_output::texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, 64, 64);
    let view = output.create_view(&Default::default());
    let mut input = scene();
    let initial = Pose::initial();
    apply(&mut input, initial);
    let mut s = retained(&r, &input, size);
    let mut poses = vec![initial; 16];
    poses.extend([
        Pose {
            mesh_y: 0.125,
            ..initial
        },
        Pose {
            mesh_y: 0.25,
            ..initial
        },
        Pose {
            mesh_y: 0.125,
            ..initial
        },
        initial,
        Pose {
            camera_y: 0.125,
            ..initial
        },
        Pose {
            camera_y: -0.125,
            ..initial
        },
        initial,
        Pose {
            occluder_y: 0.,
            ..initial
        },
        Pose {
            occluder_y: 0.,
            occluder_z: 2.,
            ..initial
        },
        Pose {
            occluder_y: 0.,
            occluder_z: 9.8,
            ..initial
        },
        initial,
        Pose {
            mesh_y: 6.,
            ..initial
        },
        initial,
    ]);
    let mut previous = initial;
    let mut summaries = Vec::new();
    for (index, current) in poses.into_iter().enumerate() {
        apply(&mut input, current);
        submit(&r, &mut s, &input, &view, size);
        let samples = s.visibility_reconstruction_probe().read_fixture(&r);
        let pixels = reader.read(&r, &s);
        write(
            &root,
            &format!("frame-{index:02}.bin"),
            bytemuck::cast_slice(&pixels),
        );
        super::visibility_tests::save_samples(&root, &format!("frame-{index:02}"), &samples);
        write(&root,&format!("frame-{index:02}.json"),&serde_json::to_vec_pretty(&json!({"frame":index,"pose":[current.mesh_y,current.camera_y,current.occluder_y,current.occluder_z],"previous":[previous.mesh_y,previous.camera_y,previous.occluder_y,previous.occluder_z],"acceptance_checks_complete":false})).unwrap());
        let changes = s.motion_changes().unwrap();
        let frame = s.motion_frame().unwrap();
        let ids = [
            s.visibility_test_owner(0).unwrap(),
            s.visibility_test_owner(2).unwrap(),
        ];
        let words = ids.map(|id| oracle::word(id, changes.changed(id).unwrap()));
        oracle::assert_inspection(&oracle::inspect(
            &samples,
            pattern,
            &Scene {
                current,
                previous,
                words,
                has_previous: frame.has_previous(),
            },
        ));
        assert_eq!(frame.sequence(), index as u64 + 1);
        assert_eq!(frame.epoch(), 1);
        let m = metrics(&pixels, &samples, ids);
        // After a complete observed cycle, this constant-radiance static fixture
        // must reproduce actual4x sample coverage and RGB. Analytic slivers remain.
        if (8..16).contains(&index) {
            assert_eq!(m["sample_area_error"], 0.);
            assert_eq!(m["sample_RGB_error"], 0.);
        }
        summaries.push(json!({"frame":index,"sequence":frame.sequence(),"epoch":frame.epoch(),"pose":[current.mesh_y,current.camera_y,current.occluder_y,current.occluder_z],"metrics":m}));
        previous = current;
    }
    // An invalid output must not publish any accepted image/owner/support or
    // sequence; the next accepted frame must equal an untouched same-sequence run.
    apply(&mut input, initial);
    let mut reference = retained(&r, &input, size);
    for _ in 0..8 {
        submit(&r, &mut reference, &input, &view, size);
    }
    let before = reader.read(&r, &reference);
    let old = reference.motion_frame().unwrap();
    let error = reference
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
    assert!(matches!(
        error.downcast_ref::<crate::ResourceError>(),
        Some(crate::ResourceError::OutputFormat(
            wgpu::TextureFormat::R32Float
        ))
    ));
    assert_eq!(reader.read(&r, &reference), before);
    assert_eq!(reference.motion_frame().unwrap().sequence(), old.sequence());
    submit(&r, &mut reference, &input, &view, size);
    let after = reader.read(&r, &reference);
    let mut untouched = retained(&r, &input, size);
    for _ in 0..9 {
        submit(&r, &mut untouched, &input, &view, size);
    }
    assert_eq!(after, reader.read(&r, &untouched));
    reference.invalidate_motion_history();
    submit(&r, &mut reference, &input, &view, size);
    let mut fresh = retained(&r, &input, size);
    submit(&r, &mut fresh, &input, &view, size);
    assert_eq!(reader.read(&r, &reference), reader.read(&r, &fresh));
    assert!(!reference.motion_frame().unwrap().has_previous());
    // Explicit resize owns new geometry/identity/color history; no sampling old
    // input rectangles. Compare accepted pixels with a freshly sized owner.
    for size in [[80, 64], [64, 64]] {
        let texture =
            crate::hdr_output::texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, size[0], size[1]);
        let view = texture.create_view(&Default::default());
        reference
            .prepare_visibility_reconstruction(&r, size)
            .unwrap();
        submit(&r, &mut reference, &input, &view, size);
        let mut fresh = retained(&r, &input, size);
        submit(&r, &mut fresh, &input, &view, size);
        assert_eq!(reader.read(&r, &reference), reader.read(&r, &fresh));
        assert!(!reference.motion_frame().unwrap().has_previous());
    }
    input.nodes[0].mesh = None;
    let mut removed = retained(&r, &input, size);
    submit(&r, &mut removed, &input, &view, size);
    assert!(
        reader
            .read(&r, &removed)
            .iter()
            .all(|p| p.color == [0.; 4] && p.owner == 0)
    );
    write(&root,"result.json",&serde_json::to_vec_pretty(&json!({"frames":summaries,"sample_positions":pattern,"refusal_same_sequence_equal":true,"cut_fresh_equal":true,"resize_fresh_equal":true,"removal_clear":true,"bytes_per_output_pixel":184,"bytes_per_input_owner_pixel":4,"source_readback_admits_frames":false,"appearance_accepted":false,"performance_accepted":false})).unwrap());
}
