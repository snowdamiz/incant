//! Actual retained raster registration/coverage; numerical acceptance only.
use super::{
    raster_geometry::{self as geometry, EDGE, STRIP},
    raster_readback::{INPUT, OUTPUT, Pixels, Reader},
};
use crate::{Renderer, RetainedScene, RetainedTarget, SceneInput};
use glam::{DMat4, DVec3};
use serde_json::json;
use std::{io::Write, path::Path};

fn write(root: &Path, name: &str, bytes: &[u8]) {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .unwrap()
        .write_all(bytes)
        .unwrap();
}
fn owner(r: &Renderer, input: &SceneInput) -> RetainedScene {
    let mut s = r.prepare_retained(input, "camera").unwrap();
    let scale = crate::resolution::ResolutionScale::new(12).unwrap();
    s.set_dynamic_resolution(Some(crate::resolution::ResolutionConfig {
        initial: scale,
        minimum: scale,
        maximum: scale,
        target_ms: 16.,
        deadband: 0.1,
        overload_samples: 1,
        recovery_samples: 1,
        cooldown_samples: 0,
        slew: 1,
    }))
    .unwrap();
    s.enable_temporal_upscale(r).unwrap();
    s.prepare_temporal_upscale_extent(r, [64, 64]).unwrap();
    s
}
fn render(
    r: &Renderer,
    s: &mut RetainedScene,
    input: &SceneInput,
    view: &wgpu::TextureView,
    reader: &Reader,
) -> Pixels {
    s.submit(
        r,
        input,
        RetainedTarget {
            view,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            width: 64,
            height: 64,
            viewport: None,
        },
    )
    .unwrap();
    let current = r
        .frames
        .get(&r.device, 48, 48, crate::SceneMsaa::One)
        .unwrap();
    let [motion, depth, history] = s.temporal_upscale_test_views();
    reader.read(r, &current.color, &motion, &depth, &history)
}
fn edge_error(p: &Pixels) -> f64 {
    // Fixed geometry-derived ROI contains the slanted edge, excludes corners
    // and the strip. All pixels in this ROI count, including fully clear ones.
    let mut error = 0.;
    for y in 8..33 {
        for x in 23..38 {
            error +=
                (f64::from(p.history()[y * OUTPUT + x][3]) - geometry::area(&EDGE, x, y)).abs();
        }
    }
    error / (25. * 15.)
}
#[test]
#[ignore = "coordinated GPU only: 20 tiny retained draws/readbacks; no full images or timing"]
fn retained_raster_jitter_coverage_motion_and_cut() {
    let root = std::path::PathBuf::from(
        std::env::var_os("INCANT_TAAU_RASTER_OUTPUT").expect("create-only output path"),
    );
    std::fs::create_dir(&root).unwrap();
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    pollster::block_on(r.enable_temporal_upscale()).unwrap();
    let mut input = geometry::fixture();
    let mut retained = owner(&r, &input);
    let output = crate::hdr_output::texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, 64, 64);
    let view = output.create_view(&Default::default());
    let reader = Reader::new(&r);
    write(&root,"fixture.json",&serde_json::to_vec_pretty(&json!({
        "edge":EDGE,"strip":STRIP,"input":[48,48],"output":[64,64],
        "projection":"orthographic height8 camera z10 near0.1 far50; world x/y = (pixel-32)/8 with inverted Y",
        "raw_layout":"four planes of 4096 vec4 little-endian f32: current, motion, current-depth.x, history. First three have 2304 used entries",
        "tie_distance_output_pixels":1e-5,"adapter":format!("{:?}",r.adapter.get_info()),
        "purpose":"raster registration and coverage; not appearance or timing acceptance"})).unwrap());
    let mut first_error = None;
    let mut ninth_error = None;
    let mut last_jitter = [0.; 2];
    let mut last_shift = 0.;
    let mut cut = None;
    for frame in 1..=20 {
        let object = if frame >= 17 { 1. } else { 0. };
        let camera = if frame >= 18 { 0.5 } else { 0. };
        input.nodes[0].world = DMat4::from_translation(DVec3::new(object, 0., 0.));
        input.nodes[1].world = DMat4::from_translation(DVec3::new(camera, 0., 10.));
        if frame == 19 {
            retained.invalidate_motion_history();
        }
        if frame == 20 {
            retained = owner(&r, &input);
        }
        let p = render(&r, &mut retained, &input, &view, &reader);
        let raw: Vec<_> = p.0.iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
        write(&root, &format!("frame-{frame:02}.f32le"), &raw);
        let motion_frame = retained.motion_frame().unwrap();
        let phase = motion_frame.jitter_phase().unwrap();
        let expected_phase = if frame >= 19 { 0 } else { (frame - 1) % 8 };
        assert_eq!(phase, expected_phase);
        assert_eq!(motion_frame.internal_size(), [48, 48]);
        let previous = frame != 1 && frame < 19;
        assert_eq!(motion_frame.has_previous(), previous);
        let jitter = crate::temporal::jitter::SAMPLES[phase as usize].map(f64::from);
        let shift = (object - camera) * 8.;
        let expected_motion = [
            (last_shift - shift) / 64. + (last_jitter[0] - jitter[0]) / 48.,
            (last_jitter[1] - jitter[1]) / 48.,
        ];
        let mut ties = Vec::new();
        let mut mismatches = Vec::new();
        let mut depth_mismatch = 0;
        let mut motion_mismatch = 0;
        let mut covered = 0;
        let mut strip_samples = 0;
        for y in 0..INPUT {
            for x in 0..INPUT {
                let i = y * INPUT + x;
                // Orthographic projection derived directly from authored screen
                // coordinates. No shader reconstruction/filter code is reused.
                let sample = [
                    (x as f64 + 0.5 - jitter[0]) / 0.75 - shift,
                    (y as f64 + 0.5 - jitter[1]) / 0.75,
                ];
                let edge = geometry::contains(sample, &EDGE);
                let strip = geometry::contains(sample, &STRIP);
                let actual = p.current()[i][3] > 0.;
                if edge.is_none() || strip.is_none() {
                    ties.push([x, y]);
                } else if actual != (edge == Some(true) || strip == Some(true)) {
                    mismatches.push([x, y]);
                }
                if strip == Some(true) && actual {
                    strip_samples += 1;
                }
                if actual {
                    covered += 1;
                }
                if actual != (p.depth()[i][0] < 1.) {
                    depth_mismatch += 1;
                }
                if actual && (f64::from(p.depth()[i][0]) - (10. - 0.1) / (50. - 0.1)).abs() > 2e-6 {
                    depth_mismatch += 1;
                }
                if actual {
                    let m = p.motion()[i];
                    if previous {
                        if m[3] != 1.
                            || (f64::from(m[0]) - expected_motion[0]).abs() > 2e-6
                            || (f64::from(m[1]) - expected_motion[1]).abs() > 2e-6
                            || (m[2] - p.depth()[i][0]).abs() > 2e-6
                        {
                            motion_mismatch += 1;
                        }
                    } else if m[3] != 0. {
                        motion_mismatch += 1;
                    }
                }
            }
        }
        let mut strip_missing = 0;
        if frame <= 16 {
            for y in 44..47 {
                for x in 6..55 {
                    if geometry::area(&STRIP, x, y) > 0. && p.history()[y * OUTPUT + x][3] == 0. {
                        strip_missing += 1;
                    }
                }
            }
        }
        let error = edge_error(&p);
        let stats = json!({"frame":frame,"phase":phase,"previous":previous,"net_output_shift":shift,
            "current_covered":covered,"current_strip_samples":strip_samples,"strip_missing_output_pixels":strip_missing,
            "tie_pixels":ties,"coverage_mismatches":mismatches,"depth_mismatches":depth_mismatch,
            "motion_mismatches":motion_mismatch,"static_edge_area_mae":if frame<=16 {Some(error)}else{None}});
        write(
            &root,
            &format!("frame-{frame:02}.json"),
            &serde_json::to_vec_pretty(&stats).unwrap(),
        );
        assert!(
            p.0.iter().flatten().all(|v| v.is_finite()),
            "frame {frame} nonfinite raw data"
        );
        assert!(p.history().iter().all(|v| (0. ..=1.).contains(&v[3])));
        assert!(covered > 100 && covered < INPUT * INPUT);
        assert!(
            mismatches.is_empty(),
            "frame {frame} raster registration {stats}"
        );
        assert_eq!(
            depth_mismatch, 0,
            "frame {frame} color/depth registration {stats}"
        );
        assert_eq!(
            motion_mismatch, 0,
            "frame {frame} motion registration {stats}"
        );
        if frame == 1 {
            first_error = Some(error);
        }
        if frame == 9 {
            ninth_error = Some(error);
        }
        if frame == 17 {
            // After +8 output pixels, this known vacated strip is > the entire
            // current reconstruction footprint away from the new left edge.
            for y in 10..30 {
                for x in 8..12 {
                    assert_eq!(p.history()[y * OUTPUT + x], [0.; 4]);
                }
            }
        }
        if frame == 19 {
            cut = Some(p.history().to_vec());
        }
        if frame == 20 {
            assert_eq!(
                p.history(),
                cut.as_ref().unwrap(),
                "cut must equal fresh same-pose phase0"
            );
        }
        last_jitter = jitter;
        last_shift = shift;
    }
    let first = first_error.unwrap();
    let ninth = ninth_error.unwrap();
    write(
        &root,
        "result.json",
        &serde_json::to_vec_pretty(&json!({"draws":20,"raw_readbacks":20,
        "same_phase_first_mae":first,"same_phase_ninth_mae":ninth,"edge_improved":ninth<first,
        "strip_missing_coverage_retained":true,"appearance_accepted":false}))
        .unwrap(),
    );
    assert!(
        ninth < first,
        "same-phase analytic edge coverage did not improve: first={first} ninth={ninth}"
    );
}
