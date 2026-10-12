//! Copied read-only committed physics. Applying character motion is ordinary
//! numeric gameplay work; these queries never move bodies or step a solver.
use crate::{EntityId, Error};
pub use abi::{
    PHYSICS_MAX_BATCH, PHYSICS_MAX_EVENT_BYTES, PHYSICS_MAX_REQUEST_BYTES, PHYSICS_MAX_RESULT_BYTES,
};
use incant_wasm_abi as abi;
#[derive(Clone, Copy, Debug)]
pub struct Ray3 {
    pub origin: [f64; 3],
    pub direction: [f64; 3],
    pub max_distance: f64,
    pub include_sensors: bool,
    pub exclude_entity: Option<EntityId>,
    pub memberships: u32,
    pub filter: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Ray2 {
    pub origin: [f64; 2],
    pub direction: [f64; 2],
    pub max_distance: f64,
    pub include_sensors: bool,
    pub exclude_entity: Option<EntityId>,
    pub memberships: u32,
    pub filter: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct AutoStep {
    pub max_height: f64,
    pub min_width: f64,
    pub include_dynamic_bodies: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct CharacterOptions {
    pub offset: f64,
    pub slide: bool,
    pub max_slope_climb_angle: f64,
    pub min_slope_slide_angle: f64,
    pub snap_to_ground: Option<f64>,
    pub autostep: Option<AutoStep>,
}
impl Default for CharacterOptions {
    fn default() -> Self {
        Self {
            offset: 0.01,
            slide: true,
            max_slope_climb_angle: core::f64::consts::FRAC_PI_4,
            min_slope_slide_angle: core::f64::consts::FRAC_PI_4,
            snap_to_ground: Some(0.2),
            autostep: None,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Character {
    pub entity: EntityId,
    pub translation: [f64; 3],
    pub options: CharacterOptions,
}
#[derive(Clone, Copy, Debug)]
pub enum Query {
    Ray3(Ray3),
    Ray2(Ray2),
    Character(Character),
}
pub(crate) const EMPTY: abi::PhysicsQuery = abi::PhysicsQuery {
    kind: 0,
    flags: 0,
    entity: [0; 16],
    a: [0.; 3],
    b: [0.; 3],
    c: 0.,
    d: 0.,
    e: 0.,
    memberships: 0,
    filter: 0,
    reserved: 0,
};
impl Query {
    pub(crate) fn wire(self) -> abi::PhysicsQuery {
        match self {
            Self::Ray3(q) => abi::PhysicsQuery {
                kind: 0,
                flags: u32::from(q.include_sensors) | (u32::from(q.exclude_entity.is_some()) << 1),
                entity: q.exclude_entity.map_or([0; 16], |id| id.0),
                a: q.origin,
                b: q.direction,
                c: q.max_distance,
                memberships: q.memberships,
                filter: q.filter,
                ..EMPTY
            },
            Self::Ray2(q) => abi::PhysicsQuery {
                kind: 1,
                flags: u32::from(q.include_sensors) | (u32::from(q.exclude_entity.is_some()) << 1),
                entity: q.exclude_entity.map_or([0; 16], |id| id.0),
                a: [q.origin[0], q.origin[1], 0.],
                b: [q.direction[0], q.direction[1], 0.],
                c: q.max_distance,
                memberships: q.memberships,
                filter: q.filter,
                ..EMPTY
            },
            Self::Character(q) => {
                let o = q.options;
                abi::PhysicsQuery {
                    kind: 2,
                    flags: u32::from(o.slide)
                        | (u32::from(o.snap_to_ground.is_some()) << 1)
                        | (u32::from(o.autostep.is_some()) << 2)
                        | (u32::from(o.autostep.is_some_and(|s| s.include_dynamic_bodies)) << 3),
                    entity: q.entity.0,
                    a: q.translation,
                    b: [o.offset, o.max_slope_climb_angle, o.min_slope_slide_angle],
                    c: o.snap_to_ground.unwrap_or(0.),
                    d: o.autostep.map_or(0., |s| s.max_height),
                    e: o.autostep.map_or(0., |s| s.min_width),
                    ..EMPTY
                }
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit<const N: usize> {
    pub entity: EntityId,
    pub distance: f64,
    pub point: [f64; N],
    pub normal: [f64; N],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterMovement<'a> {
    pub translation: [f64; 3],
    pub grounded: bool,
    pub sliding_down_slope: bool,
    collisions: &'a [[u8; 16]],
}
impl<'a> CharacterMovement<'a> {
    pub fn collisions(&self) -> impl ExactSizeIterator<Item = EntityId> + 'a {
        self.collisions.iter().copied().map(EntityId)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QueryResult<'a> {
    Ray3(Option<RayHit<3>>),
    Ray2(Option<RayHit<2>>),
    Character(CharacterMovement<'a>),
}
#[derive(Clone, Copy)]
pub struct Results<'a>(pub(crate) abi::physics::Results<'a>);
impl<'a> Results<'a> {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = QueryResult<'a>> + 'a {
        self.0.iter().map(|(r, ids)| match r.kind {
            0 => QueryResult::Ray3((r.flags != 0).then_some(RayHit {
                entity: EntityId(r.entity),
                distance: r.distance,
                point: r.point,
                normal: r.normal,
            })),
            1 => QueryResult::Ray2((r.flags != 0).then_some(RayHit {
                entity: EntityId(r.entity),
                distance: r.distance,
                point: [r.point[0], r.point[1]],
                normal: [r.normal[0], r.normal[1]],
            })),
            _ => QueryResult::Character(CharacterMovement {
                translation: r.point,
                grounded: r.flags & 1 != 0,
                sliding_down_slope: r.flags & 2 != 0,
                collisions: ids,
            }),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    Planar,
    Spatial,
}
impl Dimension {
    pub(crate) fn wire(self) -> i32 {
        match self {
            Self::Planar => 2,
            Self::Spatial => 3,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerEvent {
    pub first: EntityId,
    pub second: EntityId,
    pub entered: bool,
}
#[derive(Clone, Copy)]
pub struct Events<'a>(pub(crate) abi::physics::Events<'a>);
impl<'a> Events<'a> {
    /// Active scene captured with these events, including an empty frame.
    pub fn scene(&self) -> EntityId {
        EntityId(self.0.scene)
    }
    pub fn dimension(&self) -> Dimension {
        match self.0.dimension {
            2 => Dimension::Planar,
            _ => Dimension::Spatial,
        }
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = TriggerEvent> + 'a {
        self.0.iter().map(|e| TriggerEvent {
            first: EntityId(e.first),
            second: EntityId(e.second),
            entered: e.entered != 0,
        })
    }
}
pub(crate) fn error(e: abi::physics::Error) -> Error {
    match e {
        abi::physics::Error::Bounds => Error::Bounds,
        abi::physics::Error::Malformed => Error::Malformed,
    }
}
