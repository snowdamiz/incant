//! Version-7 explicit planar records. No casts/native-layout or JSON payloads.
use crate::{CookedPhysics2D, SceneError, StableId, binary::Reader};
use incant_types::{AngularVelocity2D, BodyMotion, Collider2D, ColliderShape2D, RigidBody};

pub(crate) fn encoded_size(v: &CookedPhysics2D) -> usize {
    16 + 1
        + usize::from(v.body.is_some()) * 26
        + 1
        + usize::from(v.angular_velocity.is_some()) * 8
        + 33
        + 1
        + match v.collider.shape {
            ColliderShape2D::Circle { .. } => 8,
            _ => 16,
        }
}
fn floats(out: &mut Vec<u8>, values: impl IntoIterator<Item = f64>) {
    for v in values {
        out.extend(v.to_le_bytes());
    }
}
pub(crate) fn write(out: &mut Vec<u8>, v: &CookedPhysics2D) {
    out.extend(v.entity.0);
    if let Some(body) = &v.body {
        out.push(match body.motion {
            BodyMotion::Fixed => 1,
            BodyMotion::Dynamic => 2,
            BodyMotion::Kinematic => 3,
        });
        floats(
            out,
            [
                body.gravity_scale,
                body.linear_damping,
                body.angular_damping,
            ],
        );
        out.extend([u8::from(body.can_sleep), u8::from(body.ccd)]);
    } else {
        out.push(0);
    }
    out.push(u8::from(v.angular_velocity.is_some()));
    if let Some(spin) = &v.angular_velocity {
        floats(out, [spin.angular]);
    }
    let c = &v.collider;
    floats(out, [c.density, c.friction, c.restitution]);
    out.push(u8::from(c.sensor));
    out.extend(c.memberships.to_le_bytes());
    out.extend(c.filter.to_le_bytes());
    match c.shape {
        ColliderShape2D::Box { half_extents } => {
            out.push(0);
            floats(out, half_extents);
        }
        ColliderShape2D::Circle { radius } => {
            out.push(1);
            floats(out, [radius]);
        }
        ColliderShape2D::Capsule {
            half_height,
            radius,
        } => {
            out.push(2);
            floats(out, [half_height, radius]);
        }
    }
}
fn boolean(r: &mut Reader<'_>) -> Result<bool, SceneError> {
    match r.byte()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(SceneError::Invalid("2D boolean")),
    }
}
pub(crate) fn read(r: &mut Reader<'_>) -> Result<CookedPhysics2D, SceneError> {
    let entity = StableId(r.array()?);
    let body = match r.byte()? {
        0 => None,
        tag @ 1..=3 => {
            let [gravity_scale, linear_damping, angular_damping] = r.floats()?;
            Some(RigidBody {
                motion: match tag {
                    1 => BodyMotion::Fixed,
                    2 => BodyMotion::Dynamic,
                    _ => BodyMotion::Kinematic,
                },
                gravity_scale,
                linear_damping,
                angular_damping,
                can_sleep: boolean(r)?,
                ccd: boolean(r)?,
            })
        }
        _ => return Err(SceneError::Invalid("2D body tag")),
    };
    let angular_velocity = if boolean(r)? {
        Some(AngularVelocity2D {
            angular: r.floats::<1>()?[0],
        })
    } else {
        None
    };
    let [density, friction, restitution] = r.floats()?;
    let sensor = boolean(r)?;
    let memberships = r.u32()?;
    let filter = r.u32()?;
    let shape = match r.byte()? {
        0 => ColliderShape2D::Box {
            half_extents: r.floats()?,
        },
        1 => ColliderShape2D::Circle {
            radius: r.floats::<1>()?[0],
        },
        2 => {
            let [half_height, radius] = r.floats()?;
            ColliderShape2D::Capsule {
                half_height,
                radius,
            }
        }
        _ => return Err(SceneError::Invalid("2D shape tag")),
    };
    Ok(CookedPhysics2D {
        entity,
        body,
        collider: Collider2D {
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
