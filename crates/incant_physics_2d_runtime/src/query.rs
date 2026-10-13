use crate::{PhysicsError, PhysicsWorld2D, groups, vector};
use incant_types::StableId;
use rapier2d::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "RayQuery2D")]
pub struct RayQuery<I = StableId> {
    pub origin: [f64; 2],
    pub direction: [f64; 2],
    pub max_distance: f64,
    pub include_sensors: bool,
    pub exclude_entity: Option<I>,
    pub memberships: u32,
    pub filter: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "RayHit2D")]
pub struct RayHit<I = StableId> {
    #[serde(rename = "entity_id")]
    pub entity: I,
    pub distance: f64,
    pub point: [f64; 2],
    pub normal: [f64; 2],
}
impl<I> RayQuery<I> {
    /// Pure bounded input check, available before an owner flushes numeric edits.
    pub fn validate(&self) -> Result<(), PhysicsError> {
        if !self
            .origin
            .into_iter()
            .chain(self.direction)
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
            || !(0. ..=1_000_000.).contains(&self.max_distance)
            || vector(self.direction).try_normalize().is_none()
        {
            return Err(PhysicsError::Query);
        }
        Ok(())
    }
}
impl PhysicsWorld2D {
    pub fn raycast(&self, query: &RayQuery) -> Result<Option<RayHit>, PhysicsError> {
        self.check_live()?;
        query.validate()?;
        let direction = vector(query.direction)
            .try_normalize()
            .expect("validated direction");
        let ray = Ray::new(vector(query.origin), direction);
        let filter = QueryFilter {
            exclude_collider: query
                .exclude_entity
                .and_then(|id| self.entries.get(&id))
                .map(|e| e.collider),
            groups: Some(groups(query.memberships, query.filter)),
            flags: if query.include_sensors {
                QueryFilterFlags::empty()
            } else {
                QueryFilterFlags::EXCLUDE_SENSORS
            },
            ..QueryFilter::default()
        };
        // Never run CollisionPipeline solely to refresh a query: in pinned
        // Rapier that consumes insertion flags before simulation island setup.
        // Query current shapes directly until a real step builds the BVH.
        let hits: Box<dyn Iterator<Item = (ColliderHandle, RayIntersection)> + '_> =
            if self.query_dirty {
                Box::new(self.entries.values().filter_map(|entry| {
                    let collider = &self.world.read().colliders[entry.collider];
                    if !filter.test(&self.world.read().bodies, entry.collider, collider) {
                        return None;
                    }
                    let pose = entry.body.map_or(collider.position(), |body| {
                        self.world.read().bodies[body].position()
                    });
                    collider
                        .shape()
                        .cast_ray_and_get_normal(pose, &ray, query.max_distance as f32, true)
                        .map(|hit| (entry.collider, hit))
                }))
            } else {
                Box::new(
                    self.world
                        .read()
                        .intersect_ray(ray, query.max_distance as f32, true, filter)
                        .map(|(handle, _, hit)| (handle, hit)),
                )
            };
        Ok(hits
            .map(|(handle, hit)| RayHit {
                entity: self.ids[&handle],
                distance: f64::from(hit.time_of_impact),
                point: (ray.origin + ray.dir * hit.time_of_impact)
                    .to_array()
                    .map(f64::from),
                normal: hit.normal.to_array().map(f64::from),
            })
            .min_by(|a, b| {
                a.distance
                    .total_cmp(&b.distance)
                    .then(a.entity.cmp(&b.entity))
            }))
    }
}
