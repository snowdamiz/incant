//! Readback and exact comparisons. The nearest-texel mapping and the chrome
//! distance are written out independently of the renderer's code: integer
//! arithmetic for the mapping, f64 for the rounded-rectangle distance.
use crate::{Renderer, Result, wgpu};
use incant_render_data::{UiInput, UiRole};
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone)]
pub struct Rgba {
    pub size: [u32; 2],
    pub bytes: Vec<u8>,
}
impl Rgba {
    pub fn px(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.size[0] + x) * 4) as usize;
        [
            self.bytes[i],
            self.bytes[i + 1],
            self.bytes[i + 2],
            self.bytes[i + 3],
        ]
    }
    pub fn sha256(&self) -> String {
        incant_assets_runtime::sha256(&self.bytes)
    }
    pub fn write_png(&self, path: &std::path::Path) -> Result<()> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        let mut encoder = png::Encoder::new(file, self.size[0], self.size[1]);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&self.bytes)?;
        Ok(())
    }
}

/// Decode an 8-bit RGBA PNG written by this helper.
pub fn read_png(path: &std::path::Path) -> Result<Rgba> {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path)?));
    let mut reader = decoder.read_info()?;
    let mut bytes = vec![0; reader.output_buffer_size().ok_or("png output size")?];
    let info = reader.next_frame(&mut bytes)?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err("expected an 8-bit RGBA PNG".into());
    }
    bytes.truncate(info.buffer_size());
    Ok(Rgba {
        size: [info.width, info.height],
        bytes,
    })
}

pub fn read(renderer: &Renderer, texture: &wgpu::Texture) -> Result<Rgba> {
    let [w, h] = [texture.width(), texture.height()];
    let stride = (w * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("0067 readback"),
        size: u64::from(stride) * u64::from(h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(h),
            },
        },
        texture.size(),
    );
    renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| drop(tx.send(r)));
    renderer.device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    })?;
    rx.recv_timeout(Duration::from_secs(10))??;
    let mapped = buffer.slice(..).get_mapped_range();
    let bytes = mapped
        .chunks(stride as usize)
        .flat_map(|row| row[..w as usize * 4].iter().copied())
        .collect();
    drop(mapped);
    buffer.unmap();
    Ok(Rgba {
        size: [w, h],
        bytes,
    })
}

/// Signed distance from a pixel centre to a rounded rectangle, written from the
/// output pass contract: rect `[x, y, w, h]`, radii TL, TR, BR, BL.
pub fn rounded_distance(center: [f64; 2], rect: [f32; 4], radii: [f32; 4]) -> f64 {
    let [x, y, w, h] = rect.map(f64::from);
    let half = [w / 2., h / 2.];
    let p = [center[0] - x - half[0], center[1] - y - half[1]];
    let (top, bottom) = if p[0] > 0. {
        (radii[1], radii[2])
    } else {
        (radii[0], radii[3])
    };
    let radius = f64::from(if p[1] > 0. { bottom } else { top }).min(half[0].min(half[1]));
    let q = [p[0].abs() - half[0] + radius, p[1].abs() - half[1] + radius];
    let outside = (q[0].max(0.).powi(2) + q[1].max(0.).powi(2)).sqrt();
    outside + q[0].max(q[1]).min(0.) - radius
}

/// Output pixel x samples texel floor((x + 0.5) * n / 16) of the scaled scene.
pub fn texel(x: u32, n: u32) -> u32 {
    ((2 * x + 1) * n) / 32
}

/// Physical rectangle of a logical UI box. The screen-space UI contract maps the
/// logical UI viewport onto the target's viewport rectangle (or the whole
/// target): `region.xy + logical * region.wh / ui.viewport`.
pub fn ui_to_physical(b: [f32; 4], region: [f32; 4], viewport: [f32; 2]) -> [f32; 4] {
    let s = [region[2] / viewport[0], region[3] / viewport[1]];
    [
        region[0] + b[0] * s[0],
        region[1] + b[1] * s[1],
        region[0] + b[2] * s[0],
        region[1] + b[3] * s[1],
    ]
}
/// Union of every drawn UI widget box in physical pixels, expanded by one pixel.
pub fn ui_boxes(ui: Option<&UiInput>, region: [f32; 4]) -> Vec<[f32; 4]> {
    ui.map_or(vec![], |ui| {
        ui.widgets
            .iter()
            .map(|w| {
                let p = ui_to_physical(w.visible, region, ui.viewport);
                [p[0] - 1., p[1] - 1., p[2] + 1., p[3] + 1.]
            })
            .collect()
    })
}
fn inside(boxes: &[[f32; 4]], x: u32, y: u32) -> bool {
    let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
    boxes
        .iter()
        .any(|b| cx >= b[0] && cx < b[2] && cy >= b[1] && cy < b[3])
}

/// Exact check of a frame rendered at scale n/16 against a disabled-scale
/// reference rendered at the internal size with the scaled viewport rectangle.
/// Pixels within 0.75 px of the physical or scaled viewport edge (chrome mixing)
/// and under UI widgets are skipped; everything else must be byte-identical.
pub fn compare_mapping(
    frame: &Rgba,
    reference: &Rgba,
    n: u32,
    rect: [f32; 4],
    radii: [f32; 4],
    ui: &[[f32; 4]],
) -> Value {
    let factor = n as f32 / 16.;
    let scaled = rect.map(|v| v * factor);
    let (mut compared, mut skipped_ui, mut skipped_edge, mut mismatched) = (0u64, 0u64, 0u64, 0u64);
    let mut first = Vec::new();
    for y in 0..frame.size[1] {
        for x in 0..frame.size[0] {
            if inside(ui, x, y) {
                skipped_ui += 1;
                continue;
            }
            let physical = rounded_distance([x as f64 + 0.5, y as f64 + 0.5], rect, radii);
            let (tx, ty) = (texel(x, n), texel(y, n));
            let reference_edge =
                rounded_distance([tx as f64 + 0.5, ty as f64 + 0.5], scaled, [0.; 4]);
            if physical > -0.75 || reference_edge > -0.75 {
                skipped_edge += 1;
                continue;
            }
            compared += 1;
            let (a, b) = (frame.px(x, y), reference.px(tx, ty));
            if a != b {
                mismatched += 1;
                if first.len() < 16 {
                    first.push(
                        json!({"x": x, "y": y, "texel": [tx, ty], "frame": a, "reference": b}),
                    );
                }
            }
        }
    }
    json!({"scale16": n, "compared": compared, "mismatched": mismatched,
        "skipped_ui": skipped_ui, "skipped_viewport_edge": skipped_edge,
        "reference_size": reference.size, "frame_size": frame.size, "first_mismatches": first})
}

/// Pixels at least 0.75 px outside the physical viewport: chrome (and any UI
/// over it) must be byte-identical to the disabled control.
pub fn compare_outside(a: &Rgba, b: &Rgba, rect: [f32; 4], radii: [f32; 4]) -> Value {
    let (mut compared, mut mismatched) = (0u64, 0u64);
    for y in 0..a.size[1] {
        for x in 0..a.size[0] {
            if rounded_distance([x as f64 + 0.5, y as f64 + 0.5], rect, radii) >= 0.75 {
                compared += 1;
                mismatched += u64::from(a.px(x, y) != b.px(x, y));
            }
        }
    }
    json!({"compared": compared, "mismatched": mismatched})
}

/// Opaque control interiors: Button and TextInput visible boxes in physical
/// pixels shrunk by one pixel, minus corner squares of the 4 px radius plus one.
/// Surfaces are opaque (ui theme), so these pixels cannot depend on the scene.
pub fn opaque_regions(ui: Option<&UiInput>, region: [f32; 4]) -> Vec<[u32; 4]> {
    ui.map_or(vec![], |ui| {
        ui.widgets
            .iter()
            .filter(|w| matches!(w.role, UiRole::Button | UiRole::TextInput))
            .map(|w| {
                let v = ui_to_physical(w.visible, region, ui.viewport);
                [
                    v[0].ceil() as u32 + 1,
                    v[1].ceil() as u32 + 1,
                    (v[2].floor() as u32).saturating_sub(1),
                    (v[3].floor() as u32).saturating_sub(1),
                ]
            })
            .collect()
    })
}
pub fn compare_regions(a: &Rgba, b: &Rgba, regions: &[[u32; 4]]) -> Value {
    const CORNER: u32 = 5;
    let (mut compared, mut mismatched) = (0u64, 0u64);
    for r in regions {
        for y in r[1]..r[3] {
            for x in r[0]..r[2] {
                let near_x = x < r[0] + CORNER || x + CORNER >= r[2];
                let near_y = y < r[1] + CORNER || y + CORNER >= r[3];
                if near_x && near_y {
                    continue;
                }
                compared += 1;
                mismatched += u64::from(a.px(x, y) != b.px(x, y));
            }
        }
    }
    json!({"regions": regions.len(), "compared": compared, "mismatched": mismatched})
}
pub fn equal_bytes(a: &Rgba, b: &Rgba) -> Value {
    let differing = a
        .bytes
        .chunks(4)
        .zip(b.bytes.chunks(4))
        .filter(|(p, q)| p != q)
        .count();
    json!({"identical": a.size == b.size && a.bytes == b.bytes, "differing_pixels": differing,
        "a_sha256": a.sha256(), "b_sha256": b.sha256()})
}
/// Bounding box of pixels that differ from a background colour (coarse aspect
/// and placement evidence; not an exactness claim).
pub fn content_box(frame: &Rgba, background: [u8; 4], within: [u32; 4]) -> Value {
    let mut b = [u32::MAX, u32::MAX, 0u32, 0u32];
    let mut count = 0u64;
    for y in within[1]..within[3] {
        for x in within[0]..within[2] {
            if frame.px(x, y) != background {
                count += 1;
                b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
            }
        }
    }
    json!({"pixels": count, "box": if count > 0 { json!(b) } else { Value::Null }})
}
/// UI layout snapshot: widget IDs, roles, border and visible boxes, focus.
pub fn ui_json(ui: Option<&UiInput>) -> Value {
    ui.map_or(Value::Null, |ui| {
        json!({"viewport": ui.viewport, "widgets": ui.widgets.iter().map(|w| json!({
            "id": crate::fixture::ulid_text(w.id), "role": format!("{:?}", w.role),
            "bounds": w.bounds, "visible": w.visible, "focused": w.focused,
            "pressed": w.pressed, "disabled": w.disabled})).collect::<Vec<_>>()})
    })
}
