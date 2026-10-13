//! Default-off exact storage construction from immutable, structurally validated
//! witnesses. This is not a shape-coherence, phase, owner or world-save gate.
use super::{
    Bvh, BvhIncrementalOptimizationState, BvhNode, BvhNodeData, BvhNodeIndex, BvhNodeVec,
    BvhNodeWide,
};
use crate::math::Vector;
use crate::utils::VecMap;
use incant_physics_snapshot::construction::{
    AllocationPermit, AllocationPlan, ConstructionError, ConstructionResult,
};
use incant_physics_snapshot::{BvhRecord, Identity, Node, ValidatedBvh, ValidatedPendingBvh};
#[cfg(not(feature = "f32"))]
compile_error!("checked Incant BVH construction requires the pinned f32 format");

fn check(record: &BvhRecord, expected: Identity) -> ConstructionResult<()> {
    if record.identity != expected || expected.dimension as usize != crate::math::DIM {
        return Err(ConstructionError::Payload);
    }
    Ok(())
}
fn plan(record: &BvhRecord, expected: Identity) -> ConstructionResult<AllocationPlan> {
    check(record, expected)?;
    let mut plan = AllocationPlan::array::<BvhNodeWide>(record.nodes.len())?;
    plan.append(AllocationPlan::array::<BvhNodeIndex>(record.parents.len())?)?;
    plan.append(AllocationPlan::array::<Option<BvhNodeIndex>>(
        record.leaves.slots.len(),
    )?)?;
    plan.append(AllocationPlan::array::<u32>(record.free.len())?)?;
    Ok(plan)
}
fn vector(bits: [u32; 3]) -> Vector {
    #[cfg(feature = "dim2")]
    {
        Vector::new(f32::from_bits(bits[0]), f32::from_bits(bits[1]))
    }
    #[cfg(feature = "dim3")]
    {
        Vector::new(
            f32::from_bits(bits[0]),
            f32::from_bits(bits[1]),
            f32::from_bits(bits[2]),
        )
    }
}
fn node(record: Node) -> BvhNode {
    BvhNode {
        mins: vector(record.mins),
        children: record.children,
        maxs: vector(record.maxs),
        data: BvhNodeData(record.data),
    }
}
fn construct(
    record: &BvhRecord,
    expected: Identity,
    permit: &mut AllocationPermit,
) -> ConstructionResult<Bvh> {
    check(record, expected)?;
    let mut nodes = permit.vector(record.nodes.len())?;
    nodes.extend(record.nodes.iter().map(|w| BvhNodeWide {
        left: node(w.left),
        right: node(w.right),
    }));
    let mut parents = permit.vector(record.parents.len())?;
    parents.extend(record.parents.iter().map(|p| BvhNodeIndex(*p as usize)));
    // Pinned VecMap exposes only infallible capacity allocation. Charge first;
    // allocator OOM may still abort. The temporary insert/remove establishes
    // exact empty tail length without allocating any payload or growing again.
    let count = record.leaves.slots.len();
    permit.charge(AllocationPlan::array::<Option<BvhNodeIndex>>(count)?)?;
    let mut leaves = VecMap::with_capacity(count);
    if leaves.capacity() != count {
        return Err(ConstructionError::Allocation);
    }
    if count != 0 {
        let _ = leaves.insert(count - 1, BvhNodeIndex(0));
        let _ = leaves.remove(count - 1);
    }
    for (slot, value) in record.leaves.slots.iter().enumerate() {
        if let Some(index) = value {
            let _ = leaves.insert(slot, BvhNodeIndex(*index as usize));
        }
    }
    let mut free = permit.vector(record.free.len())?;
    free.extend_from_slice(&record.free);
    Ok(Bvh {
        nodes: BvhNodeVec(nodes),
        parents,
        leaf_node_indices: leaves,
        optimization: BvhIncrementalOptimizationState {
            rebuild_frame_index: record.rebuild_frame,
            rebuild_start_index: record.rebuild_start,
        },
        free_wide_nodes: free,
    })
}
impl Bvh {
    /// Plan exact persistent requests for a strict, immutable BVH witness.
    /// The caller independently supplies the expected pinned build identity.
    pub fn checkpoint_plan(
        witness: &ValidatedBvh,
        expected: Identity,
    ) -> ConstructionResult<AllocationPlan> {
        plan(witness.record(), expected)
    }
    /// Preserve every saved bit and reuse order, consuming the same cumulative
    /// non-Clone permit as other staged owners. Failed staging publishes nothing.
    /// This does not establish shape correspondence, completed phase or saves.
    pub fn from_checkpoint(
        witness: &ValidatedBvh,
        expected: Identity,
        permit: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        construct(witness.record(), expected, permit)
    }
    /// Plan exact requests for the distinct deferred-bound witness. This does
    /// not upgrade the tree to strict/query-ready status or normalize its flags.
    pub fn checkpoint_pending_plan(
        witness: &ValidatedPendingBvh<'_>,
        expected: Identity,
    ) -> ConstructionResult<AllocationPlan> {
        plan(witness.record(), expected)
    }
    /// Reconstruct the exact deferred tree for its matched owner to continue
    /// native pending processing. Bounds/flags remain pending, so this is not a
    /// query-ready conversion. The immutable witness retains exact named updates;
    /// the enclosing owner must preserve those updates before using the tree.
    pub fn from_pending_checkpoint(
        witness: &ValidatedPendingBvh<'_>,
        expected: Identity,
        permit: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        construct(witness.record(), expected, permit)
    }
}
