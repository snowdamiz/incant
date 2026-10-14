//! Test-only raw grid observer. Never creates or changes the lighting cache.
use super::super::{DEPTH_SLICES, GpuLights, GridBuffers, LocalLightSelection};
use crate::{Renderer, camera::CameraView};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};

pub(super) struct GridReadback {
    pub dimensions: [u32; 2],
    pub words: u32,
    pub counts: Vec<u32>,
    pub masks: Vec<u32>,
}

impl GridReadback {
    pub fn validate(&self, local: u32) -> Result<(), &'static str> {
        if local > 4096 || self.words != local.div_ceil(32).max(1) {
            return Err("wrong local count or mask width");
        }
        let clusters = self.dimensions[0]
            .checked_mul(self.dimensions[1])
            .and_then(|n| n.checked_mul(DEPTH_SLICES))
            .filter(|n| *n > 0)
            .ok_or("invalid grid dimensions")? as usize;
        let mask_len = clusters
            .checked_mul(self.words as usize)
            .ok_or("invalid grid dimensions")?;
        if self.counts.len() != clusters || self.masks.len() != mask_len {
            return Err("wrong buffer lengths");
        }
        let tail = local % 32;
        let allowed = if local == 0 {
            0
        } else if tail == 0 {
            u32::MAX
        } else {
            u32::MAX >> (32 - tail)
        };
        for (count, mask) in self
            .counts
            .iter()
            .zip(self.masks.chunks(self.words as usize))
        {
            if *count != mask.iter().map(|word| word.count_ones()).sum::<u32>() {
                return Err("count differs from mask popcount");
            }
            if mask[mask.len() - 1] & !allowed != 0 {
                return Err("mask selects a nonexistent local light");
            }
        }
        Ok(())
    }

    pub fn report(&self, name: &str, local: u32) {
        self.validate(local).unwrap();
        fn digest(words: &[u32]) -> String {
            let mut hash = Sha256::new();
            for word in words {
                hash.update(word.to_le_bytes());
            }
            format!("{:x}", hash.finalize())
        }
        println!(
            "cluster-readback {name} grid={}x{}x{DEPTH_SLICES} words={} local={local} counts_sha256={} masks_sha256={}",
            self.dimensions[0],
            self.dimensions[1],
            self.words,
            digest(&self.counts),
            digest(&self.masks),
        );
    }
}

pub(super) fn last_grid(r: &Renderer) -> Arc<GridBuffers> {
    Arc::clone(
        r.lighting
            .cache
            .lock()
            .unwrap()
            .as_ref()
            .expect("caller prepared a grid"),
    )
}

/// Submit the copy after the caller's submission on the same queue. Calling
/// buffers() here would replace the observed grid for a mismatched key.
pub(super) fn read_last_grid(r: &Renderer) -> GridReadback {
    let grid = last_grid(r);
    let mut encoder = r.device.create_command_encoder(&Default::default());
    let readback = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Test-only cluster assignment readback"),
        size: grid.counts.size() + grid.masks.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&grid.counts, 0, &readback, 0, grid.counts.size());
    encoder.copy_buffer_to_buffer(
        &grid.masks,
        0,
        &readback,
        grid.counts.size(),
        grid.masks.size(),
    );
    r.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    r.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    let view = readback.slice(..).get_mapped_range();
    let (chunks, tail) = view.as_chunks::<4>();
    assert!(tail.is_empty());
    let mut counts: Vec<_> = chunks.iter().map(|v| u32::from_le_bytes(*v)).collect();
    let masks = counts.split_off(grid.counts.size() as usize / 4);
    drop(view);
    readback.unmap();
    GridReadback {
        dimensions: grid.dimensions,
        words: grid.words,
        counts,
        masks,
    }
}

pub(super) fn grid_readback_at(
    r: &Renderer,
    lights: &GpuLights,
    viewport: [f32; 4],
    camera: CameraView,
) -> GridReadback {
    let shadows = r
        .shadows
        .prepare(&r.device, camera, viewport, &[], &[])
        .unwrap();
    let frame = r
        .lighting
        .prepare(
            &r.device,
            lights,
            viewport,
            LocalLightSelection::Clustered,
            camera,
            &shadows,
        )
        .unwrap();
    let mut encoder = r.device.create_command_encoder(&Default::default());
    frame
        .compute
        .as_ref()
        .expect("local lights require compute")
        .encode(&mut encoder, None);
    r.queue.submit([encoder.finish()]);
    read_last_grid(r)
}

#[test]
fn readback_invariants_reject_bad_counts_tail_bits_and_lengths() {
    let mut data = GridReadback {
        dimensions: [1, 1],
        words: 2,
        counts: vec![2; 24],
        masks: [1, 1].repeat(24),
    };
    assert_eq!(data.validate(33), Ok(()));
    data.counts[0] = 1;
    assert_eq!(data.validate(33), Err("count differs from mask popcount"));
    data.counts[0] = 2;
    data.masks[1] = 2;
    assert_eq!(
        data.validate(33),
        Err("mask selects a nonexistent local light")
    );
    data.masks[1] = 1;
    data.masks.pop();
    assert_eq!(data.validate(33), Err("wrong buffer lengths"));
}

#[test]
fn readback_invariants_admit_full_last_word_and_directional_zero() {
    for local in [0u32, 1, 31, 32, 33, 4096] {
        let words = local.div_ceil(32).max(1);
        let mut data = GridReadback {
            dimensions: [1, 1],
            words,
            counts: vec![u32::from(local > 0); 24],
            masks: vec![0; 24 * words as usize],
        };
        if local > 0 {
            for mask in data.masks.chunks_mut(words as usize) {
                mask[(local as usize - 1) / 32] = 1 << ((local - 1) % 32);
            }
        }
        assert_eq!(data.validate(local), Ok(()));
    }
}
