//! Pinned hashbrown 0.17.1 / indexmap 2.14.2 allocation bounds.
//! These are table/control/entry allocations, not a Vec-only approximation.
use super::*;
pub(super) type Pairs = HashMap<(ColliderHandle, ColliderHandle), u32>;
fn buckets(n: usize) -> ConstructionResult<usize> {
    if n == 0 {
        return Ok(0);
    }
    // Both native payloads are at least four bytes, so the pinned small-table
    // minimum is three entries on every supported 32/64-bit target.
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
    // Pinned SIMD groups are at most 16 bytes (NEON/generic can be smaller).
    // Include control bytes, a full trailing group and worst aligned padding.
    let b = buckets(n)?;
    let align = align_of::<T>().max(16);
    let data = size_of::<T>()
        .checked_mul(b)
        .and_then(|v| v.checked_add(align - 1))
        .map(|v| v & !(align - 1))
        .ok_or(ConstructionError::Limit)?;
    let bytes = data
        .checked_add(b)
        .and_then(|v| v.checked_add(16))
        .filter(|v| *v <= isize::MAX as usize - (align - 1))
        .ok_or(ConstructionError::Limit)?;
    AllocationPlan::array::<u8>(bytes)
}
#[cfg(feature = "enhanced-determinism")]
fn entries(n: usize) -> ConstructionResult<AllocationPlan> {
    // IndexMap Bucket has exactly {HashValue(usize), key, value}. Each field
    // rounded to their maximum alignment bounds every Rust field ordering.
    let a = align_of::<usize>().max(align_of::<(ColliderHandle, ColliderHandle)>());
    let round = |v: usize| (v + a - 1) & !(a - 1);
    let stride = round(size_of::<usize>())
        + round(size_of::<(ColliderHandle, ColliderHandle)>())
        + round(size_of::<u32>());
    AllocationPlan::array::<u8>(stride.checked_mul(n).ok_or(ConstructionError::Limit)?)
}
pub(super) fn plan(n: usize) -> ConstructionResult<AllocationPlan> {
    if !matches!(size_of::<usize>(), 4 | 8) {
        return Err(ConstructionError::Payload);
    }
    #[cfg(feature = "enhanced-determinism")]
    {
        let mut p = table::<usize>(n)?;
        p.append(entries(n)?)?;
        Ok(p)
    }
    #[cfg(not(feature = "enhanced-determinism"))]
    {
        table::<((ColliderHandle, ColliderHandle), u32)>(n)
    }
}
pub(super) fn construct(
    rows: &[incant_physics_snapshot::BroadPair],
    permit: &mut AllocationPermit,
) -> ConstructionResult<Pairs> {
    permit.charge(plan(rows.len())?)?;
    let mut pairs = Pairs::default();
    #[cfg(feature = "enhanced-determinism")]
    pairs
        .try_reserve_exact(rows.len())
        .map_err(|_| ConstructionError::Allocation)?;
    #[cfg(not(feature = "enhanced-determinism"))]
    pairs
        .try_reserve(rows.len())
        .map_err(|_| ConstructionError::Allocation)?;
    #[cfg(feature = "enhanced-determinism")]
    let expected = rows.len();
    #[cfg(not(feature = "enhanced-determinism"))]
    let expected = match buckets(rows.len())? {
        0 => 0,
        b if b < 8 => b - 1,
        b => b / 8 * 7,
    };
    if pairs.capacity() != expected {
        return Err(ConstructionError::Allocation);
    }
    for row in rows {
        let [a, b] = row.colliders.map(handle);
        if pairs.insert((a, b), row.inserted_frame).is_some() {
            return Err(ConstructionError::Payload);
        }
    }
    Ok(pairs)
}
