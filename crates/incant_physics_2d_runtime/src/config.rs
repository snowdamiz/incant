use crate::{BodyState, PhysicsError};
use incant_2d::tilemap::PreparedTilemap;
use incant_types::{BodyMotion, Collider2D, ColliderShape2D, RigidBody};
use rapier2d::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Geometry {
    Primitive(ColliderShape2D),
    /// (center, half extent), prepared once at a structural boundary.
    Tiles(Vec<([f64; 2], [f64; 2])>),
}

/// Validated declaration. It cannot contain solver handles or arbitrary shapes.
/// Configuration preparation is a load/structural operation, never a tick scan.
pub struct PreparedBody {
    pub(crate) body: Option<RigidBody>,
    pub(crate) collider: Collider2D,
    pub(crate) geometry: Geometry,
    pub(crate) state: BodyState,
}
impl PreparedBody {
    pub fn new(
        body: Option<RigidBody>,
        collider: Collider2D,
        state: BodyState,
    ) -> Result<Self, PhysicsError> {
        validate_collider(&collider)?;
        if let Some(body) = &body {
            incant_types::physics_validation::rigid_body(body).map_err(PhysicsError::Component)?;
        }
        validate_state(body.as_ref(), &state)?;
        Ok(Self {
            geometry: Geometry::Primitive(collider.shape.clone()),
            body,
            collider,
            state,
        })
    }

    /// Consume the exact retained solid-cell union. Empty maps have no binding.
    /// Fixed tile geometry uses Collider2D's default friction/restitution/groups,
    /// no sensor and no rigid-body dynamics; render depth has no collision role.
    pub fn tilemap<I>(map: &PreparedTilemap<I>, state: BodyState) -> Result<Self, PhysicsError> {
        validate_state(None, &state)?;
        if map.collision().is_empty() || map.collision().len() > crate::MAX_SHAPES {
            return Err(PhysicsError::Limit);
        }
        let cell = map.cell_size();
        let parts = map
            .collision()
            .iter()
            .map(|rect| {
                let half = std::array::from_fn(|axis| f64::from(rect.size[axis]) * cell[axis] / 2.);
                let center = std::array::from_fn(|axis| {
                    f64::from(rect.origin[axis]) * cell[axis] + half[axis]
                });
                (center, half)
            })
            .collect();
        Ok(Self {
            body: None,
            collider: Collider2D::default(),
            geometry: Geometry::Tiles(parts),
            state,
        })
    }

    pub(crate) fn shapes(&self) -> usize {
        match &self.geometry {
            Geometry::Primitive(_) => 1,
            Geometry::Tiles(parts) => parts.len(),
        }
    }

    pub(crate) fn shape(&self) -> SharedShape {
        match &self.geometry {
            Geometry::Primitive(ColliderShape2D::Box { half_extents }) => {
                SharedShape::cuboid(half_extents[0] as f32, half_extents[1] as f32)
            }
            Geometry::Primitive(ColliderShape2D::Circle { radius }) => {
                SharedShape::ball(*radius as f32)
            }
            Geometry::Primitive(ColliderShape2D::Capsule {
                half_height,
                radius,
            }) => SharedShape::capsule_y(*half_height as f32, *radius as f32),
            Geometry::Tiles(parts) => SharedShape::compound(
                parts
                    .iter()
                    .map(|(center, half)| {
                        (
                            Pose::from_translation(crate::vector(*center)),
                            SharedShape::cuboid(half[0] as f32, half[1] as f32),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

pub fn validate_collider(collider: &Collider2D) -> Result<(), PhysicsError> {
    let dimension = |v: f64| (0.001..=10000.).contains(&v);
    let valid = match collider.shape {
        ColliderShape2D::Box { half_extents } => half_extents.into_iter().all(dimension),
        ColliderShape2D::Circle { radius } => dimension(radius),
        ColliderShape2D::Capsule {
            half_height,
            radius,
        } => dimension(radius) && (0. ..=10000.).contains(&half_height),
    };
    if !valid
        || !(0.001..=100000.).contains(&collider.density)
        || !(0. ..=10.).contains(&collider.friction)
        || !(0. ..=1.).contains(&collider.restitution)
    {
        return Err(PhysicsError::Component(
            "2D collider dimensions/material out of range".into(),
        ));
    }
    Ok(())
}

pub fn validate_state(body: Option<&RigidBody>, state: &BodyState) -> Result<(), PhysicsError> {
    let bounded = |value: f64, limit: f64| value.is_finite() && value.abs() <= limit;
    if !state
        .translation
        .into_iter()
        .all(|v| bounded(v, 1_000_000.))
        || !bounded(state.rotation, 1_000_000.)
        || !state.velocity.into_iter().all(|v| bounded(v, 10000.))
        || !bounded(state.angular_velocity, 10000.)
    {
        return Err(PhysicsError::Component(
            "2D position/rotation/velocity out of range".into(),
        ));
    }
    if body.is_none_or(|b| b.motion == BodyMotion::Fixed)
        && (state.velocity != [0.; 2] || state.angular_velocity != 0.)
    {
        return Err(PhysicsError::Component(
            "fixed 2D colliders cannot move".into(),
        ));
    }
    Ok(())
}
