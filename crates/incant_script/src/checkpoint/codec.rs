//! Explicit versioned data encoding; never executable/VM deserialization.
use super::{CheckpointError as Error, Result};
use crate::{
    ScriptValue,
    persistence::{
        self, Entry, Store,
        value::{Cost, MAX_BYTES, MAX_DEPTH, MAX_NODES},
    },
    schedule::{self, Scope},
};
use incant_runtime::StableId;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};
const MAGIC: &[u8; 8] = b"INCSES01";
const HEADER: usize = 128;
const UI_HEADER: usize = 132;
const AUDIO_HEADER: usize = 136;
const MAX_AUDIO_BYTES: usize = 176 + 128 * 92 + 128 * (25 + 8 + 240);
// Same bound with or without the optional UI feature.
const MAX_UI_BYTES: usize =
    80 + 48 * incant_types::MAX_UI_NODES + incant_types::MAX_SCENE_TEXT_BYTES;
pub const MAX_CHECKPOINT_BYTES: usize =
    incant_runtime::MAX_SAVE_BYTES + 2 * 1024 * 1024 + MAX_UI_BYTES + MAX_AUDIO_BYTES;
const MAX_ACTIVE: usize = crate::MAX_SCRIPT_INSTANCES;

pub(super) struct Sections<'a> {
    pub origin: [u8; 32],
    pub world: &'a [u8],
    pub active: &'a [u8],
    pub store: &'a [u8],
    pub schedule: &'a [u8],
    pub ui: Option<&'a [u8]>,
    pub audio: Option<&'a [u8]>,
}
fn digest(bytes: &[u8], header: usize) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(&bytes[..header - 32]);
    h.update(&bytes[header..]);
    h.finalize().into()
}
pub(super) fn sections<'a>(bytes: &'a [u8], program: [u8; 32]) -> Result<Sections<'a>> {
    if !(HEADER..=MAX_CHECKPOINT_BYTES).contains(&bytes.len()) || &bytes[..8] != MAGIC {
        return Err(Error::Invalid("framing"));
    }
    let header = match u32::from_le_bytes(bytes[8..12].try_into().unwrap()) {
        1 => HEADER,
        2 if bytes.len() >= UI_HEADER => UI_HEADER,
        3 if bytes.len() >= AUDIO_HEADER => AUDIO_HEADER,
        _ => return Err(Error::Invalid("version")),
    };
    if bytes[12..16] != 1u32.to_le_bytes() {
        return Err(Error::Invalid("ordinary-load input policy"));
    }
    if bytes[16..48] != program {
        return Err(Error::ProgramMismatch);
    }
    if bytes[header - 32..header] != digest(bytes, header) {
        return Err(Error::Integrity);
    }
    let mut lens = [0usize; 6];
    let mut total = header;
    let count = if header == HEADER {
        4
    } else if header == UI_HEADER {
        5
    } else {
        6
    };
    for (i, n) in lens[..count].iter_mut().enumerate() {
        *n = u32::from_le_bytes(bytes[80 + i * 4..84 + i * 4].try_into().unwrap()) as usize;
        total = total
            .checked_add(*n)
            .ok_or(Error::Invalid("length overflow"))?;
    }
    if total != bytes.len()
        || lens[0] > incant_runtime::MAX_SAVE_BYTES
        || lens[1] > 4 + 16 * MAX_ACTIVE
        || lens[2] > MAX_BYTES + 4
        || lens[3] > 20 + 128 * 97
        || lens[4] > MAX_UI_BYTES
        || (count == 5 && lens[4] == 0)
        || lens[5] > MAX_AUDIO_BYTES
        || (count == 6 && lens[5] == 0)
    {
        return Err(Error::Invalid("section lengths"));
    }
    let (world, rest) = bytes[header..].split_at(lens[0]);
    let (active, rest) = rest.split_at(lens[1]);
    let (store, rest) = rest.split_at(lens[2]);
    let (schedule, tail) = rest.split_at(lens[3]);
    let (ui, audio) = tail.split_at(lens[4]);
    Ok(Sections {
        origin: bytes[48..80].try_into().unwrap(),
        world,
        active,
        store,
        schedule,
        ui: (lens[4] != 0).then_some(ui),
        audio: (count == 6).then_some(audio),
    })
}
#[allow(clippy::too_many_arguments)] // Explicit versioned section inputs.
pub(super) fn encode(
    program: [u8; 32],
    origin: [u8; 32],
    world: &[u8],
    active: &[StableId],
    store: &Store,
    schedule: &schedule::Schedule,
    ui: Option<&[u8]>,
    audio: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut a = Vec::new();
    u32w(&mut a, active.len());
    for id in active {
        a.extend(id.0);
    }
    let mut s = Vec::new();
    u32w(&mut s, store.entries.len());
    for (k, v) in &store.entries {
        scope_w(&mut s, k.scope);
        optional_id_w(&mut s, k.entity);
        value_w(&mut s, &v.value);
    }
    let mut t = Vec::new();
    t.extend(schedule.tick.to_le_bytes());
    t.extend(schedule.elapsed_seconds.to_bits().to_le_bytes());
    u32w(&mut t, schedule.timers.len());
    for (k, v) in &schedule.timers {
        scope_w(&mut t, k.scope);
        text_w(&mut t, &k.id);
        t.extend(v.due.to_le_bytes());
        t.extend(v.interval.unwrap_or(0).to_le_bytes());
    }
    let header = if audio.is_some() {
        AUDIO_HEADER
    } else if ui.is_some() {
        UI_HEADER
    } else {
        HEADER
    };
    if audio.is_some_and(|b| b.is_empty() || b.len() > MAX_AUDIO_BYTES) {
        return Err(Error::Invalid("audio capacity"));
    }
    if ui.is_some_and(|bytes| bytes.is_empty() || bytes.len() > MAX_UI_BYTES) {
        return Err(Error::Invalid("UI capacity"));
    }
    let total = header
        + world.len()
        + a.len()
        + s.len()
        + t.len()
        + ui.map_or(0, <[u8]>::len)
        + audio.map_or(0, <[u8]>::len);
    if total > MAX_CHECKPOINT_BYTES {
        return Err(Error::Invalid("checkpoint capacity"));
    }
    let mut out = Vec::with_capacity(total);
    out.extend(MAGIC);
    out.extend(
        if audio.is_some() {
            3u32
        } else if ui.is_some() {
            2u32
        } else {
            1u32
        }
        .to_le_bytes(),
    );
    out.extend(1u32.to_le_bytes());
    out.extend(program);
    out.extend(origin);
    for n in [world.len(), a.len(), s.len(), t.len()] {
        u32w(&mut out, n);
    }
    if header >= UI_HEADER {
        u32w(&mut out, ui.map_or(0, <[u8]>::len));
    }
    if let Some(audio) = audio {
        u32w(&mut out, audio.len());
    }
    out.extend([0; 32]);
    out.extend(world);
    out.extend(a);
    out.extend(s);
    out.extend(t);
    if let Some(ui) = ui {
        out.extend(ui);
    }
    if let Some(audio) = audio {
        out.extend(audio);
    }
    let h = digest(&out, header);
    out[header - 32..header].copy_from_slice(&h);
    Ok(out)
}
fn u32w(out: &mut Vec<u8>, n: usize) {
    out.extend(
        u32::try_from(n)
            .expect("bounded checkpoint field")
            .to_le_bytes(),
    );
}
fn text_w(out: &mut Vec<u8>, s: &str) {
    u32w(out, s.len());
    out.extend(s.as_bytes());
}
fn scope_w(out: &mut Vec<u8>, scope: Scope) {
    match scope {
        Scope::Global => out.push(0),
        Scope::Source(id) => {
            out.push(1);
            out.extend(id.0);
        }
    }
}
fn optional_id_w(out: &mut Vec<u8>, id: Option<StableId>) {
    match id {
        None => out.push(0),
        Some(id) => {
            out.push(1);
            out.extend(id.0);
        }
    }
}
fn value_w(out: &mut Vec<u8>, v: &ScriptValue) {
    match v {
        ScriptValue::Null => out.push(0),
        ScriptValue::Bool(false) => out.push(1),
        ScriptValue::Bool(true) => out.push(2),
        ScriptValue::Number(n) => {
            out.push(3);
            out.extend(n.to_bits().to_le_bytes());
        }
        ScriptValue::String(s) => {
            out.push(4);
            text_w(out, s);
        }
        ScriptValue::Array(a) => {
            out.push(5);
            u32w(out, a.len());
            for v in a {
                value_w(out, v);
            }
        }
        ScriptValue::Object(o) => {
            out.push(6);
            u32w(out, o.len());
            for (k, v) in o {
                text_w(out, k);
                value_w(out, v);
            }
        }
    }
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.0.len() {
            return Err(Error::Invalid("truncation"));
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn id(&mut self) -> Result<StableId> {
        Ok(StableId(self.take(16)?.try_into().unwrap()))
    }
    fn scope(&mut self) -> Result<Scope> {
        match self.byte()? {
            0 => Ok(Scope::Global),
            1 => Ok(Scope::Source(self.id()?)),
            _ => Err(Error::Invalid("scope tag")),
        }
    }
    fn optional_id(&mut self) -> Result<Option<StableId>> {
        match self.byte()? {
            0 => Ok(None),
            1 => Ok(Some(self.id()?)),
            _ => Err(Error::Invalid("instance tag")),
        }
    }
    fn count(&mut self, max: usize, min_row: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        if n > max || n > self.0.len() / min_row {
            return Err(Error::Invalid("count"));
        }
        Ok(n)
    }
    fn text(&mut self, max: usize) -> Result<&'a str> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(Error::Invalid("text capacity"));
        }
        std::str::from_utf8(self.take(n)?).map_err(|_| Error::Invalid("UTF-8"))
    }
    fn done(self) -> Result<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(Error::Invalid("trailing bytes"))
        }
    }
    fn value(&mut self, cost: &mut Cost, depth: usize) -> Result<ScriptValue> {
        if depth > MAX_DEPTH {
            return Err(Error::Invalid("value depth"));
        }
        cost.add(1, 1)?;
        Ok(match self.byte()? {
            0 => ScriptValue::Null,
            1 => ScriptValue::Bool(false),
            2 => ScriptValue::Bool(true),
            3 => {
                cost.add(8, 0)?;
                let n = f64::from_bits(self.u64()?);
                if !n.is_finite() {
                    return Err(Error::Invalid("finite value required"));
                }
                ScriptValue::Number(n)
            }
            4 => {
                let s = self.text(MAX_BYTES - cost.bytes)?;
                cost.add(4 + s.len(), 0)?;
                ScriptValue::String(s.into())
            }
            5 => {
                cost.add(4, 0)?;
                let n = self.count(MAX_NODES - cost.nodes, 1)?;
                let mut a = Vec::new();
                for _ in 0..n {
                    a.push(self.value(cost, depth + 1)?);
                }
                ScriptValue::Array(a)
            }
            6 => {
                cost.add(4, 0)?;
                let n = self.count(MAX_NODES - cost.nodes, 5)?;
                let mut o = BTreeMap::<String, ScriptValue>::new();
                for _ in 0..n {
                    let k = self.text(MAX_BYTES - cost.bytes)?;
                    cost.add(4 + k.len(), 0)?;
                    if o.last_key_value().is_some_and(|(old, _)| old.as_str() >= k) {
                        return Err(Error::Invalid("object key order"));
                    }
                    let v = self.value(cost, depth + 1)?;
                    o.insert(k.to_owned(), v);
                }
                ScriptValue::Object(o)
            }
            _ => return Err(Error::Invalid("value tag")),
        })
    }
}
pub(super) fn active(bytes: &[u8]) -> Result<Vec<StableId>> {
    let mut r = Reader(bytes);
    let n = r.count(MAX_ACTIVE, 16)?;
    let mut ids = Vec::new();
    for _ in 0..n {
        let id = r.id()?;
        if ids.last().is_some_and(|old| *old >= id) {
            return Err(Error::Invalid("attachment order"));
        }
        ids.push(id);
    }
    r.done()?;
    Ok(ids)
}
pub(super) fn store(bytes: &[u8]) -> Result<Store> {
    let mut r = Reader(bytes);
    let n = r.count(MAX_NODES, 3)?;
    let mut store = Store::default();
    for _ in 0..n {
        let key = persistence::Key {
            scope: r.scope()?,
            entity: r.optional_id()?,
        };
        if store
            .entries
            .last_key_value()
            .is_some_and(|(old, _)| *old >= key)
        {
            return Err(Error::Invalid("state key order"));
        }
        let before = store.cost;
        store.cost.add(34, 0)?;
        let value = r.value(&mut store.cost, 0)?;
        let cost = Cost {
            bytes: store.cost.bytes - before.bytes,
            nodes: store.cost.nodes - before.nodes,
        };
        store.entries.insert(key, Arc::new(Entry { value, cost }));
    }
    r.done()?;
    Ok(store)
}
pub(super) fn schedule(bytes: &[u8]) -> Result<schedule::Schedule> {
    let mut r = Reader(bytes);
    let tick = r.u64()?;
    let elapsed_seconds = f64::from_bits(r.u64()?);
    if tick > schedule::MAX_TICK
        || !elapsed_seconds.is_finite()
        || elapsed_seconds < 0.
        || elapsed_seconds > tick as f64
        || (tick == 0) != (elapsed_seconds == 0.)
    {
        return Err(Error::Invalid("schedule clock"));
    }
    let n = r.count(schedule::MAX_TIMERS, 18)?;
    let mut timers = BTreeMap::new();
    for _ in 0..n {
        let scope = r.scope()?;
        let id = r.text(64)?;
        if id.is_empty()
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
        {
            return Err(Error::Invalid("timer ID"));
        }
        let key = schedule::Key {
            scope,
            id: id.to_owned(),
        };
        if timers.last_key_value().is_some_and(|(old, _)| *old >= key) {
            return Err(Error::Invalid("timer order"));
        }
        let due = r.u64()?;
        let interval = r.u32()?;
        if due <= tick || due > schedule::MAX_TICK || due - tick > u64::from(u32::MAX) {
            return Err(Error::Invalid("timer deadline"));
        }
        timers.insert(
            key,
            schedule::Timer {
                due,
                interval: (interval != 0).then_some(interval),
            },
        );
    }
    r.done()?;
    Ok(schedule::Schedule {
        tick,
        elapsed_seconds,
        timers,
    })
}
