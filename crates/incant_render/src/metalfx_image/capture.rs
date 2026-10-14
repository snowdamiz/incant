//! Final display output readback only; no private shader or quality oracle.
use super::{HEIGHT, WIDTH};
use crate::Renderer;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) struct Output {
    root: PathBuf,
    bytes: usize,
    pub files: Vec<Value>,
}
impl Output {
    pub fn new(root: PathBuf) -> Self {
        fs::create_dir(&root).expect("create-only image evidence directory");
        Self {
            root,
            bytes: 0,
            files: vec![],
        }
    }
    pub fn write(&mut self, name: &str, bytes: &[u8]) {
        let path = Path::new(name);
        assert!(
            path.components()
                .all(|c| matches!(c, std::path::Component::Normal(_)))
        );
        let next = self.bytes.checked_add(bytes.len()).unwrap();
        assert!(next <= 128 * 1024 * 1024, "whole fixture output cap");
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        f.write_all(bytes).unwrap();
        f.sync_all().unwrap();
        self.bytes = next;
        self.files
            .push(json!({"path":name,"bytes":bytes.len(),"sha256":hash(bytes)}));
    }
    pub fn assets(&mut self, root: &Path) {
        fn walk(root: &Path, path: &Path, out: &mut Vec<PathBuf>) {
            let mut entries: Vec<_> = fs::read_dir(path)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            entries.sort();
            for p in entries {
                let m = fs::symlink_metadata(&p).unwrap();
                assert!(!m.file_type().is_symlink());
                if m.is_dir() {
                    walk(root, &p, out);
                } else {
                    assert!(m.is_file());
                    out.push(p.strip_prefix(root).unwrap().to_owned());
                }
                assert!(out.len() <= 128, "bounded generated fixture file inventory");
            }
        }
        let mut paths = vec![];
        walk(root, root, &mut paths);
        let size: u64 = paths
            .iter()
            .map(|p| fs::metadata(root.join(p)).unwrap().len())
            .sum();
        assert!(
            size <= 16 * 1024 * 1024,
            "bounded authored/cooked fixture payload"
        );
        for path in paths {
            self.write(
                &format!("fixture/{}", path.to_str().unwrap()),
                &fs::read(root.join(path)).unwrap(),
            );
        }
    }
    pub fn image(&mut self, arm: &str, frame: u32, pixels: &[u8]) {
        assert_eq!(pixels.len(), WIDTH as usize * HEIGHT as usize * 4);
        self.write(&format!("{arm}-{frame:02}.rgba8"), pixels);
        let mut bytes = vec![];
        {
            let mut png = png::Encoder::new(&mut bytes, WIDTH, HEIGHT);
            png.set_color(png::ColorType::Rgba);
            png.set_depth(png::BitDepth::Eight);
            png.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
            png.write_header()
                .unwrap()
                .write_image_data(pixels)
                .unwrap();
        }
        self.write(&format!("{arm}-{frame:02}.png"), &bytes);
    }
}
pub(super) fn wait(r: &Renderer) {
    r.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
}
pub(super) fn texture(r: &Renderer) -> wgpu::Texture {
    r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("MetalFX real material final display capture"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
pub(super) fn read(r: &Renderer, texture: &wgpu::Texture) -> Vec<u8> {
    let stride = (WIDTH * 4).div_ceil(256) * 256;
    let buffer = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Bounded final display readback"),
        size: u64::from(stride) * u64::from(HEIGHT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = r.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    r.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |v| {
        let _ = tx.send(v);
    });
    wait(r);
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range();
    let pixels = mapped
        .chunks_exact(stride as usize)
        .flat_map(|r| r[..WIDTH as usize * 4].iter().copied())
        .collect();
    drop(mapped);
    buffer.unmap();
    pixels
}
