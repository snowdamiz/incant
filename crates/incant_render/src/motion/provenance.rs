//! Exact retained placement identities and accepted changes. No pixel history or
//! owner texture is introduced here. Integer words are ready for a separately
//! reviewed owner attachment; existing motion/depth shaders ignore their lanes.
use super::{MotionError, MotionFrame, plan};
use glam::DMat4;

pub(super) const CHANGED: u32 = 1 << 31;
const MAX: usize = incant_render_data::MAX_INSTANCES;

/// Read-only changes for one accepted frame. IDs only have meaning in this
/// frame's MotionState owner/epoch; equal numbers from another owner do not match.
/// Clear (0) and IDs above `highest_owner` are not placements.
pub struct MotionChanges<'a> {
    pub(super) frame: MotionFrame,
    pub(super) highest: u32,
    pub(super) bits: &'a [u32],
    pub(super) count: u32,
    pub(super) classes: u32,
}
impl MotionChanges<'_> {
    pub fn frame(&self) -> &MotionFrame {
        &self.frame
    }
    pub fn highest_owner(&self) -> u32 {
        self.highest
    }
    pub fn changed_count(&self) -> u32 {
        self.count
    }
    /// Conservative class-membership union, never a collision-free identity.
    pub fn changed_classes(&self) -> u32 {
        self.classes
    }
    pub fn changed(&self, id: u32) -> Option<bool> {
        (id != 0 && id <= self.highest).then(|| self.bits[id as usize / 32] & (1 << (id % 32)) != 0)
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Key(pub [usize; 4]);
#[derive(Clone, Copy)]
pub(super) struct Group {
    pub key: Key,
    pub batch: usize,
    pub count: usize,
}
#[derive(Clone, Copy, Default)]
pub(super) struct Base {
    pub leader: usize,
    pub first: u32,
}
/// All temporary grouping and both retained bitsets are charged before reserve.
/// Includes the worst case before duplicate mirrored rows have been eliminated.
pub(super) fn charge(bytes: &mut u64, batches: usize, slots: usize) -> Result<(), MotionError> {
    plan::charge(bytes, batches, size_of::<Group>() + size_of::<Base>())?;
    plan::charge(
        bytes,
        2 * (slots.min(MAX) + 1).div_ceil(32),
        size_of::<u32>(),
    )
}
/// Sort only temporary descriptors. IDs follow the first batch occurrence, not
/// the allocator address or sort order; twins receive the same contiguous range.
pub(super) fn assign(
    groups: &mut [Group],
    batch_count: usize,
) -> Result<(Vec<Base>, u32), MotionError> {
    groups.sort_unstable_by_key(|g| (g.key, g.batch));
    let mut bases = vec![Base::default(); batch_count];
    let mut previous: Option<Group> = None;
    for &g in groups.iter() {
        if g.count == 0 || g.batch >= batch_count {
            return Err(MotionError::Topology);
        }
        let leader = match previous {
            Some(old) if old.key == g.key => {
                if old.count != g.count {
                    return Err(MotionError::Topology);
                }
                bases[old.batch].leader
            }
            _ => g.batch,
        };
        bases[g.batch] = Base { leader, first: 0 };
        previous = Some(g);
    }
    // Sorting by batch restores the immutable draw order. No auxiliary map or
    // per-frame allocation and no address-derived ID ever escapes this startup.
    groups.sort_unstable_by_key(|g| g.batch);
    let mut total = 0usize;
    for g in groups {
        let leader = bases[g.batch].leader;
        if leader == g.batch {
            let first = total.checked_add(1).ok_or(MotionError::Capacity)?;
            total = total
                .checked_add(g.count)
                .filter(|n| *n <= MAX)
                .ok_or(MotionError::Capacity)?;
            bases[g.batch].first = first as u32;
        } else {
            if leader >= g.batch || bases[leader].first == 0 {
                return Err(MotionError::Topology);
            }
            bases[g.batch].first = bases[leader].first;
        }
    }
    Ok((bases, total as u32))
}
pub(super) fn same_matrix(a: DMat4, b: DMat4) -> bool {
    a.to_cols_array().map(f64::to_bits) == b.to_cols_array().map(f64::to_bits)
}
#[derive(Default)]
struct Bits {
    words: Vec<u32>,
    count: u32,
    classes: u32,
}
impl Bits {
    fn new(count: u32) -> Self {
        Self {
            words: vec![
                0;
                if count == 0 {
                    0
                } else {
                    (count as usize + 1).div_ceil(32)
                }
            ],
            ..Self::default()
        }
    }
    fn clear(&mut self) {
        self.words.fill(0);
        self.count = 0;
        self.classes = 0;
    }
    fn mark(&mut self, id: u32) {
        let bit = 1 << (id % 32);
        let row = &mut self.words[id as usize / 32];
        if *row & bit == 0 {
            *row |= bit;
            self.count += 1;
            self.classes |= 1 << (id % 31);
        }
    }
}
pub(super) struct State {
    highest: u32,
    accepted: Bits,
    candidate: Bits,
}
impl State {
    pub fn new(highest: u32) -> Self {
        Self {
            highest,
            accepted: Bits::new(highest),
            candidate: Bits::new(highest),
        }
    }
    pub fn begin(&mut self) {
        self.candidate.clear();
    }
    /// Deduplicates twins. ID0 denotes excluded BLEND geometry, not an owner.
    pub fn mark(&mut self, id: u32, changed: bool) -> u32 {
        debug_assert!(id <= self.highest);
        if id == 0 {
            return 0;
        }
        if changed {
            self.candidate.mark(id);
        }
        id | if changed { CHANGED } else { 0 }
    }
    /// Called only with the same prepare/write/submit transaction; candidates
    /// never escape the retained renderer, and failed preparation only changes
    /// private scratch. Borrowed public views expose accepted storage exclusively.
    pub fn accept(&mut self) {
        std::mem::swap(&mut self.accepted, &mut self.candidate);
    }
    pub fn view(&self, frame: MotionFrame) -> MotionChanges<'_> {
        MotionChanges {
            frame,
            highest: self.highest,
            bits: &self.accepted.words,
            count: self.accepted.count,
            classes: self.accepted.classes,
        }
    }
}
#[cfg(test)]
#[path = "provenance_tests.rs"]
mod tests;
