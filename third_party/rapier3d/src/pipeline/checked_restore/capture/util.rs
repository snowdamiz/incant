//! Preflight helpers shared by bounded native observations.
use alloc::vec::Vec;
use incant_physics_snapshot::{self as s, Budget, Error, MAX_ITEMS};
pub(crate) fn count(n: usize) -> s::Result<u32> {
    if n > MAX_ITEMS {
        Err(Error::Limit)
    } else {
        Ok(n as u32)
    }
}
pub(crate) fn total(mut values: impl Iterator<Item = usize>) -> s::Result<usize> {
    values.try_fold(0usize, |n, x| {
        n.checked_add(x)
            .filter(|v| *v <= MAX_ITEMS)
            .ok_or(Error::Limit)
    })
}
pub(crate) fn map<T, U>(
    values: &[T],
    budget: &mut Budget,
    f: impl FnMut(&T) -> U,
) -> s::Result<Vec<U>> {
    count(values.len())?;
    let mut out = budget.vector(values.len())?;
    out.extend(values.iter().map(f));
    Ok(out)
}
pub(crate) fn copy<T: Copy>(values: &[T], budget: &mut Budget) -> s::Result<Vec<T>> {
    map(values, budget, |x| *x)
}
pub(crate) fn handle(parts: (u32, u32)) -> s::Handle {
    s::Handle {
        slot: parts.0,
        generation: parts.1,
    }
}
