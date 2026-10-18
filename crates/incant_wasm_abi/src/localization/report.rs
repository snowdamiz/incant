//! Bounded prepared missing-translation coverage, in canonical table/key order.
//! Result kind 3 is additive; older kind-specific decoders reject it explicitly.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub offset: u32,
    pub limit: u32,
}
impl Default for Request {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 128,
        }
    }
}
impl Request {
    pub fn validate(self) -> Result<()> {
        if self.offset <= LOCALIZATION_MAX_REPORT_TOTAL as u32
            && (1..=LOCALIZATION_MAX_REPORT_ROWS as u32).contains(&self.limit)
        {
            Ok(())
        } else {
            Err(Error::Bounds)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    pub table_id: &'a str,
    pub key: &'a str,
    pub requested_locale: &'a str,
    pub resolved_locale: Option<&'a str>,
    pub kind: MissingKind,
}

#[derive(Clone, Copy)]
pub struct Snapshot<'a> {
    pub total: u32,
    count: usize,
    bytes: &'a [u8],
}
impl<'a> Snapshot<'a> {
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn entries(&self) -> Entries<'a> {
        Entries {
            reader: Reader(self.bytes),
            left: self.count,
        }
    }
}
pub struct Entries<'a> {
    reader: Reader<'a>,
    left: usize,
}
impl<'a> Iterator for Entries<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        // The full immutable borrowed frame was validated before publication.
        read_entry(&mut self.reader).ok()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}
impl ExactSizeIterator for Entries<'_> {}

fn read_entry<'a>(reader: &mut Reader<'a>) -> Result<Entry<'a>> {
    let table_id = reader.id()?;
    let key = reader.text(LOCALIZATION_MAX_KEY_BYTES)?;
    check_key(key)?;
    let requested_locale = reader.text(LOCALIZATION_MAX_LOCALE_BYTES)?;
    let resolved_locale = match reader.u32()? {
        0 => None,
        1 => Some(reader.text(LOCALIZATION_MAX_LOCALE_BYTES)?),
        _ => return Err(Error::Malformed),
    };
    let kind = match reader.u32()? {
        1 => MissingKind::Translation,
        2 if resolved_locale.is_none() => MissingKind::Key,
        _ => return Err(Error::Malformed),
    };
    Ok(Entry {
        table_id,
        key,
        requested_locale,
        resolved_locale,
        kind,
    })
}

pub fn encode(total: u32, entries: &[Entry<'_>], bytes: &mut [u8]) -> Result<usize> {
    if total > LOCALIZATION_MAX_REPORT_TOTAL as u32
        || entries.len() > LOCALIZATION_MAX_REPORT_ROWS
        || entries.len() > total as usize
    {
        return Err(Error::Bounds);
    }
    let mut w = Writer::frame(bytes, LOCALIZATION_RESULT_MAGIC)?;
    w.u32(3)?;
    w.u32(total)?;
    w.count(entries.len())?;
    for entry in entries {
        check_id(entry.table_id)?;
        check_key(entry.key)?;
        w.put(entry.table_id.as_bytes())?;
        w.text(entry.key, LOCALIZATION_MAX_KEY_BYTES)?;
        w.text(entry.requested_locale, LOCALIZATION_MAX_LOCALE_BYTES)?;
        w.u32(u32::from(entry.resolved_locale.is_some()))?;
        if let Some(locale) = entry.resolved_locale {
            w.text(locale, LOCALIZATION_MAX_LOCALE_BYTES)?;
        }
        w.u32(match entry.kind {
            MissingKind::Translation => 1,
            MissingKind::Key => 2,
        })?;
    }
    let used = w.used;
    decode(&bytes[..used])?;
    Ok(used)
}

pub fn decode(bytes: &[u8]) -> Result<Snapshot<'_>> {
    let mut reader = Reader::frame(
        bytes,
        LOCALIZATION_RESULT_MAGIC,
        LOCALIZATION_MAX_RESULT_BYTES,
    )?;
    if reader.u32()? != 3 {
        return Err(Error::Malformed);
    }
    let total = reader.count(LOCALIZATION_MAX_REPORT_TOTAL)? as u32;
    let count = reader.count(LOCALIZATION_MAX_REPORT_ROWS)?;
    if count > total as usize {
        return Err(Error::Malformed);
    }
    let entries = reader.0;
    let mut previous = None;
    for _ in 0..count {
        let entry = read_entry(&mut reader)?;
        let key = (entry.table_id, entry.key);
        if previous.is_some_and(|prior| prior >= key) {
            return Err(Error::Malformed);
        }
        previous = Some(key);
    }
    reader.end()?;
    Ok(Snapshot {
        total,
        count,
        bytes: entries,
    })
}
