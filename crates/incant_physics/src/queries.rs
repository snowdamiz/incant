use crate::{PhysicsError, PhysicsRuntime, groups, vector};
use rapier3d::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RayQuery {
    pub scene_id: String,
    pub origin: [f64; 3],
    /// Normalized by the runtime; a zero vector is rejected.
    pub direction: [f64; 3],
    pub max_distance: f64,
    pub include_sensors: bool,
    pub exclude_entity: Option<String>,
    pub memberships: u32,
    pub filter: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RayHit {
    pub entity_id: String,
    pub distance: f64,
    pub point: [f64; 3],
    pub normal: [f64; 3],
}
impl PhysicsRuntime {
    pub fn raycast(&self, request: &RayQuery) -> Result<Option<RayHit>, PhysicsError> {
        if self.failed {
            return Err(PhysicsError::NumericRange);
        }
        if !request
            .origin
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
            || !request
                .direction
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
            || !(0. ..=1_000_000.).contains(&request.max_distance)
        {
            return Err(PhysicsError::Query(
                "ray values exceed supported range".into(),
            ));
        }
        let Some(direction) = vector(request.direction).try_normalize() else {
            return Err(PhysicsError::Query("ray direction must be nonzero".into()));
        };
        let Some(scene) = self.scenes.get(&request.scene_id) else {
            return Err(PhysicsError::Query("scene does not exist".into()));
        };
        let exclude_collider = request
            .exclude_entity
            .as_ref()
            .and_then(|id| scene.entries.get(id))
            .map(|e| e.collider);
        let filter = QueryFilter {
            exclude_collider,
            groups: Some(groups(request.memberships, request.filter)),
            flags: if request.include_sensors {
                QueryFilterFlags::empty()
            } else {
                QueryFilterFlags::EXCLUDE_SENSORS
            },
            ..QueryFilter::default()
        };
        let ray = Ray::new(vector(request.origin), direction);
        // Stable entity ID breaks exact-distance ties independently of BVH traversal.
        // CollisionPipeline::detect_collisions clears newly inserted body's
        // modification flags without registering it in simulation islands in
        // Rapier 0.36. Never consume those flags just to refresh a query. Before
        // the next step, query edited collider shapes directly at current poses.
        // Stepped worlds use Rapier's BVH; both paths share stable tie-breaking.
        let hits: Box<dyn Iterator<Item = (ColliderHandle, RayIntersection)> + '_> =
            if scene.query_dirty {
                Box::new(scene.entries.values().filter_map(|entry| {
                    let collider = &scene.world.colliders[entry.collider];
                    if !filter.test(&scene.world.bodies, entry.collider, collider) {
                        return None;
                    }
                    let pose = entry.body.map_or(collider.position(), |handle| {
                        scene.world.bodies[handle].position()
                    });
                    collider
                        .shape()
                        .cast_ray_and_get_normal(pose, &ray, request.max_distance as f32, true)
                        .map(|hit| (entry.collider, hit))
                }))
            } else {
                Box::new(
                    scene
                        .world
                        .intersect_ray(ray, request.max_distance as f32, true, filter)
                        .map(|(handle, _, hit)| (handle, hit)),
                )
            };
        Ok(hits
            .map(|(handle, hit)| RayHit {
                entity_id: scene.ids[&handle].clone(),
                distance: f64::from(hit.time_of_impact),
                point: (ray.origin + ray.dir * hit.time_of_impact)
                    .to_array()
                    .map(f64::from),
                normal: hit.normal.to_array().map(f64::from),
            })
            .min_by(|a, b| {
                a.distance
                    .total_cmp(&b.distance)
                    .then_with(|| a.entity_id.cmp(&b.entity_id))
            }))
    }
}
