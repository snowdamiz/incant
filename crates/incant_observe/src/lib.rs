//! Explicit bounded observation of accepted native ticks. The startup plan is
//! data, never code. Checks inspect only requested values; no world snapshot or
//! JSON conversion occurs on unobserved ticks. Results serialize once at exit.
#[derive(Debug, thiserror::Error)]
pub enum ObservationError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
impl From<&'static str> for ObservationError {
    fn from(message: &'static str) -> Self {
        Self::Invalid(message)
    }
}
pub type Result<T> = std::result::Result<T, ObservationError>;
use incant_input::{ActionState, InputFrame};
use incant_runtime::{NativeWorld, StableId};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAX_PLAN_BYTES: usize = 256 * 1024;
const MAX_ITEMS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct EntityId(StableId);
impl<'de> Deserialize<'de> for EntityId {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> std::result::Result<Self, D::Error> {
        let text = String::deserialize(decoder)?;
        let id = text
            .parse::<ulid::Ulid>()
            .map_err(serde::de::Error::custom)?;
        if id.to_string() != text {
            return Err(serde::de::Error::custom(
                "expected canonical uppercase ULID",
            ));
        }
        Ok(Self(StableId(id.to_bytes())))
    }
}
impl Serialize for EntityId {
    fn serialize<S: Serializer>(&self, encoder: S) -> std::result::Result<S::Ok, S::Error> {
        encoder.collect_str(&ulid::Ulid::from_bytes(self.0.0))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    format: String,
    version: u32,
    #[serde(default)]
    checks: Vec<Check>,
    #[serde(default)]
    entities: Vec<EntityId>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Check {
    name: String,
    tick: u64,
    expect: Expect,
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Expect {
    EntityCount {
        value: usize,
    },
    Exists {
        entity: EntityId,
        value: bool,
    },
    Vector {
        entity: EntityId,
        property: VectorProperty,
        value: [f64; 3],
        tolerance: f64,
    },
    Action {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        active: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pressed: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        released: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<[f64; 2]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tolerance: Option<f64>,
    },
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum VectorProperty {
    Translation,
    WorldTranslation,
    Velocity,
    PhysicsTranslation,
    PhysicsVelocity,
}
#[derive(Serialize)]
#[serde(untagged)]
enum Actual {
    Count(usize),
    Exists(bool),
    Vector([f64; 3]),
    Action(ActionState),
}
#[derive(Serialize)]
struct Outcome {
    #[serde(flatten)]
    check: Check,
    passed: bool,
    actual: Option<Actual>,
}

fn tolerance(value: f64) -> bool {
    value.is_finite() && value >= 0.
}
fn close<const N: usize>(a: &[f64; N], b: &[f64; N], tolerance: f64) -> bool {
    a.iter()
        .zip(b)
        .all(|(a, b)| a.is_finite() && b.is_finite() && (a - b).abs() <= tolerance)
}
impl Expect {
    fn validate(&self) -> bool {
        match self {
            Self::Vector {
                value,
                tolerance: eps,
                ..
            } => value.iter().all(|v| v.is_finite()) && tolerance(*eps),
            Self::Action {
                name,
                active,
                pressed,
                released,
                value,
                tolerance: eps,
            } => {
                !name.is_empty()
                    && name.len() <= 64
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
                    && (active.is_some()
                        || pressed.is_some()
                        || released.is_some()
                        || value.is_some())
                    && match (value, eps) {
                        (Some(value), Some(eps)) => {
                            value.iter().all(|v| v.is_finite()) && tolerance(*eps)
                        }
                        (None, None) => true,
                        _ => false,
                    }
            }
            _ => true,
        }
    }
    fn observe(&self, world: &NativeWorld, input: &InputFrame) -> Option<Actual> {
        Some(match self {
            Self::EntityCount { .. } => Actual::Count(world.entity_count()),
            Self::Exists { entity, .. } => Actual::Exists(world.inspect(entity.0).is_some()),
            Self::Vector {
                entity, property, ..
            } => Actual::Vector(match property {
                VectorProperty::Translation => world.inspect(entity.0)?.transform.translation,
                VectorProperty::WorldTranslation => world.inspect(entity.0)?.world_transform[3]
                    [..3]
                    .try_into()
                    .ok()?,
                VectorProperty::Velocity => world.inspect(entity.0)?.velocity?.linear,
                VectorProperty::PhysicsTranslation => world.physics_state(entity.0)?.translation,
                VectorProperty::PhysicsVelocity => world.physics_state(entity.0)?.velocity,
            }),
            Self::Action { name, .. } => Actual::Action(input.actions.get(name)?.clone()),
        })
    }
    fn matches(&self, actual: &Option<Actual>) -> bool {
        match (self, actual) {
            (Self::EntityCount { value }, Some(Actual::Count(actual))) => value == actual,
            (Self::Exists { value, .. }, Some(Actual::Exists(actual))) => value == actual,
            (
                Self::Vector {
                    value, tolerance, ..
                },
                Some(Actual::Vector(actual)),
            ) => close(value, actual, *tolerance),
            (
                Self::Action {
                    active,
                    pressed,
                    released,
                    value,
                    tolerance,
                    ..
                },
                Some(Actual::Action(actual)),
            ) => {
                active.is_none_or(|v| v == actual.active)
                    && pressed.is_none_or(|v| v == actual.pressed)
                    && released.is_none_or(|v| v == actual.released)
                    && value.is_none_or(|v| close(&v, &actual.value, tolerance.unwrap_or(0.)))
            }
            _ => false,
        }
    }
}

pub struct Plan {
    checks: VecDeque<Check>,
    results: Vec<Outcome>,
    entities: BTreeSet<EntityId>,
}
impl Plan {
    pub fn parse(bytes: &[u8], end_tick: u64) -> Result<Self> {
        if bytes.len() > MAX_PLAN_BYTES {
            return Err("native observation plan exceeds 256 KiB".into());
        }
        let mut request: Request = serde_json::from_slice(bytes)?;
        if request.format != "incant-native-observation" || request.version != 1 {
            return Err("unsupported native observation format/version".into());
        }
        if request.checks.len() > MAX_ITEMS
            || request.entities.len() > MAX_ITEMS
            || (request.checks.is_empty() && request.entities.is_empty())
        {
            return Err(
                "native observation needs checks or selected entities; each limit is 256".into(),
            );
        }
        let mut names = BTreeSet::new();
        for check in &request.checks {
            if check.name.is_empty()
                || check.name.len() > 120
                || check.name.chars().any(char::is_control)
                || !names.insert(check.name.clone())
                || check.tick > end_tick
                || !check.expect.validate()
            {
                return Err("invalid native observation check name, tick or expectation".into());
            }
        }
        let entities: BTreeSet<_> = request.entities.iter().copied().collect();
        if entities.len() != request.entities.len() {
            return Err("duplicate native observation entity".into());
        }
        request
            .checks
            .sort_by(|a, b| (a.tick, &a.name).cmp(&(b.tick, &b.name)));
        Ok(Self {
            checks: request.checks.into(),
            results: Vec::new(),
            entities,
        })
    }
    /// Called once at startup, then after each whole accepted game tick. Reads
    /// neither documents nor VM state. Missing entities/components/actions fail
    /// their check explicitly instead of substituting a zero/default value.
    pub fn observe(&mut self, world: &NativeWorld, input: &InputFrame) -> Result<()> {
        if self.checks.front().is_some_and(|c| c.tick < world.tick()) {
            return Err("missed requested native observation tick".into());
        }
        while self.checks.front().is_some_and(|c| c.tick == world.tick()) {
            let check = self.checks.pop_front().expect("checked front");
            let actual = check.expect.observe(world, input);
            let passed = check.expect.matches(&actual);
            self.results.push(Outcome {
                check,
                passed,
                actual,
            });
        }
        Ok(())
    }
    pub fn finish(self, world: &NativeWorld) -> Result<Report> {
        if !self.checks.is_empty() {
            return Err("native observation finished before all checks".into());
        }
        let entities = self
            .entities
            .into_iter()
            .map(|id| {
                let value = world.inspect(id.0).map(|entity| Entity {
                    parent: entity.parent.map(EntityId),
                    translation: entity.transform.translation,
                    rotation: entity.transform.rotation,
                    scale: entity.transform.scale,
                    velocity: entity.velocity.map(|v| v.linear),
                    world_transform: entity.world_transform,
                });
                (id, value)
            })
            .collect();
        Ok(Report {
            passed: self.results.iter().all(|r| r.passed),
            checks: self.results,
            entities,
            tick: world.tick(),
            tick_rate: world.tick_rate(),
        })
    }
}
#[derive(Serialize)]
struct Entity {
    parent: Option<EntityId>,
    translation: [f64; 3],
    rotation: [f64; 4],
    scale: [f64; 3],
    velocity: Option<[f64; 3]>,
    world_transform: [[f64; 4]; 4],
}
#[derive(Serialize)]
pub struct Report {
    pub passed: bool,
    tick: u64,
    tick_rate: u32,
    checks: Vec<Outcome>,
    entities: BTreeMap<EntityId, Option<Entity>>,
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
