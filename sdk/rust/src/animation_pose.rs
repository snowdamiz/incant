//! Complete copied local-pose observations. Pairwise SLERP and a normalized
//! weighted quaternion mean have distinct tags and must not be interchanged.
use crate::{AnimationTransition, EntityId, Error, Result};
pub use abi::AnimationPoseKind;
use incant_wasm_abi as abi;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationPoseSample {
    pub clip: Option<u16>,
    pub seconds: f64,
    pub weight: f64,
}

#[derive(Clone, Copy)]
pub struct AnimationPosePlayback<'a> {
    pub entity: EntityId,
    /// Scalar playback sample; only representative for a weighted tree.
    pub clip: Option<u16>,
    pub seconds: f64,
    pub speed: f64,
    pub playing: bool,
    pub looping: bool,
    pub completed: bool,
    pub transition: Option<AnimationTransition>,
    pub kind: AnimationPoseKind,
    samples: &'a [u8],
}
impl<'a> AnimationPosePlayback<'a> {
    /// Full native sample order. SLERP pairs are source then destination;
    /// weighted-mean order is significant for the quaternion reference hemisphere.
    pub fn samples(&self) -> impl ExactSizeIterator<Item = AnimationPoseSample> + 'a {
        self.samples
            .as_chunks::<{ abi::AnimationPoseSample::SIZE }>()
            .0
            .iter()
            .map(|bytes| {
                let r = abi::AnimationPoseSample::decode(bytes).expect("validated pose sample");
                AnimationPoseSample {
                    clip: clip(r.clip),
                    seconds: r.seconds,
                    weight: r.weight,
                }
            })
    }
}

#[derive(Clone, Copy)]
pub struct AnimationPoseSnapshot<'a> {
    scene: EntityId,
    rows: &'a [u8],
    samples: &'a [u8],
}
impl<'a> AnimationPoseSnapshot<'a> {
    /// Validate the complete frame before exposing any row. Parsing allocates
    /// nothing and accepts unaligned copied bytes, never host pointers.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let h = abi::AnimationHeaderV4::decode(bytes).ok_or(Error::Malformed)?;
        if &h.magic != abi::ANIMATION_V4_MAGIC
            || h.stride as usize != abi::AnimationRowV4::SIZE
            || h.sample_stride as usize != abi::AnimationPoseSample::SIZE
            || h.count as usize > abi::ANIMATION_MAX_ROWS
            || h.sample_count as usize > abi::ANIMATION_POSE_MAX_SAMPLE_ROWS
        {
            return Err(Error::Malformed);
        }
        let row_len = h.count as usize * abi::AnimationRowV4::SIZE;
        let sample_len = h.sample_count as usize * abi::AnimationPoseSample::SIZE;
        if bytes.len() != abi::AnimationHeaderV4::SIZE + row_len + sample_len {
            return Err(Error::Malformed);
        }
        let (rows, samples) = bytes[abi::AnimationHeaderV4::SIZE..].split_at(row_len);
        let mut next_sample = 0usize;
        let mut previous = None;
        let mut weighted_rows = 0usize;
        for bytes in rows.as_chunks::<{ abi::AnimationRowV4::SIZE }>().0 {
            let row = abi::AnimationRowV4::decode(bytes).ok_or(Error::Malformed)?;
            let kind = kind(row.pose_kind)?;
            if row.reserved != 0
                || previous.is_some_and(|id| id >= row.id)
                || row.sample_offset as usize != next_sample
                || row.sample_count == 0
                || row.sample_count as usize > abi::ANIMATION_POSE_MAX_SAMPLES
            {
                return Err(Error::Malformed);
            }
            previous = Some(row.id);
            let end = next_sample + row.sample_count as usize;
            if end > h.sample_count as usize {
                return Err(Error::Malformed);
            }
            validate_state(row, kind)?;
            let values = &samples[next_sample * abi::AnimationPoseSample::SIZE
                ..end * abi::AnimationPoseSample::SIZE];
            validate_samples(row, kind, values)?;
            next_sample = end;
            if kind == AnimationPoseKind::WeightedQuaternionMean {
                weighted_rows += 1;
            }
        }
        if next_sample != h.sample_count as usize || weighted_rows > abi::ANIMATION_GRAPH_MAX_ROWS {
            return Err(Error::Malformed);
        }
        Ok(Self {
            scene: EntityId(h.scene),
            rows,
            samples,
        })
    }
    pub fn scene(&self) -> EntityId {
        self.scene
    }
    pub fn len(&self) -> usize {
        self.rows.len() / abi::AnimationRowV4::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = AnimationPosePlayback<'a>> + 'a {
        let samples = self.samples;
        self.rows
            .as_chunks::<{ abi::AnimationRowV4::SIZE }>()
            .0
            .iter()
            .map(move |bytes| {
                let r = abi::AnimationRowV4::decode(bytes).expect("validated pose row");
                let start = r.sample_offset as usize * abi::AnimationPoseSample::SIZE;
                let end = start + r.sample_count as usize * abi::AnimationPoseSample::SIZE;
                AnimationPosePlayback {
                    entity: EntityId(r.id),
                    clip: clip(r.clip),
                    seconds: r.seconds,
                    speed: r.speed,
                    playing: r.flags & 1 != 0,
                    looping: r.flags & 2 != 0,
                    completed: r.flags & 4 != 0,
                    transition: (r.transition_flags & 1 != 0).then_some(AnimationTransition {
                        clip: clip(r.source_clip),
                        seconds: r.source_seconds,
                        speed: r.source_speed,
                        playing: r.transition_flags & 2 != 0,
                        looping: r.transition_flags & 4 != 0,
                        elapsed: r.elapsed,
                        duration: r.duration,
                    }),
                    kind: kind(r.pose_kind).expect("validated pose kind"),
                    samples: &samples[start..end],
                }
            })
    }
}
fn clip(value: u32) -> Option<u16> {
    (value != u32::MAX).then_some(value as u16)
}
fn kind(value: u32) -> Result<AnimationPoseKind> {
    u8::try_from(value)
        .ok()
        .and_then(AnimationPoseKind::from_tag)
        .ok_or(Error::Malformed)
}
fn valid_sample(clip: u32, seconds: f64) -> bool {
    seconds.is_finite()
        && seconds >= 0.0
        && if clip == u32::MAX {
            seconds == 0.0
        } else {
            (clip as usize) < abi::ANIMATION_MAX_CLIPS
        }
}
fn validate_state(r: abi::AnimationRowV4, kind: AnimationPoseKind) -> Result<()> {
    if !r.is_finite()
        || !valid_sample(r.clip, r.seconds)
        || r.speed.abs() > abi::ANIMATION_MAX_SPEED as f64
        || r.flags & !7 != 0
        || (r.flags & 4 != 0 && r.flags & 3 != 0)
        || (kind != AnimationPoseKind::WeightedQuaternionMean
            && r.clip == u32::MAX
            && r.flags & 4 != 0)
    {
        return Err(Error::Malformed);
    }
    // v4 retains the exact scalar transition contract; a weighted tree's full
    // contributions are carried only by its sample table.
    crate::animation::validate_transition(abi::AnimationRowV2 {
        id: r.id,
        clip: r.clip,
        flags: r.flags,
        seconds: r.seconds,
        speed: r.speed,
        source_clip: r.source_clip,
        transition_flags: r.transition_flags,
        source_seconds: r.source_seconds,
        source_speed: r.source_speed,
        elapsed: r.elapsed,
        duration: r.duration,
    })?;
    if (r.transition_flags & 1 != 0) != (kind == AnimationPoseKind::SlerpTransition) {
        return Err(Error::Malformed);
    }
    Ok(())
}
fn validate_samples(r: abi::AnimationRowV4, kind: AnimationPoseKind, bytes: &[u8]) -> Result<()> {
    let mut values = [abi::AnimationPoseSample {
        clip: 0,
        reserved: 0,
        seconds: 0.0,
        weight: 0.0,
    }; abi::ANIMATION_POSE_MAX_SAMPLES];
    let mut sum = 0.0;
    for (index, bytes) in bytes
        .as_chunks::<{ abi::AnimationPoseSample::SIZE }>()
        .0
        .iter()
        .enumerate()
    {
        let value = abi::AnimationPoseSample::decode(bytes).ok_or(Error::Malformed)?;
        if value.reserved != 0
            || !valid_sample(value.clip, value.seconds)
            || !value.weight.is_finite()
            || !(0.0..=1.0).contains(&value.weight)
        {
            return Err(Error::Malformed);
        }
        if kind == AnimationPoseKind::WeightedQuaternionMean
            && (value.weight <= 0.0
                || values[..index]
                    .iter()
                    .any(|v| v.clip == value.clip && v.seconds == value.seconds))
        {
            return Err(Error::Malformed);
        }
        sum += value.weight;
        values[index] = value;
    }
    // Native normalization has at most sixteen finite terms. Preserve its exact
    // f64 weights; tolerate only their bounded summation roundoff.
    if (sum - 1.0).abs() > 16.0 * f64::EPSILON {
        return Err(Error::Malformed);
    }
    let first = values[0];
    let representative = |v: abi::AnimationPoseSample| v.clip == r.clip && v.seconds == r.seconds;
    match kind {
        AnimationPoseKind::Single => {
            if r.sample_count != 1 || first.weight != 1.0 || !representative(first) {
                return Err(Error::Malformed);
            }
        }
        AnimationPoseKind::SlerpTransition | AnimationPoseKind::SlerpParameter => {
            let second = values[1];
            if r.sample_count != 2 || !representative(second) || first.weight != 1.0 - second.weight
            {
                return Err(Error::Malformed);
            }
            if kind == AnimationPoseKind::SlerpTransition {
                if first.clip != r.source_clip
                    || first.seconds != r.source_seconds
                    || second.weight != r.elapsed / r.duration
                {
                    return Err(Error::Malformed);
                }
            } else if first.clip == u32::MAX || second.clip == u32::MAX || second.weight == 0.0 {
                return Err(Error::Malformed);
            }
        }
        AnimationPoseKind::WeightedQuaternionMean => {
            if !representative(first) {
                return Err(Error::Malformed);
            }
        }
    }
    Ok(())
}
