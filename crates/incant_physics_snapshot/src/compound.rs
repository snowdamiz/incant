//! Immutable acceleration storage joined to its closed geometry declaration.
//! Native adapters additionally verify exact shape-derived bounds and poses.
use crate::{
    Budget, BvhRecord, Dimension, Error, GeometryKind, Identity, Result, ValidatedBvh,
    ValidatedGeometry, validate,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompoundAabb {
    pub mins: [u32; 3],
    pub maxs: [u32; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompoundRecord {
    pub identity: Identity,
    pub child_aabbs: Vec<CompoundAabb>,
    pub root_aabb: CompoundAabb,
    pub tree: BvhRecord,
    pub flags: u32,
    pub pseudo_normals: bool,
}

pub(crate) struct CompoundStorage {
    identity: Identity,
    child_aabbs: Vec<CompoundAabb>,
    root_aabb: CompoundAabb,
    tree: ValidatedBvh,
}

/// Borrowed witness: acceleration and child declarations cannot be substituted
/// independently after the structural join. It authenticates no native epoch.
#[derive(Clone, Copy)]
pub struct ValidatedCompound<'a> {
    geometry: &'a ValidatedGeometry,
    storage: &'a CompoundStorage,
}
impl<'a> ValidatedCompound<'a> {
    pub fn geometry(&self) -> &'a ValidatedGeometry {
        self.geometry
    }
    pub fn tree(&self) -> &'a ValidatedBvh {
        &self.storage.tree
    }
    pub fn child_aabbs(&self) -> &'a [CompoundAabb] {
        &self.storage.child_aabbs
    }
    pub fn root_aabb(&self) -> CompoundAabb {
        self.storage.root_aabb
    }
    pub fn identity(&self) -> Identity {
        self.storage.identity
    }
}
impl ValidatedGeometry {
    pub fn compound(&self) -> Option<ValidatedCompound<'_>> {
        self.compound.as_ref().map(|storage| ValidatedCompound {
            geometry: self,
            storage,
        })
    }
}

fn bounds(a: CompoundAabb, dimension: Dimension) -> Result<()> {
    if dimension == Dimension::Planar && (a.mins[2] != 0 || a.maxs[2] != 0) {
        return Err(Error::Bounds);
    }
    for axis in 0..dimension as usize {
        let lo = f32::from_bits(a.mins[axis]);
        let hi = f32::from_bits(a.maxs[axis]);
        if !lo.is_finite() || !hi.is_finite() || lo > hi || lo.abs() > 1e9 || hi.abs() > 1e9 {
            return Err(Error::Bounds);
        }
    }
    Ok(())
}
fn contains(a: CompoundAabb, b: CompoundAabb, dimension: Dimension) -> bool {
    (0..dimension as usize).all(|axis| {
        f32::from_bits(a.mins[axis]) <= f32::from_bits(b.mins[axis])
            && f32::from_bits(a.maxs[axis]) >= f32::from_bits(b.maxs[axis])
    })
}

/// Consume actual captured storage, preserving child order, bounds and tree
/// layout. This never rebuilds a BVH. Input allocations belong to the caller's
/// capture/decode budget; tree-validation scratch consumes the same budget.
pub fn attach_compound(
    mut geometry: ValidatedGeometry,
    record: CompoundRecord,
    budget: &mut Budget,
) -> Result<ValidatedGeometry> {
    let g = geometry.record();
    if record.identity != g.identity || record.tree.identity != g.identity {
        return Err(Error::Identity);
    }
    if g.kind == GeometryKind::Primitive
        || geometry.compound.is_some()
        || record.flags != 0
        || record.pseudo_normals
    {
        return Err(Error::Geometry);
    }
    let count = g.parts.len();
    if record.child_aabbs.len() != count
        || record.tree.leaves.slots.len() != count
        || record.tree.leaves.occupied as usize != count
    {
        return Err(Error::Length);
    }
    let dimension = g.identity.dimension;
    bounds(record.root_aabb, dimension)?;
    let tree = validate(record.tree, budget)?;
    for (index, &aabb) in record.child_aabbs.iter().enumerate() {
        bounds(aabb, dimension)?;
        let packed = tree.record().leaves.slots[index].ok_or(Error::Leaf)?;
        let wide = tree.record().nodes[packed as usize / 2];
        let node = if packed % 2 == 0 {
            wide.left
        } else {
            wide.right
        };
        if node.children as usize != index
            || node.mins != aabb.mins
            || node.maxs != aabb.maxs
            || !contains(record.root_aabb, aabb, dimension)
        {
            return Err(Error::Bounds);
        }
    }
    // The root may differ in signed-zero tie selection from the stored compound
    // root; native adapters check the latter against native shape arithmetic.
    let root = tree.record().nodes[0];
    for node in [root.left, root.right] {
        if node.leaf_count() != 0
            && !contains(
                record.root_aabb,
                CompoundAabb {
                    mins: node.mins,
                    maxs: node.maxs,
                },
                dimension,
            )
        {
            return Err(Error::Bounds);
        }
    }
    geometry.compound = Some(CompoundStorage {
        identity: record.identity,
        child_aabbs: record.child_aabbs,
        root_aabb: record.root_aabb,
        tree,
    });
    Ok(geometry)
}
