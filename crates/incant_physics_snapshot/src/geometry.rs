//! Closed geometry admission only. Compound acceleration and native stable-ID
//! correspondence still need validation in the future complete owner format.
use crate::{Dimension, Error, Identity, MAX_ITEMS, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Primitive {
    Box { half_extents: [u32; 3] },
    Ball { radius: u32 },
    CapsuleY { half_height: u32, radius: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Part {
    pub translation: [u32; 3],
    pub rotation: [u32; 4],
    pub shape: Primitive,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum GeometryKind {
    Primitive = 0,
    SpatialCompound = 1,
    PlanarTiles = 2,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometryRecord {
    pub identity: Identity,
    pub kind: GeometryKind,
    pub parts: Vec<Part>,
}
pub struct ValidatedGeometry {
    record: GeometryRecord,
}
impl ValidatedGeometry {
    pub fn record(&self) -> &GeometryRecord {
        &self.record
    }
}
const IDENTITY: [u32; 4] = [0, 0, 0, 1f32.to_bits()];

pub(crate) fn part_count(kind: GeometryKind, dim: Dimension, count: usize) -> Result<()> {
    let valid = match kind {
        GeometryKind::Primitive => count == 1,
        GeometryKind::SpatialCompound => dim == Dimension::Spatial && (1..=64).contains(&count),
        GeometryKind::PlanarTiles => dim == Dimension::Planar && (1..=MAX_ITEMS).contains(&count),
    };
    if valid { Ok(()) } else { Err(Error::Geometry) }
}
fn range(bits: u32, min: f32, max: f32) -> bool {
    (min..=max).contains(&f32::from_bits(bits))
}

pub fn validate_geometry(record: GeometryRecord) -> Result<ValidatedGeometry> {
    part_count(record.kind, record.identity.dimension, record.parts.len())?;
    let dimension = record.identity.dimension;
    for part in &record.parts {
        match record.kind {
            GeometryKind::Primitive => {
                if part.translation != [0; 3] || part.rotation != IDENTITY {
                    return Err(Error::Geometry);
                }
            }
            GeometryKind::SpatialCompound => {
                let q = part.rotation.map(|b| f64::from(f32::from_bits(b)));
                let norm = q.iter().map(|v| v * v).sum::<f64>();
                if !norm.is_finite()
                    || (norm - 1.).abs() > 1e-5
                    || part.translation.iter().any(|v| !range(*v, -10000., 10000.))
                {
                    return Err(Error::Geometry);
                }
            }
            GeometryKind::PlanarTiles => {
                if part.translation[2] != 0
                    || part.rotation != IDENTITY
                    || part.translation[..2]
                        .iter()
                        .any(|v| !range(*v, 0., 1_000_000.))
                {
                    return Err(Error::Geometry);
                }
            }
        }
        let tiles = record.kind == GeometryKind::PlanarTiles;
        match part.shape {
            Primitive::Box { half_extents } => {
                let (min, max) = if tiles {
                    (0.0000005, 500_000.)
                } else {
                    (0.001, 10000.)
                };
                if (dimension == Dimension::Planar && half_extents[2] != 0)
                    || half_extents[..dimension as usize]
                        .iter()
                        .any(|v| !range(*v, min, max))
                {
                    return Err(Error::Geometry);
                }
            }
            Primitive::Ball { radius } => {
                if tiles || !range(radius, 0.001, 10000.) {
                    return Err(Error::Geometry);
                }
            }
            Primitive::CapsuleY {
                half_height,
                radius,
            } => {
                if tiles || !range(half_height, 0., 10000.) || !range(radius, 0.001, 10000.) {
                    return Err(Error::Geometry);
                }
            }
        }
    }
    Ok(ValidatedGeometry { record })
}
