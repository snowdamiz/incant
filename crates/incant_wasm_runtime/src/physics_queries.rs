//! Callback-scoped solver authority and one completely copied reply.
use crate::BoundaryError as Error;
use incant_runtime::{CharacterOptions, CharacterQuery, CharacterStep, NativeQueries, StableId};
use incant_wasm_abi::{self as abi, physics as wire};

pub(crate) struct State {
    queries: Option<NativeQueries>,
    ever_bound: bool,
    dt: f64,
    characters: usize,
    reply: Option<Vec<u8>>,
    pub rejected: bool,
}
impl State {
    pub fn new(dt: f64) -> Self {
        Self {
            queries: None,
            ever_bound: false,
            dt,
            characters: 0,
            reply: None,
            rejected: false,
        }
    }
    pub fn bind(&mut self, queries: NativeQueries) -> Result<(), Error> {
        if self.ever_bound || self.rejected || !queries.is_active() {
            return Err(Error::Abi("invalid physics loan"));
        }
        self.ever_bound = true;
        self.queries = Some(queries);
        Ok(())
    }
    pub fn revoke(&mut self) {
        self.queries = None;
        self.reply = None;
    }
    pub fn bound(&self) -> bool {
        self.queries.is_some()
    }
    pub fn reject(&mut self) {
        self.rejected = true;
        self.reply = None;
    }
    fn queries(&self) -> Result<&NativeQueries, Error> {
        if self.rejected {
            return Err(Error::Abi("rejected physics transaction"));
        }
        self.queries
            .as_ref()
            .filter(|q| q.is_active())
            .ok_or(Error::Abi("expired physics loan"))
    }
    pub fn reply(&self) -> Result<&[u8], Error> {
        self.queries()?;
        self.reply.as_deref().ok_or(Error::Abi("no physics result"))
    }
    pub fn query(&mut self, batch: wire::Batch<'_>) -> Result<i32, Error> {
        self.reply = None;
        self.queries()?;
        self.characters = self
            .characters
            .checked_add(batch.iter().filter(|q| q.kind == 2).count())
            .filter(|n| *n <= abi::PHYSICS_MAX_CHARACTER_QUERIES)
            .ok_or(Error::Limit("character query quota"))?;
        let mut rows = Vec::with_capacity(batch.len());
        let mut collisions = Vec::new();
        let queries = self.queries()?;
        for q in batch.iter() {
            let mut row = abi::PhysicsRow {
                kind: q.kind,
                flags: 0,
                entity: [0; 16],
                distance: 0.,
                point: [0.; 3],
                normal: [0.; 3],
                first: 0,
                count: 0,
                reserved: 0,
            };
            match q.kind {
                0 => {
                    if let Some(h) = queries.raycast(&incant_runtime::RayQuery {
                        scene_id: queries.scene_id(),
                        origin: q.a,
                        direction: q.b,
                        max_distance: q.c,
                        include_sensors: q.flags & 1 != 0,
                        exclude_entity: (q.flags & 2 != 0).then_some(StableId(q.entity)),
                        memberships: q.memberships,
                        filter: q.filter,
                    })? {
                        row.flags = 1;
                        row.entity = h.entity_id.0;
                        row.distance = h.distance;
                        row.point = h.point;
                        row.normal = h.normal;
                    }
                }
                1 => {
                    if let Some(h) = queries.raycast_2d(&incant_runtime::RayQuery2D {
                        origin: [q.a[0], q.a[1]],
                        direction: [q.b[0], q.b[1]],
                        max_distance: q.c,
                        include_sensors: q.flags & 1 != 0,
                        exclude_entity: (q.flags & 2 != 0).then_some(StableId(q.entity)),
                        memberships: q.memberships,
                        filter: q.filter,
                    })? {
                        row.flags = 1;
                        row.entity = h.entity.0;
                        row.distance = h.distance;
                        row.point = [h.point[0], h.point[1], 0.];
                        row.normal = [h.normal[0], h.normal[1], 0.];
                    }
                }
                2 => {
                    let motion = queries.compute_character_motion(
                        &CharacterQuery {
                            scene_id: queries.scene_id(),
                            entity_id: StableId(q.entity),
                            translation: q.a,
                            options: CharacterOptions {
                                offset: q.b[0],
                                slide: q.flags & 1 != 0,
                                max_slope_climb_angle: q.b[1],
                                min_slope_slide_angle: q.b[2],
                                snap_to_ground: (q.flags & 2 != 0).then_some(q.c),
                                autostep: (q.flags & 4 != 0).then_some(CharacterStep {
                                    max_height: q.d,
                                    min_width: q.e,
                                    include_dynamic_bodies: q.flags & 8 != 0,
                                }),
                            },
                        },
                        self.dt,
                    )?;
                    if motion.collisions.len() > abi::PHYSICS_MAX_COLLISIONS - collisions.len() {
                        return Err(Error::Limit("physics collision result"));
                    }
                    row.flags =
                        u32::from(motion.grounded) | (u32::from(motion.sliding_down_slope) << 1);
                    row.point = motion.translation;
                    row.first = collisions.len() as u32;
                    row.count = motion.collisions.len() as u32;
                    collisions.extend(motion.collisions.into_iter().map(|id| id.0));
                }
                _ => return Err(Error::Abi("physics query kind")),
            }
            rows.push(row);
        }
        let mut result = Vec::with_capacity(
            abi::PhysicsHeader::SIZE + rows.len() * abi::PhysicsRow::SIZE + collisions.len() * 16,
        );
        result.extend_from_slice(
            &abi::PhysicsHeader {
                tag: *abi::PHYSICS_RESULT_MAGIC,
                count: rows.len() as u32,
                extra: collisions.len() as u32,
            }
            .encode(),
        );
        for row in rows {
            result.extend_from_slice(&row.encode());
        }
        for id in collisions {
            result.extend_from_slice(&id);
        }
        wire::decode_results(&result).map_err(|_| Error::Abi("native physics result bounds"))?;
        self.publish(result)
    }
    pub fn events(&mut self, dimension: i32) -> Result<i32, Error> {
        self.reply = None;
        let queries = self.queries()?;
        let rows: Vec<abi::PhysicsEvent> = match dimension {
            2 => queries
                .trigger_events_2d()?
                .into_iter()
                .map(|e| abi::PhysicsEvent {
                    first: e.first.0,
                    second: e.second.0,
                    entered: u32::from(e.entered),
                    reserved: 0,
                })
                .collect(),
            3 => {
                let events = queries.trigger_events()?;
                if events.iter().any(|e| e.scene_id != queries.scene_id()) {
                    return Err(Error::Abi("physics event scene"));
                }
                events
                    .into_iter()
                    .map(|e| abi::PhysicsEvent {
                        first: e.first.0,
                        second: e.second.0,
                        entered: u32::from(e.entered),
                        reserved: 0,
                    })
                    .collect()
            }
            _ => return Err(Error::Abi("physics event dimension")),
        };
        if rows.len() > abi::PHYSICS_MAX_EVENTS {
            return Err(Error::Limit("physics event result"));
        }
        let mut result = Vec::with_capacity(
            abi::PhysicsEventHeader::SIZE + rows.len() * abi::PhysicsEvent::SIZE,
        );
        result.extend_from_slice(
            &abi::PhysicsEventHeader {
                tag: *abi::PHYSICS_EVENTS_MAGIC,
                scene: queries.scene_id().0,
                count: rows.len() as u32,
                dimension: dimension as u32,
            }
            .encode(),
        );
        for row in rows {
            result.extend_from_slice(&row.encode());
        }
        wire::decode_events(&result).map_err(|_| Error::Abi("native physics event ordering"))?;
        self.publish(result)
    }
    fn publish(&mut self, result: Vec<u8>) -> Result<i32, Error> {
        let len = result.len() as i32;
        self.reply = Some(result);
        Ok(len)
    }
}
