//! Checked allocation-free views over canonical initial properties. No guest JSON.
use crate::{INSTANCE_MAGIC, INSTANCE_MAX_BYTES, INSTANCE_MAX_ROWS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invalid;
type Result<T> = core::result::Result<T, Invalid>;
pub const HEADER: usize = crate::InstanceHeader::SIZE;
pub const ROW: usize = crate::InstanceRow::SIZE;
pub const MAX_CHECKPOINT: usize = crate::InstanceMembershipHeader::SIZE + INSTANCE_MAX_ROWS * 16;

fn take<'a>(bytes: &mut &'a [u8], len: usize) -> Result<&'a [u8]> {
    let (head, tail) = bytes.split_at_checked(len).ok_or(Invalid)?;
    *bytes = tail;
    Ok(head)
}
fn count(bytes: &mut &[u8]) -> Result<usize> {
    usize::try_from(u64::from_le_bytes(take(bytes, 8)?.try_into().unwrap())).map_err(|_| Invalid)
}
fn text<'a>(bytes: &mut &'a [u8]) -> Result<&'a str> {
    let len = count(bytes)?;
    core::str::from_utf8(take(bytes, len)?).map_err(|_| Invalid)
}
#[derive(Default)]
struct Budget {
    nodes: usize,
    bytes: usize,
}
impl Budget {
    fn add(&mut self, depth: usize, bytes: usize) -> Result<()> {
        self.nodes = self.nodes.checked_add(1).ok_or(Invalid)?;
        self.bytes = self.bytes.checked_add(bytes).ok_or(Invalid)?;
        if depth > crate::INSTANCE_MAX_PROP_DEPTH
            || self.nodes > crate::INSTANCE_MAX_PROP_NODES
            || self.bytes > crate::INSTANCE_MAX_PROP_BYTES
        {
            return Err(Invalid);
        }
        Ok(())
    }
    fn text<'a>(&mut self, bytes: &mut &'a [u8]) -> Result<&'a str> {
        let value = text(bytes)?;
        self.add(0, value.len().checked_add(8).ok_or(Invalid)?)?;
        Ok(value)
    }
    fn object(&mut self, bytes: &mut &[u8], depth: usize) -> Result<()> {
        self.add(depth, 8)?;
        let len = count(bytes)?;
        if len > crate::INSTANCE_MAX_PROP_NODES {
            return Err(Invalid);
        }
        let mut previous = None;
        for _ in 0..len {
            let key = self.text(bytes)?;
            if previous.is_some_and(|old| old >= key) {
                return Err(Invalid);
            }
            previous = Some(key);
            self.value(bytes, depth + 1)?;
        }
        Ok(())
    }
    fn value(&mut self, bytes: &mut &[u8], depth: usize) -> Result<()> {
        self.add(depth, 1)?;
        match take(bytes, 1)?[0] {
            0 => (),
            1 => {
                if take(bytes, 1)?[0] > 1 {
                    return Err(Invalid);
                }
            }
            2 => {
                if !f64::from_le_bytes(take(bytes, 8)?.try_into().unwrap()).is_finite() {
                    return Err(Invalid);
                }
            }
            3 => {
                self.text(bytes)?;
            }
            4 => {
                let len = count(bytes)?;
                if len > crate::INSTANCE_MAX_PROP_NODES {
                    return Err(Invalid);
                }
                for _ in 0..len {
                    self.value(bytes, depth + 1)?;
                }
            }
            5 => self.object(bytes, depth)?,
            _ => return Err(Invalid),
        }
        Ok(())
    }
}

/// Each view borrows only the caller's copied scratch, never a host/VM pointer.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot<'a> {
    bytes: &'a [u8],
    count: usize,
}
impl<'a> Snapshot<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        if !(HEADER..=INSTANCE_MAX_BYTES).contains(&bytes.len())
            || &bytes[..8] != INSTANCE_MAGIC
            || bytes[8..12] != 1u32.to_le_bytes()
            || bytes[20..24] != [0; 4]
        {
            return Err(Invalid);
        }
        let count = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        if count > INSTANCE_MAX_ROWS
            || u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize
                != bytes.len() - HEADER
        {
            return Err(Invalid);
        }
        let mut tail = &bytes[HEADER..];
        let mut previous = None;
        let mut budget = Budget::default();
        for _ in 0..count {
            let row = take(&mut tail, ROW)?;
            let id: [u8; 16] = row[..16].try_into().unwrap();
            if previous.is_some_and(|old| old >= id) || row[20..] != [0; 4] {
                return Err(Invalid);
            }
            previous = Some(id);
            let len = u32::from_le_bytes(row[16..20].try_into().unwrap()) as usize;
            let mut props = take(&mut tail, len)?;
            if take(&mut props, 1)? != [5] {
                return Err(Invalid);
            }
            budget.object(&mut props, 0)?;
            if !props.is_empty() {
                return Err(Invalid);
            }
        }
        if !tail.is_empty() {
            return Err(Invalid);
        }
        Ok(Self { bytes, count })
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn iter(&self) -> impl Iterator<Item = Instance<'a>> {
        let mut tail = &self.bytes[HEADER..];
        (0..self.count).map(move |_| {
            let row = take(&mut tail, ROW).unwrap();
            let len = u32::from_le_bytes(row[16..20].try_into().unwrap()) as usize;
            Instance {
                entity: row[..16].try_into().unwrap(),
                props: Properties(take(&mut tail, len).unwrap()),
            }
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Instance<'a> {
    pub entity: [u8; 16],
    pub props: Properties<'a>,
}
#[derive(Clone, Copy, Debug)]
pub struct Properties<'a>(&'a [u8]);
impl<'a> Properties<'a> {
    pub fn iter(self) -> impl Iterator<Item = (&'a str, Value<'a>)> {
        let mut bytes = &self.0[1..];
        let n = count(&mut bytes).unwrap();
        (0..n).map(move |_| {
            let key = text(&mut bytes).unwrap();
            (key, next(&mut bytes))
        })
    }
    pub fn get(self, key: &str) -> Option<Value<'a>> {
        self.iter().find(|(name, _)| *name == key).map(|(_, v)| v)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    Number(f64),
    String(&'a str),
    Array(Values<'a>),
    Object(Properties<'a>),
}
#[derive(Clone, Copy, Debug)]
pub struct Values<'a>(&'a [u8]);
impl<'a> Values<'a> {
    pub fn iter(self) -> impl Iterator<Item = Value<'a>> {
        let mut bytes = &self.0[1..];
        let n = count(&mut bytes).unwrap();
        (0..n).map(move |_| next(&mut bytes))
    }
}
fn next<'a>(bytes: &mut &'a [u8]) -> Value<'a> {
    let original = *bytes;
    let tag = take(bytes, 1).unwrap()[0];
    match tag {
        0 => Value::Null,
        1 => Value::Bool(take(bytes, 1).unwrap()[0] != 0),
        2 => Value::Number(f64::from_le_bytes(
            take(bytes, 8).unwrap().try_into().unwrap(),
        )),
        3 => Value::String(text(bytes).unwrap()),
        4 | 5 => {
            let mut budget = Budget::default();
            let mut tail = original;
            budget.value(&mut tail, 0).unwrap();
            let len = original.len() - tail.len();
            *bytes = tail;
            if tag == 4 {
                Value::Array(Values(&original[..len]))
            } else {
                Value::Object(Properties(&original[..len]))
            }
        }
        _ => unreachable!("validated property tag"),
    }
}

/// Data-only checkpoint membership. Catalog digest is receiving-program identity.
pub fn checkpoint(bytes: &[u8]) -> Result<([u8; 32], impl Iterator<Item = [u8; 16]> + '_)> {
    if !(36..=MAX_CHECKPOINT).contains(&bytes.len()) {
        return Err(Invalid);
    }
    let n = u32::from_le_bytes(bytes[32..36].try_into().unwrap()) as usize;
    if n > INSTANCE_MAX_ROWS || bytes.len() != 36 + n * 16 {
        return Err(Invalid);
    }
    let mut previous = None;
    for id in bytes[36..].as_chunks::<16>().0 {
        if previous.is_some_and(|old| old >= *id) {
            return Err(Invalid);
        }
        previous = Some(*id);
    }
    Ok((
        bytes[..32].try_into().unwrap(),
        bytes[36..].as_chunks::<16>().0.iter().copied(),
    ))
}
