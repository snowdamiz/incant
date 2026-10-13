//! Numeric query/lifecycle proof only. No image decoding or screenshot capture.
use super::*;
use incant_render_authoring::RendererAuthoringExt as _;
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires native encoder timestamp support; run serially under GPU ownership"]
fn bounded_timestamp_queries_follow_real_submission_and_abandonment() {
    let instance = wgpu::Instance::default();
    let renderer =
        pollster::block_on(crate::Renderer::new_with_gpu_timing(&instance, None)).unwrap();
    eprintln!("Timestamp adapter: {:?}", renderer.adapter.get_info());
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
