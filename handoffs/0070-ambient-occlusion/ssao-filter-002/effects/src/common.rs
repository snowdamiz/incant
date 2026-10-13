//! Receipts, frame accounting, verdicts, renderer construction, readback and
//! retained raw outputs shared by the numerical cases and appearance renders.
use incant_render::{AmbientOcclusion, Bloom, Renderer, wgpu};
use serde_json::{Value, json};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// The renderer's display-referred backdrop (#141519, output.rs).
pub const BACKDROP_SRGB: [u8; 3] = [20, 21, 25];
/// The packet's connected interface values: numerical fixture inputs in
/// world/view units, not an approved art preset.
pub const AO: AmbientOcclusion = AmbientOcclusion {
    radius: 2.0,
    bias: 0.01,
    strength: 1.0,
};
/// The bloom device tests' configuration, used for the composed appearance.
pub const BLOOM: Bloom = Bloom {
    threshold: 1.0,
    intensity: 0.5,
    levels: 6,
};

/// One raw receipt line; the guarded runner retains the whole log.
pub fn emit(case: &str, kind: &str, body: Value) {
    println!(
        "EFFECTS-RECEIPT {}",
        json!({"case": case, "kind": kind, "body": body})
    );
}

/// Every encode/submit/tick attempt is charged before it starts, including
/// attempts that are expected to be refused. The declaration equals plan.json.
pub struct Frames {
    case: &'static str,
    declared: u32,
    labels: Vec<String>,
}
impl Frames {
    pub fn new(case: &'static str, declared: u32) -> Self {
        Self {
            case,
            declared,
            labels: Vec::new(),
        }
    }
    pub fn charge(&mut self, label: &str) -> Result<()> {
        if self.labels.len() >= self.declared as usize {
            return Err(format!(
                "{}: frame {} ({label}) exceeds the declared {}",
                self.case,
                self.labels.len() + 1,
                self.declared
            )
            .into());
        }
        self.labels.push(label.to_owned());
        Ok(())
    }
    pub fn used(&self) -> usize {
        self.labels.len()
    }
    pub fn receipt(&self) {
        emit(
            self.case,
            "budget",
            json!({"declared": self.declared, "frames": self.used(), "labels": self.labels}),
        );
    }
}

/// Collects every check so all receipts print before the verdict.
#[derive(Default)]
pub struct Verdict {
    failures: Vec<String>,
    checks: Vec<Value>,
}
impl Verdict {
    pub fn check(&mut self, ok: bool, what: &str, detail: Value) {
        self.checks
            .push(json!({"check": what, "ok": ok, "detail": detail}));
        if !ok {
            self.failures.push(what.to_owned());
        }
    }
    pub fn finish(self, case: &str) -> bool {
        let ok = self.failures.is_empty();
        emit(
            case,
            "verdict",
            json!({"failures": self.failures, "checks": self.checks}),
        );
        ok
    }
}

/// A renderer on a helper-owned instance, so wgpu's registry report is visible.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub renderer: Renderer,
}
impl Gpu {
    /// The backend must equal the runner's explicit expectation.
    pub fn new() -> Result<Self> {
        let instance = wgpu::Instance::default();
        let renderer = pollster::block_on(Renderer::new(&instance, None))?;
        let expected = std::env::var("INCANT_EXPECT_GPU_BACKEND")
            .map_err(|_| "INCANT_EXPECT_GPU_BACKEND must be set by the guarded runner")?;
        let actual = format!("{:?}", renderer.adapter.get_info().backend);
        if !matches!(expected.as_str(), "Metal" | "Vulkan" | "Dx12") || actual != expected {
            return Err(format!("adapter backend {actual} is not the required {expected}").into());
        }
        Ok(Self { instance, renderer })
    }
    /// Startup configuration only: prepares the finite SSAO pipeline catalog.
    pub fn with_ao(mut self, settings: AmbientOcclusion) -> Result<Self> {
        pollster::block_on(self.renderer.enable_ambient_occlusion(settings))?;
        Ok(self)
    }
    /// Startup configuration only: prepares the finite bloom pipeline catalog.
    pub fn with_bloom(mut self, config: Bloom) -> Result<Self> {
        pollster::block_on(self.renderer.enable_bloom(config))?;
        Ok(self)
    }
    pub fn adapter(&self) -> Value {
        let info = self.renderer.adapter.get_info();
        json!({"name": info.name, "backend": format!("{:?}", info.backend),
            "device_type": format!("{:?}", info.device_type), "driver": info.driver,
            "driver_info": info.driver_info, "vendor": info.vendor, "device": info.device})
    }
    pub fn registry(&self) -> Value {
        registry_of(&self.instance)
    }
}

/// Live wgpu-core registry counts; not a driver-memory measurement.
pub fn registry_of(instance: &wgpu::Instance) -> Value {
    instance.generate_report().map_or(Value::Null, |r| {
        json!({"textures": r.hub.textures.num_allocated,
            "render_pipelines": r.hub.render_pipelines.num_allocated,
            "compute_pipelines": r.hub.compute_pipelines.num_allocated,
            "buffers": r.hub.buffers.num_allocated,
            "bind_groups": r.hub.bind_groups.num_allocated})
    })
}

pub fn target(renderer: &Renderer, size: [u32; 2]) -> wgpu::Texture {
    renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("0070 output target"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// Row-major RGBA8 with row padding removed.
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
}

pub fn read(renderer: &Renderer, texture: &wgpu::Texture) -> Result<Rgba> {
    let [w, h] = [texture.width(), texture.height()];
    let stride = (w * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("0070 readback"),
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

pub fn write_png(path: &Path, bytes: &[u8], size: [u32; 2]) -> Result<()> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let mut encoder = png::Encoder::new(file, size[0], size[1]);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(bytes)?;
    Ok(())
}

/// Retained raw frames and an index. Files are created new, never replaced.
pub struct Store {
    dir: PathBuf,
    entries: Vec<Value>,
}
impl Store {
    pub fn create(dir: &Path) -> Result<Self> {
        std::fs::create_dir(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            entries: Vec::new(),
        })
    }
    pub fn dir(&self) -> &Path {
        &self.dir
    }
    pub fn keep(&mut self, label: &str, frame: &Rgba, meta: Value) -> Result<()> {
        let name = format!("{label}.rgba");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.dir.join(&name))?;
        file.write_all(&frame.bytes)?;
        self.entries
            .push(json!({"label": label, "file": name, "size": frame.size,
            "rgba_sha256": frame.sha256(), "meta": meta}));
        Ok(())
    }
    pub fn png(&mut self, label: &str, frame: &Rgba, meta: Value) -> Result<()> {
        let name = format!("{label}.png");
        write_png(&self.dir.join(&name), &frame.bytes, frame.size)?;
        let png_sha256 = incant_assets_runtime::sha256(&std::fs::read(self.dir.join(&name))?);
        self.entries
            .push(json!({"label": label, "file": name, "size": frame.size,
            "rgba_sha256": frame.sha256(), "png_sha256": png_sha256, "meta": meta}));
        Ok(())
    }
    /// The three R8 stage planes of one AO-on frame: AO pass (T0), horizontal
    /// pass (scratch) and vertical pass (T0), tightly packed at internal size.
    /// Each plane must be exactly `size` bytes and the size the frame's own;
    /// anything else refuses the case instead of retaining a partial capture.
    pub fn keep_stages(&mut self, label: &str, size: [u32; 2], planes: [&[u8]; 3]) -> Result<()> {
        let bytes = u64::from(size[0]) * u64::from(size[1]);
        for (stage, plane) in ["raw", "h", "v"].into_iter().zip(planes) {
            if plane.len() as u64 != bytes {
                return Err(format!("{label}: {stage} plane has {} bytes, expected {bytes}", plane.len()).into());
            }
            let name = format!("{label}.{stage}.r8");
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.dir.join(&name))?;
            file.write_all(plane)?;
            self.entries.push(json!({"label": format!("{label}.{stage}"), "file": name, "size": size,
                "r8_sha256": incant_assets_runtime::sha256(plane),
                "meta": {"frame": label, "stage": stage}}));
        }
        Ok(())
    }
    pub fn finish(&self, summary: Value) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.dir.join("index.json"))?;
        let index = json!({"frames": self.entries, "summary": summary});
        file.write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }
}

/// Byte comparison of two complete frames.
pub fn same(a: &Rgba, b: &Rgba) -> Value {
    let differing = if a.size == b.size {
        a.bytes
            .chunks(4)
            .zip(b.bytes.chunks(4))
            .filter(|(p, q)| p != q)
            .count()
    } else {
        usize::MAX
    };
    json!({"identical": a.size == b.size && a.bytes == b.bytes,
        "differing_pixels": differing, "a_sha256": a.sha256(), "b_sha256": b.sha256()})
}
pub fn identical(a: &Rgba, b: &Rgba) -> bool {
    a.size == b.size && a.bytes == b.bytes
}

/// Pixels where `candidate` is darker / brighter than `baseline` in any colour channel.
pub fn darker_brighter(candidate: &Rgba, baseline: &Rgba) -> (usize, usize) {
    let mut darker = 0;
    let mut brighter = 0;
    for (c, b) in candidate.bytes.chunks(4).zip(baseline.bytes.chunks(4)) {
        if (0..3).any(|i| c[i] < b[i]) {
            darker += 1;
        }
        if (0..3).any(|i| c[i] > b[i]) {
            brighter += 1;
        }
    }
    (darker, brighter)
}

/// Signed distance from a pixel centre to the output pass's rounded rectangle
/// (rect `[x, y, w, h]`, radii TL, TR, BR, BL), written independently in f64.
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

/// Pixels at least 0.75 px outside the viewport (chrome/canvas) must not differ.
pub fn chrome_identical(a: &Rgba, b: &Rgba, rect: [f32; 4], radii: [f32; 4]) -> Value {
    let (mut compared, mut differing) = (0u64, 0u64);
    for y in 0..a.size[1] {
        for x in 0..a.size[0] {
            if rounded_distance([f64::from(x) + 0.5, f64::from(y) + 0.5], rect, radii) >= 0.75 {
                compared += 1;
                differing += u64::from(a.px(x, y) != b.px(x, y));
            }
        }
    }
    json!({"compared": compared, "differing": differing})
}
