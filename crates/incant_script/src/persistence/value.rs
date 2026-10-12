//! Checkpoint values are finite typed data, never executable/VM objects.
use crate::{ScriptError, ScriptValue};
use rquickjs::{Array, Ctx, Function, Value};
use std::collections::BTreeMap;

pub(crate) const MAX_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_NODES: usize = 65_536;
pub(crate) const MAX_DEPTH: usize = 16;
#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct Cost {
    pub bytes: usize,
    pub nodes: usize,
}
impl Cost {
    pub fn add(&mut self, bytes: usize, nodes: usize) -> Result<(), ScriptError> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(ScriptError::InputLimit)?;
        self.nodes = self
            .nodes
            .checked_add(nodes)
            .ok_or(ScriptError::InputLimit)?;
        if self.bytes > MAX_BYTES || self.nodes > MAX_NODES {
            return Err(ScriptError::InputLimit);
        }
        Ok(())
    }
}
fn invalid(ctx: &Ctx<'_>) -> rquickjs::Error {
    rquickjs::Exception::throw_type(ctx, "persistent state requires bounded finite data")
}
pub(crate) fn text(ctx: &Ctx<'_>, value: &Value<'_>, max: usize) -> rquickjs::Result<String> {
    let value = value.as_string().ok_or_else(|| invalid(ctx))?;
    let bytes = value.clone().to_cstring()?;
    if bytes.len() > max {
        return Err(invalid(ctx));
    }
    drop(bytes);
    value.to_string().map_err(|_| invalid(ctx))
}
/// The fixed hardening inspector rejects accessors, symbols and class objects.
/// It may run proxy traps; callers must hold no Rust borrow across this call.
pub(crate) fn parse<'js>(
    ctx: &Ctx<'js>,
    inspect: &Function<'js>,
    value: Value<'js>,
    cost: &mut Cost,
    depth: usize,
) -> rquickjs::Result<ScriptValue> {
    if depth > MAX_DEPTH {
        return Err(invalid(ctx));
    }
    cost.add(1, 1).map_err(|_| invalid(ctx))?;
    if value.is_null() {
        return Ok(ScriptValue::Null);
    }
    if let Some(value) = value.as_bool() {
        return Ok(ScriptValue::Bool(value));
    }
    if let Some(value) = value.as_number() {
        if !value.is_finite() {
            return Err(invalid(ctx));
        }
        cost.add(8, 0).map_err(|_| invalid(ctx))?;
        return Ok(ScriptValue::Number(value));
    }
    if value.is_string() {
        let value = text(ctx, &value, MAX_BYTES - cost.bytes)?;
        cost.add(4 + value.len(), 0).map_err(|_| invalid(ctx))?;
        return Ok(ScriptValue::String(value));
    }
    if !value.is_object() || value.is_function() {
        return Err(invalid(ctx));
    }
    let array = value.is_array();
    let rows: Array = inspect.call((value, (MAX_NODES - cost.nodes) as u32))?;
    cost.add(4, 0).map_err(|_| invalid(ctx))?;
    if array {
        let mut out = Vec::new();
        for entry in rows.iter::<Value>() {
            out.push(parse(ctx, inspect, entry?, cost, depth + 1)?);
        }
        Ok(ScriptValue::Array(out))
    } else {
        let mut out = BTreeMap::new();
        for entry in rows.iter::<Array>() {
            let entry = entry?;
            let key = text(ctx, &entry.get(0)?, MAX_BYTES - cost.bytes)?;
            cost.add(4 + key.len(), 0).map_err(|_| invalid(ctx))?;
            let value = parse(ctx, inspect, entry.get(1)?, cost, depth + 1)?;
            if out.insert(key, value).is_some() {
                return Err(invalid(ctx));
            }
        }
        Ok(ScriptValue::Object(out))
    }
}
