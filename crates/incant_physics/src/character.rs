//! Read-only kinematic movement queries. Applying the result is an ordinary
//! transform/velocity command owned by the caller, never a second mutation path.
use crate::{PhysicsError, PhysicsRuntime, groups, vector};
use incant_doc::BodyMotion;
use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use rapier3d::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CharacterStep {
    /// Maximum stair height in meters.
    #[schemars(range(min = 0.001, max = 1000))]
    pub max_height: f64,
    /// Minimum free landing width in meters.
    #[schemars(range(min = 0.001, max = 1000))]
    pub min_width: f64,
    pub include_dynamic_bodies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CharacterOptions {
    /// Skin gap in meters; must be positive.
    #[schemars(range(min = 0.001, max = 1000))]
    pub offset: f64,
    pub slide: bool,
    /// Radians from the world's +Y direction, in 0..pi/2.
    #[schemars(range(min = 0, max = std::f64::consts::FRAC_PI_2))]
    pub max_slope_climb_angle: f64,
    /// Radians from the world's +Y direction, in 0..pi/2.
    #[schemars(range(min = 0, max = std::f64::consts::FRAC_PI_2))]
    pub min_slope_slide_angle: f64,
    /// Downward ground-snap distance in meters, or null to disable.
    #[schemars(range(min = 0.001, max = 1000))]
    pub snap_to_ground: Option<f64>,
    pub autostep: Option<CharacterStep>,
}
impl Default for CharacterOptions {
    fn default() -> Self {
        Self {
            offset: 0.01,
            slide: true,
            max_slope_climb_angle: std::f64::consts::FRAC_PI_4,
            min_slope_slide_angle: std::f64::consts::FRAC_PI_4,
            snap_to_ground: Some(0.2),
            autostep: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CharacterQuery {
    pub scene_id: String,
    /// A nonsensor primitive collider attached to a velocity-kinematic body.
    pub entity_id: String,
    /// Desired world-space displacement in meters for one fixed tick.
    /// Gravity and jumping are authored by the calling behavior.
    pub translation: [f64; 3],
    #[serde(default)]
    pub options: CharacterOptions,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CharacterMovement {
    pub translation: [f64; 3],
    /// Touching a supporting surface; steep/sliding slopes may also be grounded.
    pub grounded: bool,
    /// Moving downward along a contacted slope steeper than the configured limit.
    pub sliding_down_slope: bool,
    /// Stable IDs of colliders encountered during the sweep, sorted and unique.
    pub collisions: Vec<String>,
}

impl PhysicsRuntime {
    pub fn compute_character_motion(
        &self,
        request: &CharacterQuery,
        dt: f64,
    ) -> Result<CharacterMovement, PhysicsError> {
        if self.failed {
            return Err(PhysicsError::NumericRange);
        }
        if !dt.is_finite() || !(1. / 240. ..=1.).contains(&dt) {
            return Err(PhysicsError::InvalidStep);
        }
        let o = &request.options;
        let positive_distance = |v: f64| (0.001..=1000.).contains(&v);
        if !request
            .translation
            .iter()
            .all(|v| v.is_finite() && v.abs() / dt <= 10000.)
            || !positive_distance(o.offset)
            || ![o.max_slope_climb_angle, o.min_slope_slide_angle]
                .into_iter()
                .all(|v| (0. ..=std::f64::consts::FRAC_PI_2).contains(&v))
            || o.snap_to_ground.is_some_and(|v| !positive_distance(v))
            || o.autostep.as_ref().is_some_and(|s| {
                !positive_distance(s.max_height) || !positive_distance(s.min_width)
            })
        {
            return Err(PhysicsError::Query(
                "invalid character movement or options".into(),
            ));
        }
        let scene = self
            .scenes
            .get(&request.scene_id)
            .ok_or_else(|| PhysicsError::Query("scene does not exist".into()))?;
        let entry = scene.entries.get(&request.entity_id).ok_or_else(|| {
            PhysicsError::Query("character collider does not exist in this scene".into())
        })?;
        if entry.config.collider.sensor
            || !entry
                .config
                .body
                .as_ref()
                .is_some_and(|b| b.motion == BodyMotion::Kinematic)
            || entry.config.state.angular_velocity != [0.; 3]
        {
            return Err(PhysicsError::Query(
                "character requires a nonsensor kinematic body with zero angular velocity".into(),
            ));
        }
        let handle = entry.body.expect("validated kinematic body");
        let body = &scene.world.bodies[handle];
        let collider = &scene.world.colliders[entry.collider];
        let filter = QueryFilter {
            flags: QueryFilterFlags::EXCLUDE_SENSORS,
            groups: Some(groups(
                entry.config.collider.memberships,
                entry.config.collider.filter,
            )),
            exclude_rigid_body: Some(handle),
            ..Default::default()
        };
        // An author edit can precede the first solver step. Construct a query-only
        // snapshot rather than consuming the live world's body modification flags.
        // The normal stepped path borrows the existing world and BVH directly.
        let mut fresh_bvh = BroadPhaseBvh::new();
        let mut fresh_colliders;
        let queries = if scene.query_dirty {
            fresh_colliders = scene.world.colliders.clone();
            for e in scene.entries.values() {
                let c = &mut fresh_colliders[e.collider];
                if let Some(b) = e.body {
                    c.set_position(*scene.world.bodies[b].position());
                }
                fresh_bvh.set_aabb(
                    &scene.world.integration_parameters,
                    e.collider,
                    c.compute_aabb(),
                );
            }
            fresh_bvh.as_query_pipeline(
                scene.world.narrow_phase.query_dispatcher(),
                &scene.world.bodies,
                &fresh_colliders,
                filter,
            )
        } else {
            scene.world.query_pipeline_with_filter(filter)
        };
        let dispatcher = crate::character_queries::CharacterQueries(queries.dispatcher);
        let queries = QueryPipeline {
            dispatcher: &dispatcher,
            ..queries
        };
        let controller = KinematicCharacterController {
            up: Vector::Y,
            offset: CharacterLength::Absolute(o.offset as f32),
            slide: o.slide,
            autostep: o.autostep.as_ref().map(|s| CharacterAutostep {
                max_height: CharacterLength::Absolute(s.max_height as f32),
                min_width: CharacterLength::Absolute(s.min_width as f32),
                include_dynamic_bodies: s.include_dynamic_bodies,
            }),
            max_slope_climb_angle: o.max_slope_climb_angle as f32,
            min_slope_slide_angle: o.min_slope_slide_angle as f32,
            snap_to_ground: o
                .snap_to_ground
                .map(|v| CharacterLength::Absolute(v as f32)),
            ..Default::default()
        };
        let mut collisions = BTreeSet::new();
        let movement = controller.move_shape(
            dt as f32,
            &queries,
            collider.shape(),
            body.position(),
            vector(request.translation),
            |hit| {
                collisions.insert(scene.ids[&hit.handle].clone());
            },
        );
        let translation = movement.translation.to_array().map(f64::from);
        // Inspect final support, including ground snapping which may not produce
        // a sweep callback. Side-wall contact noise must not turn a level floor
        // into a slope. The backend's own flag also labels uphill/horizontal
        // contact projection as "sliding down".
        let mut steep_support = false;
        if translation[1] < -1.0e-5 && movement.grounded {
            let final_pose = Pose::from_translation(movement.translation) * *body.position();
            if let Some((_, support)) = queries.cast_shape(
                &final_pose,
                -Vector::Y,
                collider.shape(),
                rapier3d::parry::query::ShapeCastOptions {
                    max_time_of_impact: (2. * o.offset + 0.001) as f32,
                    stop_at_penetration: false,
                    ..Default::default()
                },
            ) {
                let up = f64::from(support.normal1.y);
                steep_support = up > 1.0e-4 && up < o.min_slope_slide_angle.cos();
            }
        }
        if !translation
            .iter()
            .zip(entry.config.state.translation)
            .all(|(delta, position)| {
                delta.is_finite()
                    && delta.abs() / dt <= 10000.
                    && (position + delta).abs() <= 1_000_000.
            })
        {
            return Err(PhysicsError::Query(
                "character movement exceeds supported position or velocity range".into(),
            ));
        }
        Ok(CharacterMovement {
            translation,
            grounded: movement.grounded,
            sliding_down_slope: steep_support,
            collisions: collisions.into_iter().collect(),
        })
    }
}
