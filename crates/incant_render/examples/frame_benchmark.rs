//! Read-only, fenced CPU+GPU frame timing for an existing cooked project.
//! This excludes loading/PNG output and includes encoding, submission and a GPU
//! completion wait. It is not a GPU timestamp or presented-frame benchmark.
use incant_assets::AssetStore;
use incant_doc::Project;
use incant_render::{Renderer, wgpu};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: frame_benchmark <project> [camera-id]")?,
    );
    let project = Project::from_text(&std::fs::read_to_string(&path)?)?;
    let mut assets = AssetStore::default();
    assets.sync_project(&project, path.parent().ok_or("project parent missing")?)?;
    let renderer = Renderer::headless()?;
    let camera = std::env::args().nth(2);
    let mut scene = renderer.prepare_scene(&project, &assets)?;
    if let Some(id) = camera.as_deref() {
        scene = scene.with_camera(id)?;
    }
    let (width, height) = (1920, 1080);
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Frame benchmark"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let mut milliseconds = Vec::new();
    for frame in 0..35 {
        let start = Instant::now();
        let commands = renderer.draw_scene(&scene, &view, format, width, height, None)?;
        let submission = renderer.queue.submit([commands]);
        renderer.device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(30)),
        })?;
        if frame >= 5 {
            milliseconds.push(start.elapsed().as_secs_f64() * 1000.);
        }
    }
    milliseconds.sort_by(f64::total_cmp);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "adapter":renderer.adapter_name,"width":width,"height":height,
            "warmup_frames":5,"measured_frames":milliseconds.len(),
            "median_ms":(milliseconds[14]+milliseconds[15])*0.5,"p95_ms":milliseconds[28],
            "min_ms":milliseconds[0],"max_ms":milliseconds[29],
            "sorted_samples_ms":milliseconds,
            "scene":scene.stats(),
            "camera":camera,
            "method":"CPU encode + queue submit + GPU completion wait; no present, readback, PNG, asset load or simulation"
        }))?
    );
    Ok(())
}
