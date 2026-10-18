//! Shared initial-property validation/canonical encoding. No project or VM.
use crate::CoroutineValue;
use std::collections::BTreeMap;
pub const MAX_INSTANCES: usize = 32_768;
pub const MAX_PROP_NODES: usize = 65_536;
pub const MAX_PROP_BYTES: usize = 1024 * 1024;
pub const MAX_PROP_DEPTH: usize = 16;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyError {
    Limit,
    NonFinite,
}
#[derive(Default)]
pub struct PropertyBudget {
    nodes: usize,
    bytes: usize,
}
impl PropertyBudget {
    pub fn add(&mut self, depth: usize, bytes: usize) -> Result<(), PropertyError> {
        self.nodes = self.nodes.checked_add(1).ok_or(PropertyError::Limit)?;
        self.bytes = self.bytes.checked_add(bytes).ok_or(PropertyError::Limit)?;
        if depth > MAX_PROP_DEPTH || self.nodes > MAX_PROP_NODES || self.bytes > MAX_PROP_BYTES {
            return Err(PropertyError::Limit);
        }
        Ok(())
    }
    pub fn text_size(&mut self, text: &str) -> Result<(), PropertyError> {
        self.add(0, text.len().checked_add(8).ok_or(PropertyError::Limit)?)
    }
    fn text(&mut self, text: &str, emit: &mut impl FnMut(&[u8])) -> Result<(), PropertyError> {
        self.text_size(text)?;
        emit(&(text.len() as u64).to_le_bytes());
        emit(text.as_bytes());
        Ok(())
    }
    pub fn object(
        &mut self,
        values: &BTreeMap<String, CoroutineValue>,
        depth: usize,
        emit: &mut impl FnMut(&[u8]),
    ) -> Result<(), PropertyError> {
        self.add(depth, 8)?;
        emit(&[5]);
        emit(&(values.len() as u64).to_le_bytes());
        for (key, value) in values {
            self.text(key, emit)?;
            self.value(value, depth + 1, emit)?;
        }
        Ok(())
    }
    fn value(
        &mut self,
        value: &CoroutineValue,
        depth: usize,
        emit: &mut impl FnMut(&[u8]),
    ) -> Result<(), PropertyError> {
        self.add(depth, 1)?;
        match value {
            CoroutineValue::Null => emit(&[0]),
            CoroutineValue::Bool(value) => emit(&[1, u8::from(*value)]),
            CoroutineValue::Number(value) => {
                if !value.is_finite() {
                    return Err(PropertyError::NonFinite);
                }
                emit(&[2]);
                emit(&value.to_bits().to_le_bytes());
            }
            CoroutineValue::String(value) => {
                emit(&[3]);
                self.text(value, emit)?;
            }
            CoroutineValue::Array(values) => {
                emit(&[4]);
                emit(&(values.len() as u64).to_le_bytes());
                for value in values {
                    self.value(value, depth + 1, emit)?;
                }
            }
            CoroutineValue::Object(values) => self.object(values, depth, emit)?,
        }
        Ok(())
    }
}
