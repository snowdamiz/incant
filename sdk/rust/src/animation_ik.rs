//! Complete v4 local poses followed by ordered model-space IK constraints.
//! A copied frame owns neither native pointers nor authority to change a chain.
use crate::{AnimationPoseSnapshot, EntityId, Error, Result};
use incant_wasm_abi as abi;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationIkConstraint {
    pub entity: EntityId,
    pub constraint: EntityId,
    /// Immediate root, middle and tip cooked model-node indices, not entity IDs.
    pub chain: [u16; 3],
    /// Model space before the entity/world transform, in cooked model units.
    pub target: [f64; 3],
    pub pole: [f64; 3],
    pub weight: f64,
}

#[derive(Clone, Copy)]
pub struct AnimationIkSnapshot<'a> {
    poses: AnimationPoseSnapshot<'a>,
    constraints: &'a [u8],
}
impl<'a> AnimationIkSnapshot<'a> {
    /// Validate both complete sections before exposing either. Parsing uses no
    /// allocation and accepts unaligned copied bytes. Owners are sorted; each
    /// owner's constraint order is its native application order, not ID order.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let h = abi::AnimationIkHeader::decode(bytes).ok_or(Error::Malformed)?;
        if &h.magic != abi::ANIMATION_IK_MAGIC
            || h.reserved != 0
            || h.stride as usize != abi::AnimationIkRow::SIZE
            || h.count as usize > abi::ANIMATION_IK_MAX_ROWS
            || h.pose_bytes as usize > abi::ANIMATION_V4_MAX_BYTES
        {
            return Err(Error::Malformed);
        }
        let pose_end = abi::AnimationIkHeader::SIZE + h.pose_bytes as usize;
        let expected = pose_end + h.count as usize * abi::AnimationIkRow::SIZE;
        if bytes.len() != expected {
            return Err(Error::Malformed);
        }
        let poses = AnimationPoseSnapshot::parse(&bytes[abi::AnimationIkHeader::SIZE..pose_end])?;
        let constraints = &bytes[pose_end..];
        let mut pose_owners = poses.iter().peekable();
        let mut owner = None;
        let mut ids = [[0; 16]; abi::ANIMATION_IK_MAX_PER_OWNER];
        let mut count = 0;
        for bytes in constraints.as_chunks::<{ abi::AnimationIkRow::SIZE }>().0 {
            let row = abi::AnimationIkRow::decode(bytes).ok_or(Error::Malformed)?;
            if row.reserved != 0
                || row
                    .chain
                    .iter()
                    .any(|n| *n as usize >= abi::ANIMATION_IK_MAX_MODEL_NODES)
                || row.chain[0] == row.chain[1]
                || row.chain[1] == row.chain[2]
                || row.chain[0] == row.chain[2]
                || !numeric(row.target, row.pole, row.weight)
            {
                return Err(Error::Malformed);
            }
            if owner != Some(row.entity) {
                if owner.is_some_and(|old| old >= row.entity) {
                    return Err(Error::Malformed);
                }
                while pose_owners.peek().is_some_and(|p| p.entity.0 < row.entity) {
                    pose_owners.next();
                }
                if pose_owners.peek().is_none_or(|p| p.entity.0 != row.entity) {
                    return Err(Error::Malformed);
                }
                owner = Some(row.entity);
                count = 0;
            }
            if count == ids.len() || ids[..count].contains(&row.constraint) {
                return Err(Error::Malformed);
            }
            ids[count] = row.constraint;
            count += 1;
        }
        Ok(Self { poses, constraints })
    }
    /// Ordinary pose operands retain their original SLERP/weighted semantics.
    /// Apply this complete pose first, then this frame's ordered constraints.
    pub fn poses(&self) -> AnimationPoseSnapshot<'a> {
        self.poses
    }
    pub fn constraints(&self) -> impl ExactSizeIterator<Item = AnimationIkConstraint> + 'a {
        self.constraints
            .as_chunks::<{ abi::AnimationIkRow::SIZE }>()
            .0
            .iter()
            .map(|bytes| {
                let r = abi::AnimationIkRow::decode(bytes).expect("validated IK row");
                AnimationIkConstraint {
                    entity: EntityId(r.entity),
                    constraint: EntityId(r.constraint),
                    chain: r.chain.map(|n| n as u16),
                    target: r.target,
                    pole: r.pole,
                    weight: r.weight,
                }
            })
    }
}
pub(crate) fn numeric(target: [f64; 3], pole: [f64; 3], weight: f64) -> bool {
    target
        .iter()
        .chain(&pole)
        .all(|n| n.is_finite() && n.abs() <= abi::ANIMATION_IK_MAX_COORDINATE as f64)
        && weight.is_finite()
        && (0.0..=1.0).contains(&weight)
}
