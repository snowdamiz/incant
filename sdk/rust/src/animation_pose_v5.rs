//! Complete local pose groups. Evaluate each inner interpolation independently,
//! then the optional outer TRS crossfade; never flatten the two weighted groups.
use crate::{AnimationPoseSample, AnimationTransition, EntityId, Error, Result};
pub use abi::{AnimationPoseKindV5, PoseInterpolation};
use incant_wasm_abi as abi;

#[derive(Clone, Copy)]
pub struct AnimationPoseGroup<'a> {
    pub interpolation: PoseInterpolation,
    bytes: &'a [u8],
}
impl<'a> AnimationPoseGroup<'a> {
    /// Native order is significant, including zero-weight SLERP endpoints.
    pub fn samples(&self) -> impl ExactSizeIterator<Item = AnimationPoseSample> + 'a {
        self.bytes
            .as_chunks::<{ abi::AnimationPoseSample::SIZE }>()
            .0
            .iter()
            .map(|b| {
                let v = abi::AnimationPoseSample::decode(b).expect("validated pose sample");
                AnimationPoseSample {
                    clip: clip(v.clip),
                    seconds: v.seconds,
                    weight: v.weight,
                }
            })
    }
}
#[derive(Clone, Copy)]
pub struct AnimationPosePlaybackV5<'a> {
    pub entity: EntityId,
    /// Representative incoming sample, not a replacement for group evaluation.
    pub clip: Option<u16>,
    pub seconds: f64,
    pub speed: f64,
    pub playing: bool,
    pub looping: bool,
    pub completed: bool,
    pub transition: Option<AnimationTransition>,
    pub kind: AnimationPoseKindV5,
    /// Destination contribution after independently evaluating both groups.
    pub outer_weight: Option<f64>,
    groups: [Option<AnimationPoseGroup<'a>>; 2],
}
impl<'a> AnimationPosePlaybackV5<'a> {
    pub fn group(&self, index: usize) -> Option<AnimationPoseGroup<'a>> {
        self.groups.get(index).copied().flatten()
    }
}
#[derive(Clone, Copy)]
pub struct AnimationPoseSnapshotV5<'a> {
    scene: EntityId,
    rows: &'a [u8],
    samples: &'a [u8],
    compounds: &'a [u8],
}
impl<'a> AnimationPoseSnapshotV5<'a> {
    /// Entire-frame validation precedes every borrowed view, with bounded stack
    /// scratch only. Wire rows may be unaligned; no native handles cross the ABI.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let h = abi::AnimationHeaderV5::decode(bytes).ok_or(Error::Malformed)?;
        if &h.magic != abi::ANIMATION_V5_MAGIC
            || h.stride as usize != abi::AnimationRowV4::SIZE
            || h.sample_stride as usize != abi::AnimationPoseSample::SIZE
            || h.compound_stride as usize != abi::AnimationPoseGroups::SIZE
            || h.count as usize > abi::ANIMATION_MAX_ROWS
            || h.sample_count as usize > abi::ANIMATION_POSE_MAX_SAMPLE_ROWS
            || h.compound_count as usize > abi::ANIMATION_COMPOUND_MAX_ROWS
        {
            return Err(Error::Malformed);
        }
        let row_len = h.count as usize * abi::AnimationRowV4::SIZE;
        let sample_len = h.sample_count as usize * abi::AnimationPoseSample::SIZE;
        let compound_len = h.compound_count as usize * abi::AnimationPoseGroups::SIZE;
        if bytes.len() != abi::AnimationHeaderV5::SIZE + row_len + sample_len + compound_len {
            return Err(Error::Malformed);
        }
        let (rows, rest) = bytes[abi::AnimationHeaderV5::SIZE..].split_at(row_len);
        let (samples, compounds) = rest.split_at(sample_len);
        let mut next_sample = 0;
        let mut compound_iter = compounds
            .as_chunks::<{ abi::AnimationPoseGroups::SIZE }>()
            .0
            .iter();
        let mut previous = None;
        let mut complex_rows = 0;
        for b in rows.as_chunks::<{ abi::AnimationRowV4::SIZE }>().0 {
            let row = abi::AnimationRowV4::decode(b).ok_or(Error::Malformed)?;
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
            let values = &samples[next_sample * abi::AnimationPoseSample::SIZE
                ..end * abi::AnimationPoseSample::SIZE];
            // Scalar clocks retain v4 semantics. Compound observations, like
            // weighted trees, may represent a completed incoming REST state.
            let old_kind = if kind == AnimationPoseKindV5::Compound {
                abi::AnimationPoseKind::WeightedQuaternionMean
            } else {
                abi::AnimationPoseKind::from_tag(kind as u8).ok_or(Error::Malformed)?
            };
            crate::animation_pose::validate_state(row, old_kind)?;
            let representative = if kind == AnimationPoseKindV5::Compound {
                complex_rows += 1;
                let g =
                    abi::AnimationPoseGroups::decode(compound_iter.next().ok_or(Error::Malformed)?)
                        .ok_or(Error::Malformed)?;
                if g.entity != row.id
                    || g.reserved != 0
                    || !g.weight.is_finite()
                    || !(0.0..=1.0).contains(&g.weight)
                    || g.split == 0
                    || g.split > 8
                    || g.split >= row.sample_count
                    || row.sample_count - g.split > 8
                {
                    return Err(Error::Malformed);
                }
                let (a, b) = values.split_at(g.split as usize * abi::AnimationPoseSample::SIZE);
                group(a, interpolation(g.source_kind)?)?;
                group(b, interpolation(g.destination_kind)?)?
            } else {
                let inner = match kind {
                    AnimationPoseKindV5::Single => PoseInterpolation::Clip,
                    AnimationPoseKindV5::SlerpTransition | AnimationPoseKindV5::SlerpParameter => {
                        PoseInterpolation::SphericalPair
                    }
                    AnimationPoseKindV5::WeightedQuaternionMean => {
                        complex_rows += 1;
                        PoseInterpolation::Weighted
                    }
                    AnimationPoseKindV5::Compound => unreachable!(),
                };
                let representative = group(values, inner)?;
                if kind == AnimationPoseKindV5::SlerpTransition {
                    let first = abi::AnimationPoseSample::decode(values).ok_or(Error::Malformed)?;
                    if first.clip != row.source_clip
                        || first.seconds != row.source_seconds
                        || representative.weight != row.elapsed / row.duration
                    {
                        return Err(Error::Malformed);
                    }
                }
                representative
            };
            if representative.clip != row.clip || representative.seconds != row.seconds {
                return Err(Error::Malformed);
            }
            next_sample = end;
        }
        if next_sample != h.sample_count as usize
            || compound_iter.next().is_some()
            || complex_rows > abi::ANIMATION_GRAPH_MAX_ROWS
        {
            return Err(Error::Malformed);
        }
        Ok(Self {
            scene: EntityId(h.scene),
            rows,
            samples,
            compounds,
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
    pub fn iter(&self) -> impl ExactSizeIterator<Item = AnimationPosePlaybackV5<'a>> + 'a {
        let samples = self.samples;
        let mut compounds = self
            .compounds
            .as_chunks::<{ abi::AnimationPoseGroups::SIZE }>()
            .0
            .iter();
        self.rows
            .as_chunks::<{ abi::AnimationRowV4::SIZE }>()
            .0
            .iter()
            .map(move |bytes| {
                let r = abi::AnimationRowV4::decode(bytes).expect("validated pose row");
                let kind = kind(r.pose_kind).expect("validated kind");
                let start = r.sample_offset as usize * abi::AnimationPoseSample::SIZE;
                let values = &samples
                    [start..start + r.sample_count as usize * abi::AnimationPoseSample::SIZE];
                let (groups, outer_weight) = if kind == AnimationPoseKindV5::Compound {
                    let g = abi::AnimationPoseGroups::decode(
                        compounds.next().expect("validated compound"),
                    )
                    .expect("validated descriptor");
                    let (a, b) = values.split_at(g.split as usize * abi::AnimationPoseSample::SIZE);
                    (
                        [
                            Some(AnimationPoseGroup {
                                interpolation: interpolation(g.source_kind).unwrap(),
                                bytes: a,
                            }),
                            Some(AnimationPoseGroup {
                                interpolation: interpolation(g.destination_kind).unwrap(),
                                bytes: b,
                            }),
                        ],
                        Some(g.weight),
                    )
                } else {
                    let interpolation = match kind {
                        AnimationPoseKindV5::Single => PoseInterpolation::Clip,
                        AnimationPoseKindV5::SlerpTransition
                        | AnimationPoseKindV5::SlerpParameter => PoseInterpolation::SphericalPair,
                        _ => PoseInterpolation::Weighted,
                    };
                    (
                        [
                            Some(AnimationPoseGroup {
                                interpolation,
                                bytes: values,
                            }),
                            None,
                        ],
                        None,
                    )
                };
                AnimationPosePlaybackV5 {
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
                    kind,
                    outer_weight,
                    groups,
                }
            })
    }
}
fn clip(value: u32) -> Option<u16> {
    (value != u32::MAX).then_some(value as u16)
}
fn kind(value: u32) -> Result<AnimationPoseKindV5> {
    u8::try_from(value)
        .ok()
        .and_then(AnimationPoseKindV5::from_tag)
        .ok_or(Error::Malformed)
}
fn interpolation(value: u32) -> Result<PoseInterpolation> {
    u8::try_from(value)
        .ok()
        .and_then(PoseInterpolation::from_tag)
        .ok_or(Error::Malformed)
}
fn group(bytes: &[u8], kind: PoseInterpolation) -> Result<abi::AnimationPoseSample> {
    let rows = bytes.as_chunks::<{ abi::AnimationPoseSample::SIZE }>().0;
    if rows.is_empty()
        || rows.len() > abi::ANIMATION_POSE_MAX_SAMPLES
        || (kind == PoseInterpolation::Clip && rows.len() != 1)
        || (kind == PoseInterpolation::SphericalPair && rows.len() != 2)
    {
        return Err(Error::Malformed);
    }
    let mut values = [abi::AnimationPoseSample {
        clip: 0,
        reserved: 0,
        seconds: 0.,
        weight: 0.,
    }; abi::ANIMATION_POSE_MAX_SAMPLES];
    let mut sum = 0.;
    for (n, bytes) in rows.iter().enumerate() {
        let v = abi::AnimationPoseSample::decode(bytes).ok_or(Error::Malformed)?;
        if v.reserved != 0
            || !crate::animation_pose::valid_sample(v.clip, v.seconds)
            || !v.weight.is_finite()
            || !(0.0..=1.0).contains(&v.weight)
            || (kind == PoseInterpolation::Weighted
                && (v.weight <= 0.
                    || values[..n]
                        .iter()
                        .any(|old| old.clip == v.clip && old.seconds == v.seconds)))
        {
            return Err(Error::Malformed);
        }
        sum += v.weight;
        values[n] = v;
    }
    if (sum - 1.).abs() > 16. * f64::EPSILON {
        return Err(Error::Malformed);
    }
    match kind {
        PoseInterpolation::Clip if values[0].weight != 1. => Err(Error::Malformed),
        PoseInterpolation::SphericalPair if values[0].weight != 1. - values[1].weight => {
            Err(Error::Malformed)
        }
        PoseInterpolation::SphericalPair => Ok(values[1]),
        _ => Ok(values[0]),
    }
}
