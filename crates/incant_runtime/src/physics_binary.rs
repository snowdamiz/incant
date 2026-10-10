//! Explicit, bounded cooked physics records. No native layouts or JSON payloads.
use crate::binary::Reader;
use crate::{CookedPhysics, SceneError, StableId};
use incant_types::{
    AngularVelocity, BodyMotion, Collider, ColliderPart, ColliderShape, PrimitiveColliderShape,
    RigidBody,
};

pub(crate) fn encoded_size(value: &CookedPhysics) -> usize {
    // Structural boundary only; exact size, bounded to 64 compound parts.
    let mut bytes = Vec::new();
    write(&mut bytes, value);
    bytes.len()
}
fn floats(bytes: &mut Vec<u8>, values: impl IntoIterator<Item = f64>) {
    for value in values {
        bytes.extend(value.to_le_bytes());
    }
}
fn primitive(bytes: &mut Vec<u8>, shape: &PrimitiveColliderShape) {
    match shape {
        PrimitiveColliderShape::Box { half_extents } => {
            bytes.push(0);
            floats(bytes, *half_extents);
        }
        PrimitiveColliderShape::Sphere { radius } => {
            bytes.push(1);
            floats(bytes, [*radius]);
        }
        PrimitiveColliderShape::Capsule {
            half_height,
            radius,
        } => {
            bytes.push(2);
            floats(bytes, [*half_height, *radius]);
        }
    }
}
pub(crate) fn write(bytes: &mut Vec<u8>, value: &CookedPhysics) {
    bytes.extend(value.entity.0);
    match &value.body {
        None => bytes.push(0),
        Some(body) => {
            bytes.push(match body.motion {
                BodyMotion::Fixed => 1,
                BodyMotion::Dynamic => 2,
                BodyMotion::Kinematic => 3,
            });
            floats(
                bytes,
                [
                    body.gravity_scale,
                    body.linear_damping,
                    body.angular_damping,
                ],
            );
            bytes.extend([u8::from(body.can_sleep), u8::from(body.ccd)]);
        }
    }
    bytes.push(u8::from(value.angular_velocity.is_some()));
    if let Some(spin) = &value.angular_velocity {
        floats(bytes, spin.angular);
    }
    let collider = &value.collider;
    floats(
        bytes,
        [collider.density, collider.friction, collider.restitution],
    );
    bytes.push(u8::from(collider.sensor));
    bytes.extend(collider.memberships.to_le_bytes());
    bytes.extend(collider.filter.to_le_bytes());
    match &collider.shape {
        ColliderShape::Box { half_extents } => primitive(
            bytes,
            &PrimitiveColliderShape::Box {
                half_extents: *half_extents,
            },
        ),
        ColliderShape::Sphere { radius } => {
            primitive(bytes, &PrimitiveColliderShape::Sphere { radius: *radius })
        }
        ColliderShape::Capsule {
            half_height,
            radius,
        } => primitive(
            bytes,
            &PrimitiveColliderShape::Capsule {
                half_height: *half_height,
                radius: *radius,
            },
        ),
        ColliderShape::Compound { parts } => {
            bytes.push(3);
            bytes.push(parts.len() as u8);
            for part in parts {
                bytes.extend(part.id.as_bytes());
                floats(bytes, part.translation);
                floats(bytes, part.rotation);
                primitive(bytes, &part.shape);
            }
        }
    }
}
fn read_primitive(reader: &mut Reader<'_>, tag: u8) -> Result<PrimitiveColliderShape, SceneError> {
    Ok(match tag {
        0 => PrimitiveColliderShape::Box {
            half_extents: reader.floats()?,
        },
        1 => PrimitiveColliderShape::Sphere {
            radius: reader.floats::<1>()?[0],
        },
        2 => {
            let [half_height, radius] = reader.floats()?;
            PrimitiveColliderShape::Capsule {
                half_height,
                radius,
            }
        }
        _ => return Err(SceneError::Invalid("unknown primitive tag")),
    })
}
fn boolean(reader: &mut Reader<'_>) -> Result<bool, SceneError> {
    match reader.byte()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(SceneError::Invalid("invalid boolean")),
    }
}
pub(crate) fn read(reader: &mut Reader<'_>) -> Result<CookedPhysics, SceneError> {
    let entity = StableId(reader.array()?);
    let body = match reader.byte()? {
        0 => None,
        tag @ 1..=3 => {
            let [gravity_scale, linear_damping, angular_damping] = reader.floats()?;
            Some(RigidBody {
                motion: match tag {
                    1 => BodyMotion::Fixed,
                    2 => BodyMotion::Dynamic,
                    _ => BodyMotion::Kinematic,
                },
                gravity_scale,
                linear_damping,
                angular_damping,
                can_sleep: boolean(reader)?,
                ccd: boolean(reader)?,
            })
        }
        _ => return Err(SceneError::Invalid("unknown body tag")),
    };
    let angular_velocity = if boolean(reader)? {
        Some(AngularVelocity {
            angular: reader.floats()?,
        })
    } else {
        None
    };
    let [density, friction, restitution] = reader.floats()?;
    let sensor = boolean(reader)?;
    let memberships = reader.u32()?;
    let filter = reader.u32()?;
    let tag = reader.byte()?;
    let shape = if tag == 3 {
        let count = reader.byte()? as usize;
        if !(1..=64).contains(&count) {
            return Err(SceneError::Invalid("compound part count"));
        }
        let mut parts: Vec<ColliderPart> = Vec::with_capacity(count);
        for _ in 0..count {
            let id = String::from_utf8(reader.array::<26>()?.to_vec())
                .map_err(|_| SceneError::Invalid("compound part ID"))?;
            if parts.last().is_some_and(|part| part.id >= id) {
                return Err(SceneError::Invalid("noncanonical compound parts"));
            }
            let translation = reader.floats()?;
            let rotation = reader.floats()?;
            let tag = reader.byte()?;
            parts.push(ColliderPart {
                id,
                translation,
                rotation,
                shape: read_primitive(reader, tag)?,
            });
        }
        ColliderShape::Compound { parts }
    } else {
        match read_primitive(reader, tag)? {
            PrimitiveColliderShape::Box { half_extents } => ColliderShape::Box { half_extents },
            PrimitiveColliderShape::Sphere { radius } => ColliderShape::Sphere { radius },
            PrimitiveColliderShape::Capsule {
                half_height,
                radius,
            } => ColliderShape::Capsule {
                half_height,
                radius,
            },
        }
    };
    Ok(CookedPhysics {
        entity,
        body,
        collider: Collider {
            shape,
            density,
            friction,
            restitution,
            sensor,
            memberships,
            filter,
        },
        angular_velocity,
    })
}
