//! Conservative pinned hashbrown 0.17.1 / IndexMap 2.14.2 logical requests.
use super::*;
use crate::utils::hashmap::HashMap;
use core::hash::Hash;
pub(in crate::query::contact_manifolds) trait Key:
    Copy + Eq + Hash
{
    fn words(self) -> [u32; 2];
    fn from_words(words: [u32; 2]) -> ConstructionResult<Self>;
}
impl Key for u32 {
    fn words(self) -> [u32; 2] {
        [self, 0]
    }
    fn from_words(w: [u32; 2]) -> ConstructionResult<Self> {
        if w[1] != 0 {
            Err(ConstructionError::Payload)
        } else {
            Ok(w[0])
        }
    }
}
impl Key for (u32, u32) {
    fn words(self) -> [u32; 2] {
        [self.0, self.1]
    }
    fn from_words(w: [u32; 2]) -> ConstructionResult<Self> {
        Ok((w[0], w[1]))
    }
}
fn buckets(n: usize) -> ConstructionResult<usize> {
    if n == 0 {
        return Ok(0);
    }
    if n < 4 {
        return Ok(4);
    }
    if n < 8 {
        return Ok(8);
    }
    if n < 15 {
        return Ok(16);
    }
    n.checked_mul(8)
        .map(|v| v / 7)
        .and_then(usize::checked_next_power_of_two)
        .ok_or(ConstructionError::Limit)
}
fn table<T>(n: usize) -> ConstructionResult<AllocationPlan> {
    if n == 0 {
        return Ok(AllocationPlan::default());
    }
    let buckets = buckets(n)?;
    let align = align_of::<T>().max(16);
    let data = size_of::<T>()
        .checked_mul(buckets)
        .and_then(|v| v.checked_add(align - 1))
        .map(|v| v & !(align - 1))
        .ok_or(ConstructionError::Limit)?;
    let bytes = data
        .checked_add(buckets)
        .and_then(|v| v.checked_add(16))
        .filter(|v| *v <= isize::MAX as usize - (align - 1))
        .ok_or(ConstructionError::Limit)?;
    AllocationPlan::array::<u8>(bytes)
}
pub(in crate::query::contact_manifolds) fn plan<K, V>(
    n: usize,
) -> ConstructionResult<AllocationPlan> {
    if n > s::MAX_ITEMS || !matches!(size_of::<usize>(), 4 | 8) {
        return Err(ConstructionError::Limit);
    }
    #[cfg(feature = "enhanced-determinism")]
    {
        let mut plan = table::<usize>(n)?;
        let a = align_of::<usize>()
            .max(align_of::<K>())
            .max(align_of::<V>());
        let round = |v: usize| (v + a - 1) & !(a - 1);
        let stride = round(size_of::<usize>()) + round(size_of::<K>()) + round(size_of::<V>());
        plan.append(AllocationPlan::array::<u8>(
            stride.checked_mul(n).ok_or(ConstructionError::Limit)?,
        )?)?;
        Ok(plan)
    }
    #[cfg(not(feature = "enhanced-determinism"))]
    {
        table::<(K, V)>(n)
    }
}
pub(in crate::query::contact_manifolds) fn capture<K: Key, V>(
    map: &HashMap<K, V>,
    timestamp: bool,
    kind: s::WorkspaceKind,
    get: impl Fn(&V) -> (usize, bool),
    budget: &mut s::Budget,
) -> s::Result<s::WorkspaceRecord> {
    if map.len() > s::MAX_ITEMS {
        return Err(s::Error::Limit);
    }
    let mut detectors = budget.vector(map.len())?;
    for (key, v) in map {
        let (manifold, timestamp) = get(v);
        detectors.push(s::SubDetector {
            key: key.words(),
            manifold: manifold.try_into().map_err(|_| s::Error::Limit)?,
            timestamp,
        });
    }
    #[cfg(not(feature = "enhanced-determinism"))]
    detectors.sort_unstable_by_key(|d| d.key);
    Ok(s::WorkspaceRecord {
        kind,
        timestamp,
        detectors,
    })
}
pub(in crate::query::contact_manifolds) fn construct<K: Key, V>(
    rows: &[s::SubDetector],
    make: impl Fn(&s::SubDetector) -> V,
    permit: &mut AllocationPermit,
) -> ConstructionResult<HashMap<K, V>> {
    permit.charge(plan::<K, V>(rows.len())?)?;
    let mut map = HashMap::default();
    #[cfg(feature = "enhanced-determinism")]
    map.try_reserve_exact(rows.len())
        .map_err(|_| ConstructionError::Allocation)?;
    #[cfg(not(feature = "enhanced-determinism"))]
    map.try_reserve(rows.len())
        .map_err(|_| ConstructionError::Allocation)?;
    #[cfg(feature = "enhanced-determinism")]
    let expected = rows.len();
    #[cfg(not(feature = "enhanced-determinism"))]
    let expected = match buckets(rows.len())? {
        0 => 0,
        b if b < 8 => b - 1,
        b => b / 8 * 7,
    };
    if map.capacity() != expected {
        return Err(ConstructionError::Allocation);
    }
    for row in rows {
        if map.insert(K::from_words(row.key)?, make(row)).is_some() {
            return Err(ConstructionError::Payload);
        }
    }
    Ok(map)
}
