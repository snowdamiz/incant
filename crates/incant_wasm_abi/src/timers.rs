//! Fixed-tick copied timer records. IDs and all unused fields are canonical.
use crate::{TIMER_ID_BYTES, TIMER_MAGIC, TIMER_MAX_TIMERS, TimerCommand, TimerEvent, TimerHeader};

pub const MAX_TICK: u64 = (1 << 53) - 1;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Malformed;
type Result<T> = core::result::Result<T, Malformed>;

pub fn id(bytes: &[u8; TIMER_ID_BYTES], len: u32) -> Result<&str> {
    let len = usize::try_from(len).map_err(|_| Malformed)?;
    if !(1..=TIMER_ID_BYTES).contains(&len)
        || bytes[len..].iter().any(|b| *b != 0)
        || !bytes[..len]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(b))
    {
        return Err(Malformed);
    }
    core::str::from_utf8(&bytes[..len]).map_err(|_| Malformed)
}

pub fn encode_id(value: &str) -> Result<([u8; TIMER_ID_BYTES], u32)> {
    if !(1..=TIMER_ID_BYTES).contains(&value.len()) {
        return Err(Malformed);
    }
    let mut bytes = [0; TIMER_ID_BYTES];
    bytes[..value.len()].copy_from_slice(value.as_bytes());
    id(&bytes, value.len() as u32)?;
    Ok((bytes, value.len() as u32))
}

pub fn validate_command(command: &TimerCommand) -> Result<()> {
    id(&command.id, command.id_len)?;
    match command.kind {
        0 if command.delay > 0 => Ok(()),
        1 if command.delay == 0 && command.interval == 0 => Ok(()),
        _ => Err(Malformed),
    }
}

/// Events are a stable, nondestructive view of this callback's due timers.
#[derive(Clone, Copy)]
pub struct Snapshot<'a> {
    header: TimerHeader,
    rows: &'a [u8],
}
impl<'a> Snapshot<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let header = TimerHeader::decode(bytes).ok_or(Malformed)?;
        if &header.magic != TIMER_MAGIC
            || header.tick == 0
            || header.tick > MAX_TICK
            || !header.elapsed_seconds.is_finite()
            || header.elapsed_seconds <= 0.
            || header.count as usize > TIMER_MAX_TIMERS
            || header.stride as usize != TimerEvent::SIZE
            || bytes.len() != TimerHeader::SIZE + header.count as usize * TimerEvent::SIZE
        {
            return Err(Malformed);
        }
        let snapshot = Self {
            header,
            rows: &bytes[TimerHeader::SIZE..],
        };
        let mut previous = None;
        for row in snapshot.rows.as_chunks::<{ TimerEvent::SIZE }>().0 {
            let event = TimerEvent::decode(row).ok_or(Malformed)?;
            id(&event.id, event.id_len)?;
            if event.reserved != 0
                || event.scheduled_tick != header.tick
                || previous.is_some_and(|old| old >= event.id)
            {
                return Err(Malformed);
            }
            previous = Some(event.id);
        }
        Ok(snapshot)
    }
    pub fn tick(&self) -> u64 {
        self.header.tick
    }
    pub fn elapsed_seconds(&self) -> f64 {
        self.header.elapsed_seconds
    }
    pub fn len(&self) -> usize {
        self.rows.len() / TimerEvent::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn events(&self) -> impl ExactSizeIterator<Item = (&'a str, u64)> + 'a {
        self.rows
            .as_chunks::<{ TimerEvent::SIZE }>()
            .0
            .iter()
            .map(|row| {
                let event = TimerEvent::decode(row).unwrap();
                // Validated ASCII and length; borrow only the original scratch.
                (
                    core::str::from_utf8(&row[..event.id_len as usize]).unwrap(),
                    event.scheduled_tick,
                )
            })
    }
}
