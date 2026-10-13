use crate::{
    Budget, Dimension, Error, GeometryKind, GeometryRecord, Identity, MAX_BYTES, Part, Primitive,
    Result, ValidatedGeometry, codec::Reader, geometry::part_count, validate_geometry, vector,
};
const MAGIC: &[u8; 8] = b"ICGEO001";
const HEADER: usize = 56;
const PART: usize = 44;
fn length(count: usize) -> Result<usize> {
    count
        .checked_mul(PART)
        .and_then(|n| n.checked_add(HEADER))
        .filter(|n| *n <= MAX_BYTES)
        .ok_or(Error::Limit)
}

pub fn decode_geometry(
    bytes: &[u8],
    expected: Identity,
    budget: &mut Budget,
) -> Result<ValidatedGeometry> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut read = Reader { bytes, position: 0 };
    if &read.take::<8>()? != MAGIC {
        return Err(Error::Header);
    }
    if u16::from_le_bytes(read.take()?) != 1 {
        return Err(Error::Version);
    }
    let dimension = match read.take::<1>()?[0] {
        2 => Dimension::Planar,
        3 => Dimension::Spatial,
        _ => return Err(Error::Identity),
    };
    if read.take::<1>()?[0] != 1 {
        return Err(Error::Identity);
    }
    let identity = Identity {
        dimension,
        build: read.take()?,
    };
    if identity != expected {
        return Err(Error::Identity);
    }
    let kind = match read.word()? {
        0 => GeometryKind::Primitive,
        1 => GeometryKind::SpatialCompound,
        2 => GeometryKind::PlanarTiles,
        _ => return Err(Error::Geometry),
    };
    let count = read.word()? as usize;
    if read.word()? != 0 {
        return Err(Error::Header);
    }
    part_count(kind, dimension, count)?;
    if bytes.len() != length(count)? {
        return Err(Error::Length);
    }
    budget.charge(count.checked_mul(size_of::<Part>()).ok_or(Error::Limit)?)?;
    let mut parts = vector(count)?;
    for _ in 0..count {
        let shape = read.word()?;
        let parameters = [read.word()?, read.word()?, read.word()?];
        let shape = match shape {
            0 => Primitive::Box {
                half_extents: parameters,
            },
            1 if parameters[1..] == [0, 0] => Primitive::Ball {
                radius: parameters[0],
            },
            2 if parameters[2] == 0 => Primitive::CapsuleY {
                half_height: parameters[0],
                radius: parameters[1],
            },
            _ => return Err(Error::Geometry),
        };
        parts.push(Part {
            shape,
            translation: [read.word()?, read.word()?, read.word()?],
            rotation: [read.word()?, read.word()?, read.word()?, read.word()?],
        });
    }
    validate_geometry(GeometryRecord {
        identity,
        kind,
        parts,
    })
}

pub fn encode_geometry(value: &ValidatedGeometry, budget: &mut Budget) -> Result<Vec<u8>> {
    let record = value.record();
    let len = length(record.parts.len())?;
    budget.charge(len)?;
    let mut bytes = vector(len)?;
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&[record.identity.dimension as u8, 1]);
    bytes.extend_from_slice(&record.identity.build);
    for n in [record.kind as u32, record.parts.len() as u32, 0] {
        bytes.extend_from_slice(&n.to_le_bytes());
    }
    for part in &record.parts {
        let (kind, parameters) = match part.shape {
            Primitive::Box { half_extents } => (0, half_extents),
            Primitive::Ball { radius } => (1, [radius, 0, 0]),
            Primitive::CapsuleY {
                half_height,
                radius,
            } => (2, [half_height, radius, 0]),
        };
        for n in [kind]
            .into_iter()
            .chain(parameters)
            .chain(part.translation)
            .chain(part.rotation)
        {
            bytes.extend_from_slice(&n.to_le_bytes());
        }
    }
    Ok(bytes)
}
