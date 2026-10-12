//! Canonical copied desired audio declarations and bounded transport commands.
//! These records never contain a mixer handle, playback cursor or device state.
use crate::{
    AUDIO_COMMAND_MAGIC, AUDIO_MAGIC, AUDIO_MAX_BUSES, AUDIO_MAX_COMMANDS, AUDIO_MAX_NODES,
    AUDIO_MAX_SOURCES, AudioCommand, AudioCommandHeader, AudioControlKind, AudioHeader, AudioRow,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Malformed;
type Result<T> = core::result::Result<T, Malformed>;

#[derive(Clone, Copy)]
pub struct Snapshot<'a> {
    pub scene: [u8; 16],
    pub tick: u64,
    rows: &'a [u8],
}
impl<'a> Snapshot<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let h = AudioHeader::decode(bytes).ok_or(Malformed)?;
        if &h.magic != AUDIO_MAGIC
            || h.count as usize > AUDIO_MAX_NODES
            || h.stride as usize != AudioRow::SIZE
            || bytes.len() != AudioHeader::SIZE + h.count as usize * AudioRow::SIZE
        {
            return Err(Malformed);
        }
        let result = Self {
            scene: h.scene,
            tick: h.tick,
            rows: &bytes[AudioHeader::SIZE..],
        };
        let (mut buses, mut sources, mut listeners, mut spatial) = (0, 0, 0, false);
        let mut previous = None;
        for row in result.iter() {
            validate_row(&row)?;
            if previous.is_some_and(|id| id >= row.entity) {
                return Err(Malformed);
            }
            previous = Some(row.entity);
            buses += usize::from(row.roles & 1 != 0);
            sources += usize::from(row.roles & 2 != 0);
            listeners += usize::from(row.roles & 4 != 0);
            spatial |= row.source_flags & 8 != 0;
        }
        if buses > AUDIO_MAX_BUSES
            || sources > AUDIO_MAX_SOURCES
            || listeners > 1
            || (spatial && listeners != 1)
        {
            return Err(Malformed);
        }
        for row in result.iter() {
            if row.source_bus_present != 0 {
                result.bus(row.source_bus)?;
            }
            // A valid parent chain has at most the bounded number of buses.
            // This detects cycles without allocation or trusting row order.
            let mut node = row;
            let mut hops = 0;
            while node.bus_parent_present != 0 {
                node = result.bus(node.bus_parent)?;
                hops += 1;
                if hops >= buses {
                    return Err(Malformed);
                }
            }
        }
        Ok(result)
    }
    fn bus(&self, id: [u8; 16]) -> Result<AudioRow> {
        self.find(id).filter(|r| r.roles == 1).ok_or(Malformed)
    }
    pub fn len(&self) -> usize {
        self.rows.len() / AudioRow::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = AudioRow> + '_ {
        self.rows
            .as_chunks::<{ AudioRow::SIZE }>()
            .0
            .iter()
            .map(|r| AudioRow::decode(r).unwrap())
    }
    pub fn find(&self, id: [u8; 16]) -> Option<AudioRow> {
        let (mut lo, mut hi) = (0, self.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let row = AudioRow::decode(&self.rows[mid * AudioRow::SIZE..]).unwrap();
            match row.entity.cmp(&id) {
                core::cmp::Ordering::Less => lo = mid + 1,
                core::cmp::Ordering::Greater => hi = mid,
                core::cmp::Ordering::Equal => return Some(row),
            }
        }
        None
    }
}
fn zero(values: &[f64]) -> bool {
    values.iter().all(|v| v.to_bits() == 0)
}
fn native_f32(value: f64, min: f32, max: f32) -> bool {
    (f64::from(min)..=f64::from(max)).contains(&value) && f64::from(value as f32) == value
}
fn validate_row(r: &AudioRow) -> Result<()> {
    if !r.is_finite()
        || !matches!(r.roles, 1 | 2 | 4 | 6)
        || r.bus_parent_present > 1
        || r.source_bus_present > 1
        || r.source_flags & !15 != 0
        || (r.bus_parent_present == 0 && r.bus_parent != [0; 16])
        || (r.source_bus_present == 0 && r.source_bus != [0; 16])
    {
        return Err(Malformed);
    }
    if r.roles & 1 != 0 {
        if !native_f32(r.bus_gain, -80., 24.) {
            return Err(Malformed);
        }
    } else if r.bus_parent_present != 0 || !zero(&[r.bus_gain]) {
        return Err(Malformed);
    }
    if r.roles & 2 != 0 {
        if !native_f32(r.gain, -80., 24.)
            || !native_f32(r.pan, -1., 1.)
            || !(0.25..=4.).contains(&r.rate)
            || !(0. ..=3600.).contains(&r.start_seconds)
        {
            return Err(Malformed);
        }
    } else if r.clip != [0; 16]
        || r.source_bus_present != 0
        || r.source_flags != 0
        || r.transport_epoch != 0
        || !zero(&[r.gain, r.pan, r.rate, r.start_seconds])
    {
        return Err(Malformed);
    }
    if r.source_flags & 8 != 0 {
        if !native_f32(r.min_distance, 0.01, 1_000_000.)
            || !native_f32(r.max_distance, 0.01, 1_000_000.)
            || r.min_distance >= r.max_distance
            || r.pan != 0.
        {
            return Err(Malformed);
        }
    } else if !zero(&[r.min_distance, r.max_distance]) {
        return Err(Malformed);
    }
    if r.roles & 4 != 0 {
        let norm = r.listener_rotation.iter().map(|v| v * v).sum::<f64>();
        if !norm.is_finite() || (norm - 1.).abs() > 1e-12 {
            return Err(Malformed);
        }
    } else if !zero(&r.listener_rotation) {
        return Err(Malformed);
    }
    if r.roles & 4 == 0 && r.source_flags & 8 == 0 && !zero(&r.world_translation) {
        return Err(Malformed);
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub struct Commands<'a> {
    rows: &'a [u8],
}
impl<'a> Commands<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let h = AudioCommandHeader::decode(bytes).ok_or(Malformed)?;
        if &h.magic != AUDIO_COMMAND_MAGIC
            || h.count as usize > AUDIO_MAX_COMMANDS
            || h.stride as usize != AudioCommand::SIZE
            || bytes.len() != AudioCommandHeader::SIZE + h.count as usize * AudioCommand::SIZE
        {
            return Err(Malformed);
        }
        let result = Self {
            rows: &bytes[AudioCommandHeader::SIZE..],
        };
        for row in result.iter() {
            validate_command(&row)?;
        }
        Ok(result)
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = AudioCommand> + '_ {
        self.rows
            .as_chunks::<{ AudioCommand::SIZE }>()
            .0
            .iter()
            .map(|r| AudioCommand::decode(r).unwrap())
    }
}
/// Validate the original f64 before any host-native f32 conversion.
pub fn validate_command(row: &AudioCommand) -> Result<AudioControlKind> {
    let kind = u8::try_from(row.kind)
        .ok()
        .and_then(AudioControlKind::from_tag)
        .ok_or(Malformed)?;
    if row.reserved != 0
        || !row.value.is_finite()
        || !match kind {
            AudioControlKind::SetPlaying => row.value.to_bits() == 0 || row.value == 1.,
            AudioControlKind::Stop | AudioControlKind::Restart => row.value.to_bits() == 0,
            AudioControlKind::Seek => (0. ..=3600.).contains(&row.value),
            AudioControlKind::SourceGain | AudioControlKind::BusGain => {
                (-80. ..=24.).contains(&row.value)
            }
        }
    {
        return Err(Malformed);
    }
    Ok(kind)
}
