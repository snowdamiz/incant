//! Versioned copied localization frames. No allocation, JSON or native pointers.
use crate::*;
use core::str;
pub mod report;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Bounds,
    Malformed,
}
pub type Result<T> = core::result::Result<T, Error>;
const VERSION: u32 = 1;
const MAX_NUMBER: f64 = 9_007_199_254_740_991.;
pub use crate::DateLength;
/// Fixed-size copied formatting request. Calendar validity is checked by the
/// shared native formatter; the wire codec validates its primitive domain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormatRequest {
    Number(f64),
    Date(CalendarDate, DateLength),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}
fn check_number(value: f64) -> Result<()> {
    if value.is_finite() && value.abs() <= MAX_NUMBER {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}
fn check_date(date: CalendarDate) -> Result<()> {
    if (-9999..=9999).contains(&date.year)
        && (1..=12).contains(&date.month)
        && (1..=31).contains(&date.day)
    {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}
pub fn encode_format(request: FormatRequest, bytes: &mut [u8]) -> Result<usize> {
    match request {
        FormatRequest::Number(value) => check_number(value)?,
        FormatRequest::Date(date, _) => check_date(date)?,
    }
    let bytes = bytes
        .get_mut(..LOCALIZATION_FORMAT_BYTES)
        .ok_or(Error::Bounds)?;
    let mut w = Writer::frame(bytes, LOCALIZATION_FORMAT_MAGIC)?;
    match request {
        FormatRequest::Number(value) => {
            w.u32(0)?;
            w.put(&value.to_le_bytes())?;
        }
        FormatRequest::Date(date, length) => {
            w.u32(1)?;
            w.u32(date.year as u32)?;
            w.put(&[date.month, date.day, length as u8, 0])?;
        }
    }
    Ok(w.used)
}
pub fn decode_format(bytes: &[u8]) -> Result<FormatRequest> {
    let mut r = Reader::frame(bytes, LOCALIZATION_FORMAT_MAGIC, LOCALIZATION_FORMAT_BYTES)?;
    let request = match r.u32()? {
        0 => {
            let value = f64::from_le_bytes(r.take(8)?.try_into().unwrap());
            check_number(value)?;
            FormatRequest::Number(value)
        }
        1 => {
            let year = r.u32()? as i32;
            let fields = r.take(4)?;
            let date = CalendarDate {
                year,
                month: fields[0],
                day: fields[1],
            };
            check_date(date)?;
            if fields[3] != 0 {
                return Err(Error::Malformed);
            }
            let length = DateLength::from_tag(fields[2]).ok_or(Error::Malformed)?;
            FormatRequest::Date(date, length)
        }
        _ => return Err(Error::Malformed),
    };
    r.end()?;
    Ok(request)
}
/// Formatted primitive text uses result kind 2. Older kind-specific readers
/// explicitly reject it rather than reinterpret it as locale/message metadata.
pub fn encode_formatted(text: &str, bytes: &mut [u8]) -> Result<usize> {
    let mut w = Writer::frame(bytes, LOCALIZATION_RESULT_MAGIC)?;
    w.u32(2)?;
    w.text(text, LOCALIZATION_MAX_TEXT_BYTES)?;
    Ok(w.used)
}
pub fn decode_formatted(bytes: &[u8]) -> Result<&str> {
    let mut r = Reader::frame(
        bytes,
        LOCALIZATION_RESULT_MAGIC,
        LOCALIZATION_MAX_RESULT_BYTES,
    )?;
    if r.u32()? != 2 {
        return Err(Error::Malformed);
    }
    let text = r.text(LOCALIZATION_MAX_TEXT_BYTES)?;
    r.end()?;
    Ok(text)
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MessageArgument<'a> {
    Text(&'a str),
    Number(f64),
    Date(CalendarDate),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Argument<'a> {
    pub name: &'a str,
    pub value: MessageArgument<'a>,
}
#[derive(Clone, Copy, Debug)]
pub struct LocalizeRequest<'a> {
    pub table_id: &'a str,
    pub key: &'a str,
    pub arguments: &'a [Argument<'a>],
}
#[derive(Clone, Copy, Debug)]
pub struct LocaleSettings<'a> {
    pub locale: &'a str,
    pub fallbacks: &'a [&'a str],
    pub pseudo: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingKind {
    Translation,
    Key,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissingString<'a> {
    pub table_id: &'a str,
    pub key: &'a str,
    pub kind: MissingKind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalizedText<'a> {
    pub text: &'a str,
    pub requested_locale: &'a str,
    pub resolved_locale: Option<&'a str>,
    pub missing: Option<MissingString<'a>>,
}
#[derive(Clone, Copy, Debug)]
pub struct LocaleSnapshot<'a> {
    pub locale: &'a str,
    pub pseudo: bool,
    fallbacks: &'a [u8],
    count: usize,
}
impl<'a> LocaleSnapshot<'a> {
    pub fn fallbacks(&self) -> Fallbacks<'a> {
        Fallbacks {
            reader: Reader(self.fallbacks),
            left: self.count,
        }
    }
}
pub struct Fallbacks<'a> {
    reader: Reader<'a>,
    left: usize,
}
impl<'a> Iterator for Fallbacks<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        self.reader.text(LOCALIZATION_MAX_LOCALE_BYTES).ok()
    }
}
#[derive(Clone, Copy)]
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let bytes = self.0.get(..len).ok_or(Error::Bounds)?;
        self.0 = &self.0[len..];
        Ok(bytes)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn count(&mut self, max: usize) -> Result<usize> {
        usize::try_from(self.u32()?)
            .ok()
            .filter(|n| *n <= max)
            .ok_or(Error::Bounds)
    }
    fn text(&mut self, max: usize) -> Result<&'a str> {
        let len = self.count(max)?;
        str::from_utf8(self.take(len)?).map_err(|_| Error::Malformed)
    }
    fn frame(bytes: &'a [u8], magic: &[u8; 8], max: usize) -> Result<Self> {
        if bytes.len() > max {
            return Err(Error::Bounds);
        }
        let mut r = Self(bytes);
        if r.take(8)? != magic || r.u32()? != VERSION {
            return Err(Error::Malformed);
        }
        Ok(r)
    }
    fn end(self) -> Result<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(Error::Malformed)
        }
    }
    fn id(&mut self) -> Result<&'a str> {
        let id = str::from_utf8(self.take(26)?).map_err(|_| Error::Malformed)?;
        check_id(id)?;
        Ok(id)
    }
}
struct Writer<'a> {
    bytes: &'a mut [u8],
    used: usize,
}
impl<'a> Writer<'a> {
    fn put(&mut self, bytes: &[u8]) -> Result<()> {
        let end = self.used.checked_add(bytes.len()).ok_or(Error::Bounds)?;
        self.bytes
            .get_mut(self.used..end)
            .ok_or(Error::Bounds)?
            .copy_from_slice(bytes);
        self.used = end;
        Ok(())
    }
    fn u32(&mut self, value: u32) -> Result<()> {
        self.put(&value.to_le_bytes())
    }
    fn count(&mut self, value: usize) -> Result<()> {
        self.u32(u32::try_from(value).map_err(|_| Error::Bounds)?)
    }
    fn text(&mut self, value: &str, max: usize) -> Result<()> {
        if value.len() > max {
            return Err(Error::Bounds);
        }
        self.count(value.len())?;
        self.put(value.as_bytes())
    }
    fn frame(bytes: &'a mut [u8], magic: &[u8; 8]) -> Result<Self> {
        let mut w = Self { bytes, used: 0 };
        w.put(magic)?;
        w.u32(VERSION)?;
        Ok(w)
    }
}
fn check_id(id: &str) -> Result<()> {
    if id.len() == 26
        && id.as_bytes()[0] <= b'7'
        && id
            .bytes()
            .all(|b| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&b))
    {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}
fn check_key(key: &str) -> Result<()> {
    if !key.is_empty()
        && key.len() <= LOCALIZATION_MAX_KEY_BYTES
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
    {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}
fn argument<'a>(r: &mut Reader<'a>) -> Result<Argument<'a>> {
    let name = r.text(LOCALIZATION_MAX_KEY_BYTES)?;
    check_key(name)?;
    let tag = r.u32()?;
    let value = match tag {
        0 => MessageArgument::Text(r.text(LOCALIZATION_MAX_TEXT_BYTES)?),
        1 => {
            let value = f64::from_le_bytes(r.take(8)?.try_into().unwrap());
            if !value.is_finite() || value.abs() > MAX_NUMBER {
                return Err(Error::Malformed);
            }
            MessageArgument::Number(value)
        }
        2 => {
            let year = r.u32()? as i32;
            let date = r.take(4)?;
            if !(-9999..=9999).contains(&year)
                || !(1..=12).contains(&date[0])
                || !(1..=31).contains(&date[1])
                || date[2..] != [0, 0]
            {
                return Err(Error::Malformed);
            }
            MessageArgument::Date(CalendarDate {
                year,
                month: date[0],
                day: date[1],
            })
        }
        _ => return Err(Error::Malformed),
    };
    Ok(Argument { name, value })
}
#[derive(Clone, Copy)]
pub struct RequestSnapshot<'a> {
    pub table_id: &'a str,
    pub key: &'a str,
    arguments: &'a [u8],
    count: usize,
    pub input_bytes: usize,
}
impl<'a> RequestSnapshot<'a> {
    pub fn arguments(&self) -> Arguments<'a> {
        Arguments {
            reader: Reader(self.arguments),
            left: self.count,
        }
    }
}
pub struct Arguments<'a> {
    reader: Reader<'a>,
    left: usize,
}
impl<'a> Iterator for Arguments<'a> {
    type Item = Argument<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        argument(&mut self.reader).ok()
    }
}
pub fn decode_request(bytes: &[u8]) -> Result<RequestSnapshot<'_>> {
    let mut r = Reader::frame(
        bytes,
        LOCALIZATION_REQUEST_MAGIC,
        LOCALIZATION_MAX_REQUEST_BYTES,
    )?;
    let table_id = r.id()?;
    let key = r.text(LOCALIZATION_MAX_KEY_BYTES)?;
    check_key(key)?;
    let count = r.count(LOCALIZATION_MAX_ARGUMENTS)?;
    let arguments = r.0;
    let mut input_bytes = table_id.len() + key.len();
    let mut names = [""; LOCALIZATION_MAX_ARGUMENTS];
    for name in names.iter_mut().take(count) {
        *name = argument(&mut r)?.name;
    }
    r.end()?;
    let result = RequestSnapshot {
        table_id,
        key,
        arguments,
        count,
        input_bytes: 0,
    };
    for (i, a) in result.arguments().enumerate() {
        if names[..i].contains(&a.name) {
            return Err(Error::Malformed);
        }
        input_bytes += a.name.len()
            + match a.value {
                MessageArgument::Text(text) => text.len(),
                _ => 16,
            };
    }
    if input_bytes > LOCALIZATION_MAX_INPUT_BYTES {
        return Err(Error::Bounds);
    }
    Ok(RequestSnapshot {
        input_bytes,
        ..result
    })
}
pub fn encode_request(request: LocalizeRequest<'_>, bytes: &mut [u8]) -> Result<usize> {
    check_id(request.table_id)?;
    check_key(request.key)?;
    if request.arguments.len() > LOCALIZATION_MAX_ARGUMENTS {
        return Err(Error::Bounds);
    }
    let mut w = Writer::frame(bytes, LOCALIZATION_REQUEST_MAGIC)?;
    w.put(request.table_id.as_bytes())?;
    w.text(request.key, LOCALIZATION_MAX_KEY_BYTES)?;
    w.count(request.arguments.len())?;
    for a in request.arguments {
        w.text(a.name, LOCALIZATION_MAX_KEY_BYTES)?;
        match a.value {
            MessageArgument::Text(text) => {
                w.u32(0)?;
                w.text(text, LOCALIZATION_MAX_TEXT_BYTES)?;
            }
            MessageArgument::Number(number) => {
                w.u32(1)?;
                w.put(&number.to_le_bytes())?;
            }
            MessageArgument::Date(date) => {
                w.u32(2)?;
                w.u32(date.year as u32)?;
                w.put(&[date.month, date.day, 0, 0])?;
            }
        }
    }
    let used = w.used;
    decode_request(&bytes[..used])?;
    Ok(used)
}
fn write_settings(w: &mut Writer<'_>, settings: LocaleSettings<'_>) -> Result<()> {
    if settings.fallbacks.len() > LOCALIZATION_MAX_FALLBACKS {
        return Err(Error::Bounds);
    }
    w.text(settings.locale, LOCALIZATION_MAX_LOCALE_BYTES)?;
    w.count(settings.fallbacks.len())?;
    for fallback in settings.fallbacks {
        w.text(fallback, LOCALIZATION_MAX_LOCALE_BYTES)?;
    }
    w.u32(u32::from(settings.pseudo))
}
fn read_settings<'a>(r: &mut Reader<'a>) -> Result<LocaleSnapshot<'a>> {
    let locale = r.text(LOCALIZATION_MAX_LOCALE_BYTES)?;
    let count = r.count(LOCALIZATION_MAX_FALLBACKS)?;
    let fallbacks = r.0;
    let mut names = [""; LOCALIZATION_MAX_FALLBACKS];
    for i in 0..count {
        let name = r.text(LOCALIZATION_MAX_LOCALE_BYTES)?;
        if names[..i].contains(&name) {
            return Err(Error::Malformed);
        }
        names[i] = name;
    }
    let fallbacks = &fallbacks[..fallbacks.len() - r.0.len()];
    let pseudo = match r.u32()? {
        0 => false,
        1 => true,
        _ => return Err(Error::Malformed),
    };
    Ok(LocaleSnapshot {
        locale,
        pseudo,
        fallbacks,
        count,
    })
}
pub fn encode_settings(settings: LocaleSettings<'_>, bytes: &mut [u8]) -> Result<usize> {
    let mut w = Writer::frame(bytes, LOCALIZATION_SETTINGS_MAGIC)?;
    write_settings(&mut w, settings)?;
    let used = w.used;
    decode_settings(&bytes[..used])?;
    Ok(used)
}
pub fn decode_settings(bytes: &[u8]) -> Result<LocaleSnapshot<'_>> {
    let mut r = Reader::frame(
        bytes,
        LOCALIZATION_SETTINGS_MAGIC,
        LOCALIZATION_MAX_SETTINGS_BYTES,
    )?;
    let result = read_settings(&mut r)?;
    r.end()?;
    Ok(result)
}
pub fn encode_locale(settings: LocaleSettings<'_>, bytes: &mut [u8]) -> Result<usize> {
    let mut w = Writer::frame(bytes, LOCALIZATION_RESULT_MAGIC)?;
    w.u32(0)?;
    write_settings(&mut w, settings)?;
    Ok(w.used)
}
pub fn decode_locale(bytes: &[u8]) -> Result<LocaleSnapshot<'_>> {
    let mut r = Reader::frame(
        bytes,
        LOCALIZATION_RESULT_MAGIC,
        LOCALIZATION_MAX_RESULT_BYTES,
    )?;
    if r.u32()? != 0 {
        return Err(Error::Malformed);
    }
    let result = read_settings(&mut r)?;
    r.end()?;
    Ok(result)
}
pub fn encode_localized(value: LocalizedText<'_>, bytes: &mut [u8]) -> Result<usize> {
    let mut w = Writer::frame(bytes, LOCALIZATION_RESULT_MAGIC)?;
    w.u32(1)?;
    w.text(value.text, LOCALIZATION_MAX_TEXT_BYTES)?;
    w.text(value.requested_locale, LOCALIZATION_MAX_LOCALE_BYTES)?;
    w.u32(u32::from(value.resolved_locale.is_some()))?;
    if let Some(locale) = value.resolved_locale {
        w.text(locale, LOCALIZATION_MAX_LOCALE_BYTES)?;
    }
    w.u32(match value.missing {
        None => 0,
        Some(MissingString {
            kind: MissingKind::Translation,
            ..
        }) => 1,
        Some(_) => 2,
    })?;
    if let Some(missing) = value.missing {
        check_id(missing.table_id)?;
        check_key(missing.key)?;
        w.put(missing.table_id.as_bytes())?;
        w.text(missing.key, LOCALIZATION_MAX_KEY_BYTES)?;
    }
    Ok(w.used)
}
pub fn decode_localized(bytes: &[u8]) -> Result<LocalizedText<'_>> {
    let mut r = Reader::frame(
        bytes,
        LOCALIZATION_RESULT_MAGIC,
        LOCALIZATION_MAX_RESULT_BYTES,
    )?;
    if r.u32()? != 1 {
        return Err(Error::Malformed);
    }
    let text = r.text(LOCALIZATION_MAX_TEXT_BYTES)?;
    let requested_locale = r.text(LOCALIZATION_MAX_LOCALE_BYTES)?;
    let resolved_locale = match r.u32()? {
        0 => None,
        1 => Some(r.text(LOCALIZATION_MAX_LOCALE_BYTES)?),
        _ => return Err(Error::Malformed),
    };
    let missing = match r.u32()? {
        0 => None,
        tag @ (1 | 2) => {
            let table_id = r.id()?;
            let key = r.text(LOCALIZATION_MAX_KEY_BYTES)?;
            check_key(key)?;
            Some(MissingString {
                table_id,
                key,
                kind: if tag == 1 {
                    MissingKind::Translation
                } else {
                    MissingKind::Key
                },
            })
        }
        _ => return Err(Error::Malformed),
    };
    r.end()?;
    if matches!(
        missing,
        Some(MissingString {
            kind: MissingKind::Key,
            ..
        })
    ) && resolved_locale.is_some()
        || matches!(
            missing,
            Some(MissingString {
                kind: MissingKind::Translation,
                ..
            })
        ) && resolved_locale.is_none()
    {
        return Err(Error::Malformed);
    }
    Ok(LocalizedText {
        text,
        requested_locale,
        resolved_locale,
        missing,
    })
}
