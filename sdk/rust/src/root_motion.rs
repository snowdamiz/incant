//! Complete copied pose → root stripping → ordered IK, with exact motion receipts.
//! The model fingerprint/root pair identifies the cooked rest transform to strip.
use crate::{
    AnimationIkConstraint, AnimationIkSnapshotV2, AnimationPoseSnapshotV5, EntityId, Error, Result,
};
pub use abi::RootMotionPolicy;
use incant_wasm_abi as abi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootMotionClockOwner {
    Rest,
    Clip(u16),
    State(EntityId),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootMotionClock {
    pub owner: RootMotionClockOwner,
    /// Exact signed cycle provenance, never rounded through a floating-point value.
    pub cycle: i64,
    pub seconds: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootMotionDelta {
    pub translation: [f64; 3],
    /// Unit XYZW quaternion. Compose on the right of the previous rigid frame.
    pub rotation: [f64; 4],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootMotionPlayback {
    pub entity: EntityId,
    pub fingerprint: [u8; 32],
    pub root: u16,
    pub policy: RootMotionPolicy,
    /// Controls body movement only. Root stripping remains active when false.
    pub applying: bool,
    pub tick: u64,
    pub current: RootMotionClock,
    pub outgoing: Option<RootMotionClock>,
    pub requested: RootMotionDelta,
    pub applied: RootMotionDelta,
    pub accumulated_requested: RootMotionDelta,
    pub accumulated_applied: RootMotionDelta,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootMotionControl {
    SetApplying(bool),
    ResetAccumulated,
}
#[derive(Clone, Copy)]
pub struct RootMotionSnapshot<'a> {
    ik: AnimationIkSnapshotV2<'a>,
    rows: &'a [u8],
}
impl<'a> RootMotionSnapshot<'a> {
    /// Validate every nested section and owner association before exposing a row.
    /// Unaligned byte slices are valid; parsing allocates nothing.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let h = abi::RootMotionHeader::decode(bytes).ok_or(Error::Malformed)?;
        if &h.magic != abi::ROOT_MOTION_MAGIC
            || h.reserved != 0
            || h.pose_bytes as usize > abi::ANIMATION_IK_V2_MAX_BYTES
            || h.count as usize > abi::ROOT_MOTION_MAX_ROWS
            || h.stride as usize != abi::RootMotionRow::SIZE
        {
            return Err(Error::Malformed);
        }
        let end = abi::RootMotionHeader::SIZE + h.pose_bytes as usize;
        if bytes.len() != end + h.count as usize * abi::RootMotionRow::SIZE {
            return Err(Error::Malformed);
        }
        let ik = AnimationIkSnapshotV2::parse(&bytes[abi::RootMotionHeader::SIZE..end])?;
        if ik.poses().scene().0 != h.scene {
            return Err(Error::Malformed);
        }
        let rows = &bytes[end..];
        let mut owners = ik.poses().iter().peekable();
        let mut previous = None;
        for bytes in rows.as_chunks::<{ abi::RootMotionRow::SIZE }>().0 {
            let r = abi::RootMotionRow::decode(bytes).ok_or(Error::Malformed)?;
            validate(r)?;
            if previous.is_some_and(|id| id >= r.entity) {
                return Err(Error::Malformed);
            }
            while owners.peek().is_some_and(|p| p.entity.0 < r.entity) {
                owners.next();
            }
            if owners.peek().is_none_or(|p| p.entity.0 != r.entity) {
                return Err(Error::Malformed);
            }
            previous = Some(r.entity);
        }
        Ok(Self { ik, rows })
    }
    pub fn scene(&self) -> EntityId {
        self.ik.poses().scene()
    }
    /// Sample the full native pose first, retaining each group's interpolation.
    pub fn poses(&self) -> AnimationPoseSnapshotV5<'a> {
        self.ik.poses()
    }
    /// Apply these in order AFTER stripping every selected root's sampled
    /// translation/rotation to cooked rest; sampled scale remains unchanged.
    pub fn constraints(&self) -> impl ExactSizeIterator<Item = AnimationIkConstraint> + 'a {
        self.ik.constraints()
    }
    pub fn len(&self) -> usize {
        self.rows.len() / abi::RootMotionRow::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = RootMotionPlayback> + 'a {
        self.rows
            .as_chunks::<{ abi::RootMotionRow::SIZE }>()
            .0
            .iter()
            .map(|bytes| {
                let r = abi::RootMotionRow::decode(bytes).expect("validated root-motion row");
                RootMotionPlayback {
                    entity: EntityId(r.entity),
                    fingerprint: r.fingerprint,
                    root: r.root as u16,
                    policy: policy(r.policy).expect("validated root-motion policy"),
                    applying: r.flags & 1 != 0,
                    tick: r.tick,
                    current: clock(
                        r.current_kind,
                        r.current_clip,
                        r.current_state,
                        r.current_cycle,
                        r.current_seconds,
                    )
                    .expect("validated current clock"),
                    outgoing: (r.flags & 2 != 0).then(|| {
                        clock(
                            r.outgoing_kind,
                            r.outgoing_clip,
                            r.outgoing_state,
                            r.outgoing_cycle,
                            r.outgoing_seconds,
                        )
                        .expect("validated outgoing clock")
                    }),
                    requested: delta(r.requested),
                    applied: delta(r.applied),
                    accumulated_requested: delta(r.accumulated_requested),
                    accumulated_applied: delta(r.accumulated_applied),
                }
            })
    }
}
fn delta(v: [f64; 7]) -> RootMotionDelta {
    RootMotionDelta {
        translation: [v[0], v[1], v[2]],
        rotation: [v[3], v[4], v[5], v[6]],
    }
}
fn policy(v: u32) -> Result<RootMotionPolicy> {
    u8::try_from(v)
        .ok()
        .and_then(RootMotionPolicy::from_tag)
        .ok_or(Error::Malformed)
}
fn clock(
    kind: u32,
    clip: u32,
    state: [u8; 16],
    cycle: i64,
    seconds: f64,
) -> Result<RootMotionClock> {
    if !seconds.is_finite() || !(0.0..=abi::ROOT_MOTION_MAX_SECONDS as f64).contains(&seconds) {
        return Err(Error::Malformed);
    }
    let owner = match kind {
        0 if clip == u32::MAX && state == [0; 16] && cycle == 0 && seconds == 0.0 => {
            RootMotionClockOwner::Rest
        }
        1 if (clip as usize) < abi::ANIMATION_MAX_CLIPS && state == [0; 16] => {
            RootMotionClockOwner::Clip(clip as u16)
        }
        2 if clip == u32::MAX => RootMotionClockOwner::State(EntityId(state)),
        _ => return Err(Error::Malformed),
    };
    Ok(RootMotionClock {
        owner,
        cycle,
        seconds,
    })
}
fn validate(r: abi::RootMotionRow) -> Result<()> {
    if r.reserved != 0
        || r.flags & !3 != 0
        || r.root as usize >= abi::ROOT_MOTION_MAX_MODEL_NODES
        || !r.is_finite()
    {
        return Err(Error::Malformed);
    }
    policy(r.policy)?;
    clock(
        r.current_kind,
        r.current_clip,
        r.current_state,
        r.current_cycle,
        r.current_seconds,
    )?;
    if r.flags & 2 != 0 {
        clock(
            r.outgoing_kind,
            r.outgoing_clip,
            r.outgoing_state,
            r.outgoing_cycle,
            r.outgoing_seconds,
        )?;
    } else if r.outgoing_kind != 0
        || r.outgoing_clip != 0
        || r.outgoing_state != [0; 16]
        || r.outgoing_cycle != 0
        || r.outgoing_seconds.to_bits() != 0
    {
        return Err(Error::Malformed);
    }
    for v in [
        r.requested,
        r.applied,
        r.accumulated_requested,
        r.accumulated_applied,
    ] {
        let norm = v[3..].iter().map(|n| n * n).sum::<f64>();
        if (norm - 1.).abs() > 1e-9 {
            return Err(Error::Malformed);
        }
    }
    Ok(())
}
