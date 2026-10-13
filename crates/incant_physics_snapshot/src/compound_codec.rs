//! Version-two geometry extension; primitive/declaration-only v1 stays stable.
use crate::{
    Budget, CompoundAabb, CompoundRecord, Error, MAX_BYTES, Result, ValidatedCompound,
    ValidatedGeometry, attach_compound, codec::Reader, vector,
};
const HEADER: usize = 40;
fn length(children: usize, tree: usize) -> Result<usize> {
    children
        .checked_mul(24)
        .and_then(|n| n.checked_add(HEADER))
        .and_then(|n| n.checked_add(tree))
        .filter(|n| *n <= MAX_BYTES)
        .ok_or(Error::Limit)
}
fn aabb(read: &mut Reader<'_>) -> Result<CompoundAabb> {
    Ok(CompoundAabb {
        mins: [read.word()?, read.word()?, read.word()?],
        maxs: [read.word()?, read.word()?, read.word()?],
    })
}
pub(crate) fn decode(
    bytes: &[u8],
    geometry: ValidatedGeometry,
    budget: &mut Budget,
) -> Result<ValidatedGeometry> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut read = Reader { bytes, position: 0 };
    let flags = read.word()?;
    let pseudo_normals = match read.word()? {
        0 => false,
        1 => true,
        _ => return Err(Error::Geometry),
    };
    let count = read.word()? as usize;
    let tree_length = read.word()? as usize;
    if count != geometry.record().parts.len() || bytes.len() != length(count, tree_length)? {
        return Err(Error::Length);
    }
    let root_aabb = aabb(&mut read)?;
    budget.charge(
        count
            .checked_mul(size_of::<CompoundAabb>())
            .ok_or(Error::Limit)?,
    )?;
    let mut child_aabbs = vector(count)?;
    for _ in 0..count {
        child_aabbs.push(aabb(&mut read)?);
    }
    let identity = geometry.record().identity;
    let tree = crate::codec::decode_record(&bytes[read.position..], identity, budget)?;
    attach_compound(
        geometry,
        CompoundRecord {
            identity,
            child_aabbs,
            root_aabb,
            tree,
            flags,
            pseudo_normals,
        },
        budget,
    )
}
pub(crate) fn encode(value: ValidatedCompound<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let tree = crate::encode(value.tree(), budget)?;
    let len = length(value.child_aabbs().len(), tree.len())?;
    budget.charge(len)?;
    let mut bytes = vector(len)?;
    for word in [0, 0, value.child_aabbs().len() as u32, tree.len() as u32] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for aabb in core::iter::once(value.root_aabb()).chain(value.child_aabbs().iter().copied()) {
        for word in aabb.mins.into_iter().chain(aabb.maxs) {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&tree);
    Ok(bytes)
}
