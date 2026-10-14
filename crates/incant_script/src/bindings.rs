//! Startup-only typed attachment data. No authoring or JSON dependency.
use crate::ScriptError;
use incant_runtime::{EntityIdentity, NativeWorld, StableId};
use rquickjs::{Array, Ctx, Function, Object, Persistent, Value, object::Property};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[cfg(test)]
mod cache_tests;

pub const MAX_SCRIPT_SOURCES: usize = 64;
pub const MAX_SCRIPT_INSTANCES: usize = 32_768;
const MAX_PROP_NODES: usize = 65_536;
const MAX_PROP_BYTES: usize = 1024 * 1024;
const MAX_PROP_DEPTH: usize = 16;

/// Initial data only. Module-local mutable state is separate and is not a save
/// format. Numeric values must be finite; keys are ordinary own data properties.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<ScriptValue>),
    Object(BTreeMap<String, ScriptValue>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptInstance {
    pub entity: StableId,
    pub source: StableId,
    pub props: BTreeMap<String, ScriptValue>,
}

/// A bounded canonical attachment catalog paired to a cooked scene. The hash
/// prevents accidental scene/program mixing; it never authenticates bytecode.
#[derive(Clone, Debug)]
pub struct ScriptBindings {
    pub(crate) scene_sha256: [u8; 32],
    pub(crate) sources: Vec<StableId>,
    pub(crate) instances: Vec<ScriptInstance>,
    fingerprint: [u8; 32],
}
impl ScriptBindings {
    pub fn new(
        scene_sha256: [u8; 32],
        mut sources: Vec<StableId>,
        mut instances: Vec<ScriptInstance>,
    ) -> Result<Self, ScriptError> {
        if sources.is_empty()
            || sources.len() > MAX_SCRIPT_SOURCES
            || instances.is_empty()
            || instances.len() > MAX_SCRIPT_INSTANCES
        {
            return Err(ScriptError::InputLimit);
        }
        sources.sort();
        instances.sort_by_key(|instance| instance.entity);
        if sources.windows(2).any(|pair| pair[0] == pair[1])
            || instances
                .windows(2)
                .any(|pair| pair[0].entity == pair[1].entity)
        {
            return Err(ScriptError::ScriptBindings);
        }
        let used: BTreeSet<_> = instances.iter().map(|instance| instance.source).collect();
        if used.iter().copied().collect::<Vec<_>>() != sources {
            return Err(ScriptError::ScriptBindings);
        }
        let mut budget = PropsBudget::default();
        let mut hash = Sha256::new();
        hash.update(b"incant-native-script-bindings-v1");
        hash.update(scene_sha256);
        hash.update((sources.len() as u64).to_le_bytes());
        for source in &sources {
            hash.update(source.0);
        }
        hash.update((instances.len() as u64).to_le_bytes());
        for instance in &instances {
            hash.update(instance.entity.0);
            hash.update(instance.source.0);
            budget.object(&instance.props, 0, &mut hash)?;
        }
        Ok(Self {
            scene_sha256,
            sources,
            instances,
            fingerprint: hash.finalize().into(),
        })
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn sources(&self) -> &[StableId] {
        &self.sources
    }
    pub fn instances(&self) -> &[ScriptInstance] {
        &self.instances
    }
}

#[derive(Default)]
pub(crate) struct PropsBudget {
    nodes: usize,
    bytes: usize,
}
impl PropsBudget {
    pub(crate) fn add(&mut self, depth: usize, bytes: usize) -> Result<(), ScriptError> {
        self.nodes += 1;
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(ScriptError::InputLimit)?;
        if depth > MAX_PROP_DEPTH || self.nodes > MAX_PROP_NODES || self.bytes > MAX_PROP_BYTES {
            return Err(ScriptError::InputLimit);
        }
        Ok(())
    }
    pub(crate) fn text_size(&mut self, text: &str) -> Result<(), ScriptError> {
        self.add(0, text.len().checked_add(8).ok_or(ScriptError::InputLimit)?)
    }
    fn text(&mut self, text: &str, hash: &mut Sha256) -> Result<(), ScriptError> {
        self.text_size(text)?;
        hash.update((text.len() as u64).to_le_bytes());
        hash.update(text.as_bytes());
        Ok(())
    }
    fn object(
        &mut self,
        values: &BTreeMap<String, ScriptValue>,
        depth: usize,
        hash: &mut Sha256,
    ) -> Result<(), ScriptError> {
        self.add(depth, 8)?;
        hash.update([5]);
        hash.update((values.len() as u64).to_le_bytes());
        for (key, value) in values {
            self.text(key, hash)?;
            self.value(value, depth + 1, hash)?;
        }
        Ok(())
    }
    fn value(
        &mut self,
        value: &ScriptValue,
        depth: usize,
        hash: &mut Sha256,
    ) -> Result<(), ScriptError> {
        self.add(depth, 1)?;
        match value {
            ScriptValue::Null => hash.update([0]),
            ScriptValue::Bool(value) => hash.update([1, u8::from(*value)]),
            ScriptValue::Number(value) => {
                if !value.is_finite() {
                    return Err(ScriptError::ScriptBindings);
                }
                hash.update([2]);
                hash.update(value.to_bits().to_le_bytes());
            }
            ScriptValue::String(value) => {
                hash.update([3]);
                self.text(value, hash)?;
            }
            ScriptValue::Array(values) => {
                hash.update([4]);
                hash.update((values.len() as u64).to_le_bytes());
                for value in values {
                    self.value(value, depth + 1, hash)?;
                }
            }
            ScriptValue::Object(values) => self.object(values, depth, hash)?,
        }
        Ok(())
    }
}

pub(crate) struct BoundProject {
    pub groups: Vec<BoundGroup>,
    pub incarnation: incant_runtime::WorldIncarnation,
}
pub(crate) struct BoundGroup {
    pub source: StableId,
    instances: Vec<(StableId, EntityIdentity, Persistent<Object<'static>>)>,
    snapshot: Option<GroupSnapshot>,
}

// Immutable attachment data only. Tick capabilities and numeric loans must
// never be retained here. This root drops with BoundProject before its VM.
struct GroupSnapshot {
    array: Persistent<Array<'static>>,
    ids: Arc<[StableId]>,
}
impl BoundProject {
    /// Build and deeply freeze props before any guest initialization runs.
    pub fn new<'js>(
        ctx: &Ctx<'js>,
        freeze: &Function<'js>,
        bindings: &ScriptBindings,
        world: &NativeWorld,
    ) -> Result<Self, ScriptError> {
        Self::build(ctx, freeze, bindings, world, None)
    }
    pub(crate) fn restore<'js>(
        ctx: &Ctx<'js>,
        freeze: &Function<'js>,
        bindings: &ScriptBindings,
        world: &NativeWorld,
        active: &[StableId],
    ) -> Result<Self, ScriptError> {
        Self::build(ctx, freeze, bindings, world, Some(active))
    }
    fn build<'js>(
        ctx: &Ctx<'js>,
        freeze: &Function<'js>,
        bindings: &ScriptBindings,
        world: &NativeWorld,
        active: Option<&[StableId]>,
    ) -> Result<Self, ScriptError> {
        if (active.is_none() && world.tick() != 0)
            || world.source_scene_fingerprint() != bindings.scene_sha256
        {
            return Err(ScriptError::ScriptBindings);
        }
        if let Some(active) = active
            && (active.windows(2).any(|p| p[0] >= p[1])
                || active.iter().any(|id| {
                    bindings
                        .instances
                        .binary_search_by_key(id, |i| i.entity)
                        .is_err()
                }))
        {
            return Err(ScriptError::ScriptBindings);
        }
        let mut groups: Vec<_> = bindings
            .sources
            .iter()
            .map(|&source| BoundGroup {
                source,
                instances: Vec::new(),
                snapshot: None,
            })
            .collect();
        for instance in &bindings.instances {
            if active.is_some_and(|ids| ids.binary_search(&instance.entity).is_err()) {
                continue;
            }
            let identity = world
                .entity_identity(instance.entity)
                .ok_or(ScriptError::ScriptBindings)?;
            let group = bindings
                .sources
                .binary_search(&instance.source)
                .map_err(|_| ScriptError::ScriptBindings)?;
            let object =
                Object::new_proto(ctx.clone(), None).map_err(crate::diagnostics::allocation)?;
            let props = expose_object(ctx, freeze, &instance.props)
                .map_err(crate::diagnostics::allocation)?;
            object
                .prop(
                    "entity",
                    Property::from(ulid::Ulid::from_bytes(instance.entity.0).to_string())
                        .enumerable(),
                )
                .map_err(crate::diagnostics::allocation)?;
            object
                .prop("props", Property::from(props).enumerable())
                .map_err(crate::diagnostics::allocation)?;
            freeze
                .call::<_, ()>((object.clone(),))
                .map_err(crate::diagnostics::allocation)?;
            groups[group].instances.push((
                instance.entity,
                identity,
                Persistent::save(ctx, object),
            ));
        }
        Ok(Self {
            groups,
            incarnation: world.incarnation(),
        })
    }
}
impl BoundProject {
    pub fn active_ids(&self, world: &NativeWorld) -> Result<Vec<StableId>, ScriptError> {
        if self.incarnation != world.incarnation() {
            return Err(ScriptError::ScriptBindings);
        }
        let mut ids: Vec<_> = self
            .groups
            .iter()
            .flat_map(|g| g.instances.iter())
            .filter(|(id, identity, _)| world.entity_identity(*id) == Some(*identity))
            .map(|v| v.0)
            .collect();
        ids.sort_unstable();
        Ok(ids)
    }
}
impl BoundGroup {
    pub fn snapshot<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        freeze: &Function<'js>,
        world: &NativeWorld,
        retired: &mut Vec<crate::persistence::Key>,
    ) -> rquickjs::Result<(Array<'js>, Arc<[StableId]>)> {
        // A committed despawn retires an attachment forever. Stable-ID reuse,
        // even between script ticks, cannot reconnect it to a new ECS lifetime.
        let previous_len = self.instances.len();
        self.instances.retain(|(id, identity, _)| {
            let live = world.entity_identity(*id) == Some(*identity);
            if !live {
                retired.push(crate::persistence::Key {
                    scope: crate::schedule::Scope::Source(self.source),
                    entity: Some(*id),
                });
            }
            live
        });
        if self.instances.len() != previous_len {
            // Invalidate both views before any fallible construction. Previously
            // returned frozen arrays remain valid historical data, not authority.
            self.snapshot = None;
        }
        if let Some(snapshot) = &self.snapshot {
            return Ok((
                snapshot.array.clone().restore(ctx)?,
                Arc::clone(&snapshot.ids),
            ));
        }
        let array = Array::new(ctx.clone())?;
        for (index, (_, _, instance)) in self.instances.iter().enumerate() {
            array.as_object().prop(
                index as u32,
                Property::from(instance.clone().restore(ctx)?).enumerable(),
            )?;
        }
        freeze.call::<_, ()>((array.clone(),))?;
        let ids: Arc<[StableId]> = self.instances.iter().map(|v| v.0).collect();
        self.snapshot = Some(GroupSnapshot {
            array: Persistent::save(ctx, array.clone()),
            ids: Arc::clone(&ids),
        });
        Ok((array, ids))
    }
}
fn expose_object<'js>(
    ctx: &Ctx<'js>,
    freeze: &Function<'js>,
    values: &BTreeMap<String, ScriptValue>,
) -> rquickjs::Result<Object<'js>> {
    let object = Object::new_proto(ctx.clone(), None)?;
    for (key, value) in values {
        object.prop(
            key.as_str(),
            Property::from(expose_value(ctx, freeze, value)?).enumerable(),
        )?;
    }
    freeze.call::<_, ()>((object.clone(),))?;
    Ok(object)
}
pub(crate) fn expose_value<'js>(
    ctx: &Ctx<'js>,
    freeze: &Function<'js>,
    value: &ScriptValue,
) -> rquickjs::Result<Value<'js>> {
    use rquickjs::IntoJs;
    match value {
        ScriptValue::Null => Ok(Value::new_null(ctx.clone())),
        ScriptValue::Bool(value) => value.into_js(ctx),
        ScriptValue::Number(value) => value.into_js(ctx),
        ScriptValue::String(value) => value.into_js(ctx),
        ScriptValue::Object(values) => Ok(expose_object(ctx, freeze, values)?.into_value()),
        ScriptValue::Array(values) => {
            let array = Array::new(ctx.clone())?;
            for (index, value) in values.iter().enumerate() {
                array.as_object().prop(
                    index as u32,
                    Property::from(expose_value(ctx, freeze, value)?).enumerable(),
                )?;
            }
            freeze.call::<_, ()>((array.clone(),))?;
            Ok(array.into_value())
        }
    }
}
