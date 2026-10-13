use crate::{
    Budget, BvhRecord, Dimension, Error, MAX_DEPTH, MAX_ITEMS, Node, Result, ValidatedBvh, Wide,
    records::lengths, vector,
};

fn bounds(node: Node, dimension: Dimension) -> Result<()> {
    if dimension == Dimension::Planar && (node.mins[2] != 0 || node.maxs[2] != 0) {
        return Err(Error::Bounds);
    }
    for axis in 0..dimension as usize {
        let lo = f32::from_bits(node.mins[axis]);
        let hi = f32::from_bits(node.maxs[axis]);
        // This admission bound keeps common f32 BVH area arithmetic finite.
        // It is a format limit, not a promise to save arbitrary finite worlds.
        if !lo.is_finite() || !hi.is_finite() || lo > hi || lo.abs() > 1e9 || hi.abs() > 1e9 {
            return Err(Error::Bounds);
        }
    }
    if node.flags() == 2 || node.leaf_count() == 0 || node.leaf_count() as usize > MAX_ITEMS {
        return Err(Error::Metadata);
    }
    Ok(())
}

fn contains(parent: Node, child: Node, dimension: Dimension) -> bool {
    (0..dimension as usize).all(|axis| {
        f32::from_bits(parent.mins[axis]) <= f32::from_bits(child.mins[axis])
            && f32::from_bits(parent.maxs[axis]) >= f32::from_bits(child.maxs[axis])
    })
}

/// Validate only inert data, with iterative bounded traversal. Transient trees
/// with unpropagated counts, flags or bounds are rejected, never silently refit.
/// Pending flags (3) are retained only with exact ancestor OR propagation.
pub fn validate(record: BvhRecord, budget: &mut Budget) -> Result<ValidatedBvh> {
    structure(&record, budget, None)?;
    Ok(ValidatedBvh { record })
}

/// Private structural traversal; only the distinct pending-tree constructor can
/// provide its independently checked ancestor closure. No query-ready witness
/// is returned for that path.
pub(crate) fn structure(
    record: &BvhRecord,
    budget: &mut Budget,
    pending: Option<&[u8]>,
) -> Result<()> {
    let n = record.nodes.len();
    let slots = record.leaves.slots.len();
    lengths(
        n,
        record.parents.len(),
        slots,
        record.leaves.occupied,
        record.free.len(),
    )?;
    if record.rebuild_start as usize > MAX_ITEMS {
        return Err(Error::Metadata);
    }
    if record.leaves.slots.iter().filter(|s| s.is_some()).count() != record.leaves.occupied as usize
    {
        return Err(Error::Leaf);
    }
    if n == 0 {
        if !record.free.is_empty() {
            return Err(Error::Free);
        }
        return Ok(());
    }
    if record.parents[0] != 0 {
        return Err(Error::Parent);
    }
    // One byte per slot + leaf and a bounded (node,depth) work stack.
    budget.charge(
        n.checked_mul(1 + size_of::<(u32, u32)>())
            .and_then(|v| v.checked_add(slots))
            .ok_or(Error::Limit)?,
    )?;
    let mut marks = vector::<u8>(n)?;
    marks.resize(n, 0);
    let mut leaves = vector::<u8>(slots)?;
    leaves.resize(slots, 0);
    let mut stack = vector::<(u32, u32)>(n)?;
    for &id in &record.free {
        let index = id as usize;
        if index == 0 || index >= n || marks[index] != 0 || record.nodes[index] != Wide::default() {
            return Err(Error::Free);
        }
        marks[index] = 2;
    }
    // Even stale free-slot parent values are bounded scalar indices; they are
    // not interpreted as graph edges until the slot is reused and overwritten.
    if record.parents.iter().any(|id| *id as usize >= n * 2) {
        return Err(Error::Parent);
    }
    marks[0] = 1;
    stack.push((0, 1));
    let single = record.leaves.occupied == 1;
    if single && (n != 1 || !record.free.is_empty() || record.nodes[0].right != Node::default()) {
        return Err(Error::Node);
    }
    let mut found = 0u32;
    while let Some((id, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            return Err(Error::Depth);
        }
        let wide = record.nodes[id as usize];
        for (side, node) in [wide.left, wide.right].into_iter().enumerate() {
            if single && side == 1 {
                continue;
            }
            bounds(node, record.identity.dimension)?;
            let packed = id * 2 + side as u32;
            if node.leaf_count() == 1 {
                if let Some(pending) = pending
                    && (node.flags() == 3) != (pending[packed as usize] & 2 != 0)
                {
                    return Err(Error::Metadata);
                }
                let key = node.children as usize;
                if key >= slots || leaves[key] != 0 || record.leaves.slots[key] != Some(packed) {
                    return Err(Error::Leaf);
                }
                leaves[key] = 1;
                found += 1;
            } else {
                let child_id = node.children as usize;
                if child_id >= n || marks[child_id] != 0 {
                    return Err(Error::Ownership);
                }
                if record.parents[child_id] != packed {
                    return Err(Error::Parent);
                }
                let child = record.nodes[child_id];
                bounds(child.left, record.identity.dimension)?;
                bounds(child.right, record.identity.dimension)?;
                if node.leaf_count() != child.left.leaf_count() + child.right.leaf_count()
                    || (pending.is_none_or(|p| p[packed as usize] == 0)
                        && node.flags() != child.left.flags() | child.right.flags())
                {
                    return Err(Error::Metadata);
                }
                if (pending.is_none_or(|p| p[child_id * 2] == 0)
                    && !contains(node, child.left, record.identity.dimension))
                    || (pending.is_none_or(|p| p[child_id * 2 + 1] == 0)
                        && !contains(node, child.right, record.identity.dimension))
                {
                    return Err(Error::Bounds);
                }
                marks[child_id] = 1;
                stack.push((node.children, depth + 1));
            }
        }
    }
    if found != record.leaves.occupied
        || marks.contains(&0)
        || record
            .leaves
            .slots
            .iter()
            .zip(leaves)
            .any(|(slot, seen)| slot.is_some() != (seen == 1))
    {
        return Err(Error::Ownership);
    }
    Ok(())
}
