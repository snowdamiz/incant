//! Numeric query/lifecycle proof only. No image decoding or screenshot capture.
use super::*;
use incant_render_authoring::RendererAuthoringExt as _;
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires a native GPU; validates timestamp support or typed refusal serially"]
fn bounded_timestamp_queries_follow_real_submission_and_abandonment() {
    let instance = wgpu::Instance::default();
    let ordinary = crate::test_support::checked(
        pollster::block_on(crate::Renderer::new(&instance, None)).unwrap(),
    );
    let adapter_info = ordinary.adapter.get_info();
    let features = ordinary.adapter.features();
    let supported = features.contains(wgpu::Features::TIMESTAMP_QUERY);
    eprintln!(
        "Timestamp capability: adapter={adapter_info:?}; features={features:?}; supported={supported}"
    );
    let timed = pollster::block_on(crate::Renderer::new_with_gpu_timing(&instance, None));
    let renderer = if supported {
        let renderer = crate::test_support::checked(timed.unwrap());
        assert_eq!(renderer.adapter.get_info(), adapter_info);
        assert_eq!(renderer.adapter.features(), features);
        drop(ordinary);
        renderer
    } else {
        let error = match timed {
            Ok(_) => panic!("timestamp construction succeeded without TIMESTAMP_QUERY"),
            Err(error) => error,
        };
        assert_eq!(
            error.downcast_ref::<GpuTimingError>(),
            Some(&GpuTimingError::Unsupported),
            "missing timestamp capability must produce the exact typed refusal"
        );
        ordinary
    };
    let dir = tempfile::tempdir().unwrap();
    let (project, assets, _) = crate::test_support::fixture(dir.path());
    let scene = renderer.prepare_scene(&project, &assets).unwrap();
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Timestamp lifecycle target"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let encode = || {
        renderer
            .draw_scene(&scene, &view, texture.format(), 64, 64, None)
            .unwrap()
    };
    if !supported {
        assert_eq!(
            renderer.gpu_timing_stats().unwrap(),
            GpuTimingStats::default()
        );
        let submission = renderer.queue.submit([encode()]);
        renderer
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(15)),
            })
            .unwrap();
        assert!(renderer.take_gpu_timing().unwrap().is_none());
        assert_eq!(
            renderer.gpu_timing_stats().unwrap(),
            GpuTimingStats::default()
        );
        eprintln!(
            "Timestamp contract: typed Unsupported; ordinary frame completed; no timing measurement claimed"
        );
        return;
    }
    let first = encode();
    let second = encode();
    let abandoned = encode();
    let unmeasured = encode();
    assert_eq!(
        renderer.gpu_timing_stats().unwrap(),
        GpuTimingStats {
            enabled: true,
            pending: 3,
            skipped: 1,
            ..Default::default()
        }
    );
    assert!(renderer.take_gpu_timing().unwrap().is_none());
    drop(abandoned);
    assert_eq!(renderer.gpu_timing_stats().unwrap().abandoned, 1);
    // Submission order is deliberately different from encoding order.
    renderer.queue.submit([second, unmeasured, first]);
    let mut samples = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while samples.len() < 2 && Instant::now() < deadline {
        if let Some(sample) = renderer.take_gpu_timing().unwrap() {
            samples.push(sample);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(samples.len(), 2, "actual mapping callbacks must finish");
    samples.sort_by_key(|sample| sample.sequence);
    assert_eq!(
        samples.iter().map(|s| s.sequence).collect::<Vec<_>>(),
        [1, 2]
    );
    for sample in samples {
        assert!(sample.timestamp_period_ns.is_finite() && sample.timestamp_period_ns > 0.);
        assert!(sample.frame.nanoseconds.is_finite() && sample.frame.nanoseconds > 0.);
        assert!(sample.passes[GpuPass::InitializeScene.index()].is_some());
        assert!(sample.passes[GpuPass::Display.index()].is_some());
        assert!(sample.passes[GpuPass::Models.index()].is_some());
        assert!(sample.passes[GpuPass::Text.index()].is_none());
        eprintln!(
            "GPU timestamp result: {}",
            serde_json::to_string(&sample).unwrap()
        );
    }
    assert_eq!(renderer.gpu_timing_stats().unwrap().pending, 0);
    assert_eq!(renderer.gpu_timing_stats().unwrap().ready, 0);
    let reusable = encode();
    assert_eq!(renderer.gpu_timing_stats().unwrap().pending, 1);
    drop(reusable);
    assert_eq!(renderer.gpu_timing_stats().unwrap().abandoned, 2);
}
