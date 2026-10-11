//! Bounded review helper: actual retained native-world frames from cooked data.
//! No project, script VM, input, audio, presentation or performance claim. Controls
//! apply after reaching their committed tick, before capture; earlier PNGs remain
//! if a later step/render/write fails. Output I/O is not an atomic transaction.
#[path = "native_frames/request.rs"]
mod request;
use incant_render::{Renderer, RetainedTarget, wgpu};
use incant_render_native::{NativeRenderInput, VisualCaches};
use incant_runtime::NativeWorld;
use request::{Error, Request};
use std::{fs::OpenOptions, path::Path, time::Duration};

fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 6 {
        return Err("usage: native_frames <cooked-scene> <model-cache> <texture-cache> <camera-hex-id> <request-json> <new-output-directory>".into());
    }
    let camera = request::id(&args[3])?;
    let request: Request = serde_json::from_slice(&request::read(Path::new(&args[4]), 64 * 1024)?)?;
    request.validate()?;
    let mut world =
        NativeWorld::from_bytes(&request::read(Path::new(&args[0]), 128 * 1024 * 1024)?)?;
    if world.audio_nodes().len() != 0 {
        return Err("native_frames has no audio host".into());
    }
    request::counts(
        world.entity_count(),
        world
            .spatial_view()?
            .nodes()
            .filter(|n| n.flipbook.is_some())
            .count(),
    )?;
    let mut input = NativeRenderInput::load(
        &world.spatial_view()?,
        VisualCaches {
            models: Path::new(&args[1]),
            textures: Path::new(&args[2]),
        },
        camera,
    )?;
    let output = Path::new(&args[5]);
    std::fs::create_dir(output)?;
    let renderer = Renderer::headless()?;
    let mut scene = renderer.prepare_retained(input.input(), input.camera_label())?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Native frame review"),
        size: wgpu::Extent3d {
            width: request.width,
            height: request.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let mut records = Vec::with_capacity(request.ticks.len());
    for tick in 0..=*request.ticks.last().unwrap() {
        if tick != 0 {
            world.step()?;
        }
        let mut commands = request.commands(tick)?;
        world.apply(&mut commands)?;
        if request.ticks.binary_search(&tick).is_err() {
            continue;
        }
        input.try_sync(&world.spatial_view()?, |input, _, _| {
            scene.submit(
                &renderer,
                input,
                RetainedTarget {
                    view: &view,
                    format,
                    width: request.width,
                    height: request.height,
                    viewport: None,
                },
            )
        })?;
        let name = format!("tick-{tick:05}.png");
        save(
            &renderer,
            &texture,
            request.width,
            request.height,
            &output.join(&name),
        )?;
        let states: Vec<_> = world
            .spatial_view()?
            .nodes()
            .filter_map(|node| {
                let state = node.flipbook?;
                Some(serde_json::json!({
                    "entity":format!("{:032x}", u128::from_be_bytes(node.id.0)),
                    "frame":state.frame(),"playing":state.playing(),"completed":state.completed()
                }))
            })
            .collect();
        records.push(serde_json::json!({"tick":tick,"png":name,"flipbooks":states,
            "binding_generation":input.binding_generation(),"topology_generation":input.topology_generation()}));
    }
    renderer.check_device()?;
    let report = serde_json::json!({"adapter":renderer.adapter_name,"width":request.width,"height":request.height,
        "tick_rate":world.tick_rate(),"captures":records,"scope":"retained native scene frames; no scripts, audio or presentation"});
    let text = serde_json::to_string_pretty(&report)?;
    use std::io::Write;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("report.json"))?
        .write_all(text.as_bytes())?;
    println!("{text}");
    Ok(())
}
fn save(
    renderer: &Renderer,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    path: &Path,
) -> Result<(), Error> {
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Native frame readback"),
        size: u64::from(stride) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    renderer.device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    })?;
    rx.recv_timeout(Duration::from_secs(10))??;
    let data = buffer.slice(..).get_mapped_range();
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in data.chunks(stride as usize) {
        pixels.extend_from_slice(&row[..(width * 4) as usize]);
    }
    drop(data);
    buffer.unmap();
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut png = png::Encoder::new(file, width, height);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()?.write_image_data(&pixels)?;
    Ok(())
}
