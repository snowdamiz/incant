use crate::{Error, Result};

pub const MAX_ITEMS: usize = 262_144;
pub const MAX_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_ALLOCATION: usize = 64 * 1024 * 1024;
pub const MAX_DEPTH: u32 = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Dimension {
    Planar = 2,
    Spatial = 3,
}

/// The caller supplies the expected pinned source/config identity independently
/// of the input. This inert library does not discover or authorize a build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub dimension: Dimension,
    pub build: [u8; 32],
}

/// Cumulative requested Vec payload budget, including validation scratch/output.
/// Failure does not refund charges. This is not an allocator-overhead/RSS bound.
pub struct Budget {
    remaining: usize,
}
impl Budget {
    pub fn new(bytes: usize) -> Result<Self> {
        if bytes > MAX_ALLOCATION {
            return Err(Error::Limit);
        }
        Ok(Self { remaining: bytes })
    }
    pub fn remaining(&self) -> usize {
        self.remaining
    }
    /// Precharged fallible Vec payload for typed native/outer capture. Logical
    /// bytes exclude allocator overhead and are cumulative even on failure.
    #[cfg(feature = "construction")]
    pub fn vector<T>(&mut self, count: usize) -> Result<Vec<T>> {
        let bytes = count.checked_mul(size_of::<T>()).ok_or(Error::Limit)?;
        if bytes > isize::MAX as usize {
            return Err(Error::Limit);
        }
        self.charge(bytes)?;
        crate::vector(count)
    }

    /// Charge an outer or native capture allocation before requesting it.
    /// The same budget can cover enclosing maps, records, scratch and encoding.
    pub fn charge(&mut self, bytes: usize) -> Result<()> {
        self.remaining = self.remaining.checked_sub(bytes).ok_or(Error::Limit)?;
        Ok(())
    }
}

/// All floats use exact f32 bits; signed zero is retained on active axes.
/// Planar records have canonical zero in the unused Z components.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Node {
    pub mins: [u32; 3],
    pub children: u32,
    pub maxs: [u32; 3],
    pub data: u32,
}
impl Node {
    pub fn leaf_count(self) -> u32 {
        self.data & 0x3fff_ffff
    }
    pub fn flags(self) -> u32 {
        self.data >> 30
    }
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Wide {
    pub left: Node,
    pub right: Node,
}

/// Exact slots, including trailing empty slots, and the separately stored count.
/// Values are packed (wide_index * 2 + right_side) references, not live handles.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct LeafMap {
    pub occupied: u32,
    pub slots: Vec<Option<u32>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BvhRecord {
    pub identity: Identity,
    pub nodes: Vec<Wide>,
    pub parents: Vec<u32>,
    pub leaves: LeafMap,
    /// Preserve reuse order; this is a stack, not an unordered set.
    pub free: Vec<u32>,
    pub rebuild_frame: u32,
    /// May outlive removals; bounded by historical admitted capacity, not the
    /// current leaf count. The solver wraps it on its next optimization call.
    pub rebuild_start: u32,
}

/// Private ownership prevents mutation after validation. The ordinary inert
/// crate has no native dependencies. A separate default-off Parry feature may
/// construct tree storage; that is not native world or physics-save admission.
pub struct ValidatedBvh {
    pub(crate) record: BvhRecord,
}
impl ValidatedBvh {
    pub fn record(&self) -> &BvhRecord {
        &self.record
    }
}

pub(crate) fn lengths(
    nodes: usize,
    parents: usize,
    slots: usize,
    occupied: u32,
    free: usize,
) -> Result<()> {
    if nodes > MAX_ITEMS
        || slots > MAX_ITEMS
        || parents != nodes
        || free > nodes
        || occupied as usize > slots
        || (nodes == 0) != (occupied == 0)
    {
        return Err(Error::Length);
    }
    Ok(())
}

#[cfg(all(test, feature = "construction"))]
mod capture_budget_tests {
    use super::*;
    #[test]
    fn outer_and_native_capture_share_cumulative_precharged_payload() {
        let mut budget = Budget::new(40).unwrap();
        budget.charge(8).unwrap();
        let mut values = budget.vector::<u64>(4).unwrap();
        values.extend([1, 2, 3, 4]);
        assert_eq!(budget.remaining(), 0);
        assert_eq!(values, [1, 2, 3, 4]);
        assert_eq!(budget.charge(1), Err(Error::Limit));
        assert!(budget.vector::<u64>(1).is_err());
        let mut overflow = Budget::new(40).unwrap();
        assert!(overflow.vector::<u64>(usize::MAX).is_err());
        assert_eq!(overflow.remaining(), 40);
    }
}
