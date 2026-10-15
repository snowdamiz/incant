//! Four bounded real submissions join fixed history, overlay depth and queries.
use super::*;
use crate::timing::{GpuPass, GpuTimingError, GpuTimingStats};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires native GPU; four numeric frames, no images or presentation"]
fn fixed_history_overlay_depth_and_timestamps_share_real_submissions() {
    run(false, false);
}

#[test]
#[ignore = "requires native GPU; four bloom/fixed frames, no images or presentation"]
fn fixed_history_bloom_and_display_timestamps_share_real_submissions() {
    run(true, false);
}

#[test]
#[ignore = "requires native GPU; four SSAO/fixed/bloom frames, no images or presentation"]
fn fixed_history_ssao_bloom_and_model_timestamps_share_real_submissions() {
    run(true, true);
}

fn run(with_bloom: bool, with_ssao: bool) {
    let temp = tempfile::tempdir().unwrap();
    crate::test_support::model(temp.path(), 0.);
    let path = temp.path().join("triangle.gltf");
    let mut gltf: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    gltf["materials"] = serde_json::json!([
        {"pbrMetallicRoughness":{"baseColorFactor":[1.,0.,0.,1.]}, "doubleSided":true},
        {"alphaMode":"BLEND", "pbrMetallicRoughness":{"baseColorFactor":[0.,1.,0.,0.5]}, "doubleSided":true}
    ]);
    let mut opaque = gltf["meshes"][0]["primitives"][0].clone();
    opaque["material"] = 0.into();
    let mut overlay = opaque.clone();
    overlay["material"] = 1.into();
    gltf["meshes"][0]["primitives"] = serde_json::json!([opaque, overlay]);
    fs::write(path, serde_json::to_vec(&gltf).unwrap()).unwrap();
    let cooked = incant_assets::cook_gltf(
        temp.path(),
        Path::new("triangle.gltf"),
        &temp.path().join(".incant/cache/models"),
    )
    .unwrap();
    let source = Arc::new(
        RuntimeAsset::load_model(
            "model",
            1,
            &temp.path().join(".incant/cache/models"),
            &cooked.metadata.fingerprint,
        )
        .unwrap(),
    );
    temp.close().unwrap();
    let input = SceneInput {
        nodes: vec![
            SceneNode {
                id: "model".into(),
                world: DMat4::IDENTITY,
                camera: None,
                light: None,
                mesh: Some(MeshInput {
                    source,
                    cast_shadows: true,
                    animation: None,
                    animation_blend: None,
                    animation_pose: None,
                    animation_ik: None,
                    animation_root: None,
                }),
                sprite: None,
                tilemap: None,
                diagnostic: false,
            },
            SceneNode {
                id: "camera".into(),
                world: DMat4::from_translation(DVec3::new(0., 0., 10.)),
                camera: Some(incant_types::Camera {
                    fov_degrees: 60.,
                    near: 0.1,
                    far: 50.,
                    projection: incant_types::CameraProjection::Orthographic { vertical_size: 8. },
                }),
                light: None,
                mesh: None,
                sprite: None,
                tilemap: None,
                diagnostic: false,
            },
        ],
        ..Default::default()
    };

    let instance = wgpu::Instance::default();
    let ordinary =
        crate::test_support::checked(pollster::block_on(Renderer::new(&instance, None)).unwrap());
    let info = ordinary.adapter.get_info();
    let features = ordinary.adapter.features();
    let supported = features.contains(wgpu::Features::TIMESTAMP_QUERY);
    eprintln!("FIXED_JOINED_ADAPTER {info:?}; features={features:?}; timestamps={supported}");
    let timed = pollster::block_on(Renderer::new_with_gpu_timing(&instance, None));
    let mut renderer = if supported {
        let timed = crate::test_support::checked(timed.unwrap());
        assert_eq!(timed.adapter.get_info(), info);
        assert_eq!(timed.adapter.features(), features);
        drop(ordinary);
        timed
    } else {
        let error = match timed {
            Ok(_) => panic!("timestamps admitted without capability"),
            Err(e) => e,
        };
        assert_eq!(
            error.downcast_ref::<GpuTimingError>(),
            Some(&GpuTimingError::Unsupported)
        );
        ordinary
    };
    if with_bloom {
        let enabled = pollster::block_on(renderer.enable_bloom(crate::Bloom {
            threshold: 1.,
            intensity: 0.5,
            levels: 6,
        }));
        if cfg!(all(
            feature = "cooked-output",
            not(feature = "cooked-bloom"),
            not(target_arch = "wasm32")
        )) {
            assert_eq!(
                enabled.unwrap_err().downcast_ref::<crate::BloomError>(),
                Some(&crate::BloomError::CookedOutput)
            );
            assert!(renderer.bloom.is_none());
        } else {
            enabled.unwrap();
            assert!(renderer.bloom.is_some());
        }
    }
    if with_ssao {
        let enabled =
            pollster::block_on(renderer.enable_ambient_occlusion(crate::AmbientOcclusion {
                radius: 1.,
                bias: 0.01,
                strength: 1.,
            }));
        if cfg!(all(
            feature = "cooked-output",
            not(feature = "cooked-ssao"),
            not(target_arch = "wasm32")
        )) {
            assert_eq!(
                enabled
                    .unwrap_err()
                    .downcast_ref::<crate::AmbientOcclusionError>(),
                Some(&crate::AmbientOcclusionError::CookedMaterials)
            );
            assert!(renderer.ambient_occlusion.is_none());
        } else {
            enabled.unwrap();
            assert!(renderer.ambient_occlusion.is_some());
        }
        eprintln!(
            "SSAO_JOINED admitted={}; cooked_refusal={}",
            renderer.ambient_occlusion.is_some(),
            cfg!(all(
                feature = "cooked-output",
                not(feature = "cooked-ssao"),
                not(target_arch = "wasm32")
            ))
        );
    }
    pollster::block_on(renderer.enable_temporal_accumulation()).unwrap();
    let mut retained = renderer.prepare_retained(&input, "camera").unwrap();
    retained
        .enable_temporal_history(
            &renderer,
            crate::temporal::TemporalConfig {
                history_weight: 0.5,
                relative_depth: 0.01,
                absolute_depth: 0.001,
                reactive_threshold: 1.,
                reactive_strength: 1.,
            },
        )
        .unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = crate::hdr_output::texture(&renderer, format, 65, 33);
    let view = texture.create_view(&Default::default());
    for index in 0..4 {
        if index == 2 {
            let scale = crate::resolution::ResolutionScale::new(8).unwrap();
            retained
                .set_dynamic_resolution(Some(crate::resolution::ResolutionConfig {
                    target_ms: 16.,
                    deadband: 0.1,
                    minimum: scale,
                    maximum: scale,
                    initial: scale,
                    overload_samples: 2,
                    recovery_samples: 2,
                    cooldown_samples: 1,
                    slew: 1,
                }))
                .unwrap();
        }
        let submission = retained
            .submit(
                &renderer,
                &input,
                RetainedTarget {
                    view: &view,
                    format,
                    width: 65,
                    height: 33,
                    viewport: None,
                },
            )
            .unwrap();
        renderer
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(15)),
            })
            .unwrap();
        let frame = retained.temporal_frame().unwrap();
        assert_eq!(frame.jitter_phase(), Some(index % 2));
        assert_eq!(
            frame.motion().internal_size(),
            if index < 2 { [65, 33] } else { [33, 17] }
        );
        assert_eq!(frame.motion().has_previous(), index % 2 == 1);
        if supported {
            let deadline = Instant::now() + Duration::from_secs(15);
            let sample = loop {
                if let Some(sample) = renderer.take_gpu_timing().unwrap() {
                    break sample;
                }
                assert!(
                    Instant::now() < deadline,
                    "joined frame timing callback did not complete"
                );
                std::thread::sleep(Duration::from_millis(2));
            };
            assert_eq!(sample.sequence, u64::from(index + 1));
            assert!(sample.frame.nanoseconds.is_finite() && sample.frame.nanoseconds > 0.);
            for pass in [GpuPass::InitializeScene, GpuPass::Models, GpuPass::Display] {
                let span =
                    sample.passes[pass.index()].expect("actual joined pass must be measured");
                assert!(span.nanoseconds.is_finite() && span.nanoseconds > 0.);
            }
            for span in sample.passes.iter().flatten() {
                assert!(sample.frame.start_ticks <= span.start_ticks);
                assert!(sample.frame.end_ticks >= span.end_ticks);
            }
            assert!(renderer.take_gpu_timing().unwrap().is_none());
            assert_eq!(
                renderer.gpu_timing_stats().unwrap(),
                GpuTimingStats {
                    enabled: true,
                    ..Default::default()
                }
            );
            eprintln!(
                "FIXED_JOINED_TIMING {}",
                serde_json::to_string(&sample).unwrap()
            );
        } else {
            assert!(renderer.take_gpu_timing().unwrap().is_none());
            assert_eq!(
                renderer.gpu_timing_stats().unwrap(),
                GpuTimingStats::default()
            );
            eprintln!(
                "FIXED_JOINED_UNSUPPORTED frame={index}; submitted_and_completed=true; no timing sample"
            );
        }
    }
}
