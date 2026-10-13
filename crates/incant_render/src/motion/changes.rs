//! Submitted lighting witness and the bounded owner-change upload. Candidate
//! writes never replace the accepted witness until the queue submission succeeds.
use super::{MotionError, plan, provenance};
use crate::lighting::scene::Light;

pub(crate) const UNIFORM_BYTES: u64 = 12_528;
const WORDS: usize = UNIFORM_BYTES as usize / 4;
const _: () = assert!((incant_render_data::MAX_INSTANCES + 1).div_ceil(32) <= WORDS - 4);

pub(super) struct State {
    pub buffer: wgpu::Buffer,
    upload: Box<[u32; WORDS]>,
    lights: LightHistory,
}

pub(super) fn charge(bytes: &mut u64, lights: usize) -> Result<(), MotionError> {
    if lights > crate::lighting::scene::MAX_DIRECTIONAL + crate::lighting::scene::MAX_LOCAL {
        return Err(MotionError::Capacity);
    }
    plan::charge(bytes, 2, UNIFORM_BYTES as usize)?;
    plan::charge(bytes, lights, size_of::<Light>())
}

impl State {
    pub fn new(device: &wgpu::Device, lights: usize) -> Self {
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Submitted owner changes"),
                size: UNIFORM_BYTES,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            upload: Box::new([0; WORDS]),
            lights: LightHistory::new(lights),
        }
    }
    pub fn prepare(
        &mut self,
        changes: &provenance::State,
        camera_static: bool,
        caster_changed: bool,
        lights: &[Light],
    ) -> Result<(), MotionError> {
        let lighting_changed = self.lights.changed(lights)?;
        fill_upload(
            &mut self.upload,
            changes,
            camera_static,
            caster_changed || lighting_changed,
        );
        Ok(())
    }
    pub fn write(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(self.upload.as_slice()),
        );
    }
    pub fn accept(&mut self, lights: &[Light]) {
        self.lights.accept(lights);
    }
}

struct LightHistory {
    accepted: Vec<Light>,
}
impl LightHistory {
    fn new(capacity: usize) -> Self {
        Self {
            accepted: Vec::with_capacity(capacity),
        }
    }
    fn changed(&self, candidate: &[Light]) -> Result<bool, MotionError> {
        if candidate.len() > self.accepted.capacity() {
            return Err(MotionError::Capacity);
        }
        Ok(!same_lights(&self.accepted, candidate))
    }
    fn accept(&mut self, lights: &[Light]) {
        // prepare proved capacity; no allocation is allowed after submission.
        debug_assert!(lights.len() <= self.accepted.capacity());
        self.accepted.clear();
        self.accepted.extend_from_slice(lights);
    }
}
fn same_lights(a: &[Light], b: &[Light]) -> bool {
    bytemuck::cast_slice::<Light, u8>(a) == bytemuck::cast_slice::<Light, u8>(b)
}
fn fill_upload(out: &mut [u32; WORDS], changes: &provenance::State, camera: bool, global: bool) {
    let (highest, classes, bits) = changes.candidate();
    out.fill(0);
    out[..4].copy_from_slice(&[
        classes,
        u32::from(camera) | (u32::from(global) << 1),
        highest,
        0,
    ]);
    out[4..4 + bits.len()].copy_from_slice(bits);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upload_keeps_highest_owner_and_candidate_bits_without_publishing_them() {
        let mut changes = provenance::State::new(100_000);
        changes.mark(100_000, true);
        let mut upload = [u32::MAX; WORDS];
        fill_upload(&mut upload, &changes, true, false);
        assert_eq!(upload[..4], [1 << (100_000 % 31), 1, 100_000, 0]);
        assert_eq!(upload[4 + 100_000 / 32], 1 << (100_000 % 32));
        assert_eq!(upload[WORDS - 2..], [0, 0]);
        changes.begin(); // abandon candidate; no light/history acceptance
        fill_upload(&mut upload, &changes, false, true);
        assert_eq!(upload[..4], [0, 2, 100_000, 0]);
        assert!(upload[4..].iter().all(|x| *x == 0));
    }
    #[test]
    fn abandoned_lighting_candidate_does_not_change_submitted_witness_or_allocate() {
        let light = Light {
            position_range: [0.; 4],
            direction_outer: [0.; 4],
            radiance_inner: [0.; 4],
            kind: [0; 4],
        };
        let mut moved = light;
        moved.position_range[0] = 1.;
        let mut history = LightHistory::new(1);
        assert!(history.changed(&[light]).unwrap());
        history.accept(&[light]);
        let allocations = crate::retained_alloc::measured(|| {
            assert!(history.changed(&[moved]).unwrap());
            // A failed later encode abandons the prepared candidate.
            assert!(!history.changed(&[light]).unwrap());
            assert_eq!(history.changed(&[light, moved]), Err(MotionError::Capacity));
            assert!(!history.changed(&[light]).unwrap());
            history.accept(&[moved]);
            assert!(!history.changed(&[moved]).unwrap());
        });
        assert_eq!(allocations, 0);
    }
    #[test]
    fn every_light_byte_and_count_affects_static_shading_witness() {
        let light = Light {
            position_range: [0.; 4],
            direction_outer: [0.; 4],
            radiance_inner: [0.; 4],
            kind: [0; 4],
        };
        assert!(same_lights(&[light], &[light]));
        assert!(!same_lights(&[], &[light]));
        for i in 0..size_of::<Light>() {
            let mut changed = light;
            bytemuck::bytes_of_mut(&mut changed)[i] ^= 1;
            assert!(!same_lights(&[light], &[changed]), "byte {i}");
        }
        let mut bytes = 0;
        charge(&mut bytes, 4112).unwrap();
        assert_eq!(bytes, 288_224);
        assert!(charge(&mut 0, 4113).is_err());
    }
}
