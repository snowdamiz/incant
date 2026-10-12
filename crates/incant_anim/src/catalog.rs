//! Immutable root-only catalogs. Admission owns bounded arrays; frame sampling
//! borrows them and reuses precomputed origins/cycle transforms.
use crate::{
    RootMotionDelta, RootMotionError, RootMotionSampler,
    root_motion::PreparedRootMotion,
    track::{Interpolation, Kind, Track},
};

pub const MAX_ROOT_MOTION_CLIPS: usize = 256;
pub const MAX_ROOT_MOTION_KEYS: usize = 1_000_000;
pub const MAX_ROOT_MOTION_CATALOG_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_SCENE_ROOT_MOTION_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ROOT_MOTION_INSTANCES: usize = 1024;

#[derive(Debug, PartialEq)]
pub struct RootMotionTrack {
    times: Box<[f32]>,
    values: Box<[[f32; 4]]>,
    interpolation: Interpolation,
    kind: Kind,
}
impl RootMotionTrack {
    pub fn new(
        times: Vec<f32>,
        values: Vec<[f32; 4]>,
        interpolation: Interpolation,
        kind: Kind,
    ) -> Result<Self, RootMotionError> {
        Track::new(&times, &values, interpolation, kind)?;
        Ok(Self {
            times: times.into_boxed_slice(),
            values: values.into_boxed_slice(),
            interpolation,
            kind,
        })
    }
    pub fn times(&self) -> &[f32] {
        &self.times
    }
    pub fn values(&self) -> &[[f32; 4]] {
        &self.values
    }
    pub fn interpolation(&self) -> Interpolation {
        self.interpolation
    }
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn view(&self) -> Track<'_> {
        Track {
            times: &self.times,
            values: &self.values,
            interpolation: self.interpolation,
            kind: self.kind,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct RootMotionClip {
    duration: f32,
    translation: Option<RootMotionTrack>,
    rotation: Option<RootMotionTrack>,
}
impl RootMotionClip {
    pub fn new(
        duration: f32,
        translation: Option<RootMotionTrack>,
        rotation: Option<RootMotionTrack>,
    ) -> Result<Self, RootMotionError> {
        if duration.to_bits() == (-0.0f32).to_bits() {
            return Err(RootMotionError::Duration);
        }
        // Check properties, range and endpoint admission before publication.
        RootMotionSampler::new(
            f64::from(duration),
            [0.; 3],
            [0., 0., 0., 1.],
            translation.as_ref().map(RootMotionTrack::view),
            rotation.as_ref().map(RootMotionTrack::view),
        )?;
        Ok(Self {
            duration,
            translation,
            rotation,
        })
    }
    pub fn duration(&self) -> f32 {
        self.duration
    }
    pub fn translation(&self) -> Option<&RootMotionTrack> {
        self.translation.as_ref()
    }
    pub fn rotation(&self) -> Option<&RootMotionTrack> {
        self.rotation.as_ref()
    }
}

#[derive(Debug, PartialEq)]
struct PreparedClip {
    clip: RootMotionClip,
    prepared: PreparedRootMotion,
}

/// Content identity/topology are sealed by the cooker; this constructor checks
/// numerical and capacity invariants, without pretending to load the model.
#[derive(Debug, PartialEq)]
pub struct RootMotionCatalog {
    fingerprint: [u8; 32],
    root: u16,
    rest_translation: [f32; 3],
    rest_rotation: [f32; 4],
    model_reference: RootMotionDelta,
    clips: Box<[PreparedClip]>,
    retained_bytes: usize,
}
impl RootMotionCatalog {
    pub fn new(
        fingerprint: [u8; 32],
        root: u16,
        rest_translation: [f32; 3],
        rest_rotation: [f32; 4],
        model_reference: RootMotionDelta,
        clips: Vec<RootMotionClip>,
    ) -> Result<Self, RootMotionError> {
        if root >= 4096 || clips.len() > MAX_ROOT_MOTION_CLIPS {
            return Err(RootMotionError::Capacity("root node or clips"));
        }
        let norm = model_reference.rotation.iter().map(|v| v * v).sum::<f64>();
        if !model_reference.translation.iter().all(|v| v.is_finite())
            || !model_reference.rotation.iter().all(|v| v.is_finite())
            || (norm - 1.).abs() > 1e-9
        {
            return Err(RootMotionError::Track("model root reference"));
        }
        RootMotionSampler::new(0., rest_translation, rest_rotation, None, None)?;
        let mut retained_bytes = size_of::<Self>();
        charge(&mut retained_bytes, clips.len(), size_of::<PreparedClip>())?;
        let mut keys = 0usize;
        for clip in &clips {
            for track in [clip.translation(), clip.rotation()].into_iter().flatten() {
                keys = keys
                    .checked_add(track.times.len())
                    .filter(|n| *n <= MAX_ROOT_MOTION_KEYS)
                    .ok_or(RootMotionError::Capacity("total root keys"))?;
                charge(&mut retained_bytes, track.times.len(), size_of::<f32>())?;
                charge(
                    &mut retained_bytes,
                    track.values.len(),
                    size_of::<[f32; 4]>(),
                )?;
            }
        }
        let mut prepared = Vec::with_capacity(clips.len());
        for clip in clips {
            let sampler = RootMotionSampler::new(
                f64::from(clip.duration),
                rest_translation,
                rest_rotation,
                clip.translation().map(RootMotionTrack::view),
                clip.rotation().map(RootMotionTrack::view),
            )?;
            let endpoints = sampler.prepared;
            prepared.push(PreparedClip {
                clip,
                prepared: endpoints,
            });
        }
        Ok(Self {
            fingerprint,
            root,
            rest_translation,
            rest_rotation,
            model_reference,
            clips: prepared.into_boxed_slice(),
            retained_bytes,
        })
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn root(&self) -> u16 {
        self.root
    }
    pub fn rest_translation(&self) -> [f32; 3] {
        self.rest_translation
    }
    pub fn rest_rotation(&self) -> [f32; 4] {
        self.rest_rotation
    }
    pub fn model_reference(&self) -> RootMotionDelta {
        self.model_reference
    }
    pub fn clips(&self) -> impl ExactSizeIterator<Item = &RootMotionClip> {
        self.clips.iter().map(|clip| &clip.clip)
    }
    pub fn sampler(&self, index: usize) -> Option<RootMotionSampler<'_>> {
        let entry = self.clips.get(index)?;
        Some(RootMotionSampler {
            translation: entry.clip.translation().map(RootMotionTrack::view),
            rotation: entry.clip.rotation().map(RootMotionTrack::view),
            prepared: entry.prepared,
        })
    }
    /// Exact owned structs/arrays, excluding allocation/Arc headers and any
    /// caller-owned source inputs. No spare Vec capacity survives admission.
    pub fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

fn charge(total: &mut usize, count: usize, stride: usize) -> Result<(), RootMotionError> {
    *total = count
        .checked_mul(stride)
        .and_then(|n| total.checked_add(n))
        .filter(|n| *n <= MAX_ROOT_MOTION_CATALOG_BYTES)
        .ok_or(RootMotionError::Capacity("retained catalog bytes"))?;
    Ok(())
}
